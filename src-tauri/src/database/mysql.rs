use super::{
    Capabilities, Cell, ColumnMeta, ConnectParams, DatabaseSession, DbObject, Engine, ObjectKind,
    ObjectStructure, QueryResult, ServerInfo, TextRows, TlsMode, error, first_column, metadata,
    quote, values,
};
use crate::errors::AppError;
use futures_util::TryStreamExt;
use sqlx::{
    AssertSqlSafe, Column, ConnectOptions, Connection, Either, Row, TypeInfo,
    mysql::{MySqlConnectOptions, MySqlConnection, MySqlSslMode},
};

pub struct MySqlSession {
    connection: MySqlConnection,
    info: ServerInfo,
    /// Connection id, used to cancel a running query with KILL QUERY.
    connection_id: u64,
    host: String,
    port: u16,
}

impl MySqlSession {
    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }
}

fn context(params: &ConnectParams) -> error::Context<'_> {
    error::Context {
        engine: Engine::MySql,
        host: &params.host,
        port: params.port,
        database: params.database.as_deref(),
        password: params.password.as_deref(),
        tls: params.tls,
    }
}

impl DatabaseSession for MySqlSession {
    async fn connect(params: &ConnectParams) -> Result<Self, AppError> {
        let (target_host, target_port) = match params.connect_to {
            Some(address) => (address.ip().to_string(), address.port()),
            None => (params.host.clone(), params.port),
        };
        let mut options = MySqlConnectOptions::new()
            .host(&target_host)
            .port(target_port)
            .username(&params.username)
            .charset("utf8mb4")
            .ssl_mode(match params.tls {
                // Through a tunnel the TLS peer is 127.0.0.1, so only the
                // certificate chain can be checked; SSH host keys protect the path.
                TlsMode::Verify if params.connect_to.is_some() => MySqlSslMode::VerifyCa,
                TlsMode::Verify => MySqlSslMode::VerifyIdentity,
                TlsMode::EncryptOnly => MySqlSslMode::Required,
                TlsMode::Off => MySqlSslMode::Disabled,
            });
        if let Some(password) = &params.password {
            options = options.password(password);
        }
        if let Some(database) = &params.database {
            options = options.database(database);
        }
        if let Some(ca) = &params.ca_path {
            options = options.ssl_ca(ca);
        }
        // Mutual TLS: present this certificate and key to the server.
        if let (Some(cert), Some(key)) = (&params.client_cert, &params.client_key) {
            options = options.ssl_client_cert(cert).ssl_client_key(key);
        }
        let connection = options
            .connect()
            .await
            .map_err(|e| error::from_sqlx(e, context(params)))?;

        let mut session = Self {
            connection,
            info: ServerInfo {
                engine: Engine::MySql,
                version: String::new(),
                database: params.database.clone(),
            },
            connection_id: 0,
            host: params.host.clone(),
            port: params.port,
        };
        let row = session
            .execute("SELECT VERSION(), CONNECTION_ID()", 1)
            .await?;
        let mut cells = row.rows.into_iter().next().unwrap_or_default().into_iter();
        if let Some(Cell::Value(version)) = cells.next() {
            // "8.4.2" or "11.4.2-MariaDB-ubu2404"
            let number = version.split('-').next().unwrap_or(&version);
            let product = if version.to_lowercase().contains("mariadb") {
                "MariaDB"
            } else {
                "MySQL"
            };
            session.info.version = format!("{product} {number}");
        }
        if let Some(Cell::Value(id)) = cells.next() {
            session.connection_id = id.parse().unwrap_or_default();
        }
        Ok(session)
    }

    fn server_info(&self) -> &ServerInfo {
        &self.info
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            cancel_query: true,
            schemas: false,
            rows_affected: true,
            multiple_result_sets: false,
        }
    }

    async fn ping(&mut self) -> Result<(), AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::MySql, &host, self.port);
        self.connection
            .ping()
            .await
            .map_err(|e| error::from_sqlx(e, ctx))
    }

    async fn list_databases(&mut self) -> Result<Vec<String>, AppError> {
        Ok(first_column(self.execute("SHOW DATABASES", 10_000).await?))
    }

    async fn execute_bound(
        &mut self,
        sql: &str,
        params: &[values::Param],
        max_rows: usize,
    ) -> Result<QueryResult, AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::MySql, &host, self.port);
        // Bound, so the prepared protocol: the values never touch the text.
        let mut query = sqlx::query(AssertSqlSafe(sql.to_owned()));
        for param in params {
            query = match param {
                values::Param::Text(text) => query.bind(text.clone()),
                // MySQL converts a decimal string exactly; a float would not.
                values::Param::Number(text) => {
                    if text.trim().parse::<f64>().is_err() {
                        return Err(AppError::new(
                            "invalid_parameter",
                            format!("“{text}” isn't a number."),
                        ));
                    }
                    query.bind(text.trim().to_owned())
                }
                values::Param::Null => query.bind(Option::<String>::None),
            };
        }
        let rows = query
            .fetch_all(&mut self.connection)
            .await
            .map_err(|e| error::from_sqlx(e, ctx))?;
        let mut result = QueryResult::default();
        if let Some(first) = rows.first() {
            result.columns = first
                .columns()
                .iter()
                .map(|c| ColumnMeta {
                    name: c.name().to_owned(),
                    type_name: c.type_info().name().to_owned(),
                })
                .collect();
        }
        for row in rows {
            if result.rows.len() >= max_rows {
                result.truncated = true;
                break;
            }
            result
                .rows
                .push((0..row.len()).map(|i| bound_cell(&row, i)).collect());
        }
        if !result.columns.is_empty() {
            result.rows_affected = None;
        }
        Ok(result)
    }

    async fn execute_into(&mut self, sql: &str, sink: &values::SharedSink) -> Result<(), AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::MySql, &host, self.port);
        // Without arguments SQLx uses MySQL's text protocol, so values
        // arrive as text (binary columns stay bytes). Every row set is kept.
        let mut stream =
            sqlx::raw_sql(AssertSqlSafe(sql.to_owned())).fetch_many(&mut self.connection);
        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| error::from_sqlx(e, ctx))?
        {
            let mut sink = sink.lock().expect("row sink");
            match item {
                Either::Left(done) => {
                    if sink.is_open() {
                        sink.end();
                    } else {
                        sink.affected(done.rows_affected());
                    }
                }
                Either::Right(row) => {
                    if !sink.is_open() {
                        sink.begin(
                            row.columns()
                                .iter()
                                .map(|c| ColumnMeta {
                                    name: c.name().to_owned(),
                                    type_name: c.type_info().name().to_owned(),
                                })
                                .collect(),
                        );
                    }
                    if sink.wants_row() {
                        sink.push((0..row.len()).map(|i| cell(&row, i)).collect());
                    }
                }
            }
        }
        Ok(())
    }

    async fn query_text(&mut self, sql: &str, params: &[&str]) -> Result<TextRows, AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::MySql, &host, self.port);
        let mut query = sqlx::query(AssertSqlSafe(sql.to_owned()));
        for param in params {
            query = query.bind(*param);
        }
        let rows = query
            .fetch_all(&mut self.connection)
            .await
            .map_err(|e| error::from_sqlx(e, ctx))?;
        Ok(rows
            .iter()
            .map(|row| {
                (0..row.len())
                    .map(|i| row.try_get_unchecked::<Option<String>, _>(i).ok().flatten())
                    .collect()
            })
            .collect())
    }

    async fn list_schemas(&mut self, _database: &str) -> Result<Option<Vec<String>>, AppError> {
        // In MySQL a database is a schema; tables sit directly under it.
        Ok(None)
    }

    async fn list_objects(
        &mut self,
        database: &str,
        _schema: &str,
    ) -> Result<Vec<DbObject>, AppError> {
        let rows = self
            .query_text(
                "SELECT CONVERT(TABLE_NAME USING utf8mb4),
                        CASE TABLE_TYPE WHEN 'VIEW' THEN 'view' ELSE 'table' END
                 FROM information_schema.TABLES
                 WHERE TABLE_SCHEMA = ?
                 ORDER BY TABLE_NAME",
                &[database],
            )
            .await?;
        Ok(metadata::object_rows(rows))
    }

    async fn describe(
        &mut self,
        database: &str,
        _schema: &str,
        name: &str,
        kind: ObjectKind,
    ) -> Result<ObjectStructure, AppError> {
        let columns = self
            .query_text(
                "SELECT CONVERT(c.COLUMN_NAME USING utf8mb4),
                        CONVERT(c.COLUMN_TYPE USING utf8mb4),
                        CONVERT(c.IS_NULLABLE USING utf8mb4),
                        CONVERT(c.COLUMN_DEFAULT USING utf8mb4),
                        CAST((SELECT k.ORDINAL_POSITION
                              FROM information_schema.KEY_COLUMN_USAGE k
                              WHERE k.TABLE_SCHEMA = ? AND k.TABLE_NAME = ?
                                AND k.COLUMN_NAME = c.COLUMN_NAME AND k.CONSTRAINT_NAME = 'PRIMARY')
                             AS CHAR),
                        NULL,
                        (SELECT CONVERT(CONCAT_WS(CHAR(31 USING utf8mb4), k.REFERENCED_TABLE_SCHEMA,
                                          k.REFERENCED_TABLE_NAME, k.REFERENCED_COLUMN_NAME) USING utf8mb4)
                         FROM information_schema.KEY_COLUMN_USAGE k
                         WHERE k.TABLE_SCHEMA = ? AND k.TABLE_NAME = ?
                           AND k.COLUMN_NAME = c.COLUMN_NAME AND k.REFERENCED_TABLE_NAME IS NOT NULL
                           AND (SELECT COUNT(*) FROM information_schema.KEY_COLUMN_USAGE k2
                                WHERE k2.TABLE_SCHEMA = ? AND k2.TABLE_NAME = ?
                                  AND k2.CONSTRAINT_NAME = k.CONSTRAINT_NAME) = 1
                         LIMIT 1)
                 FROM information_schema.COLUMNS c
                 WHERE c.TABLE_SCHEMA = ? AND c.TABLE_NAME = ?
                 ORDER BY c.ORDINAL_POSITION",
                // Every information_schema lookup names the table with
                // constants. MySQL 5.7 can only find a table's rows directly
                // that way; matched against another column instead, it reads
                // every table in every database, which is slow and leaves
                // thousands of table files open.
                &[
                    database, name, database, name, database, name, database, name,
                ],
            )
            .await?;
        let indexes = if kind == ObjectKind::Table {
            let rows = self
                .query_text(
                    "SELECT CONVERT(INDEX_NAME USING utf8mb4),
                            IF(MAX(NON_UNIQUE) = 0, 'true', 'false'),
                            IF(INDEX_NAME = 'PRIMARY', 'true', 'false'),
                            CONVERT(GROUP_CONCAT(COLUMN_NAME ORDER BY SEQ_IN_INDEX SEPARATOR ', ')
                                    USING utf8mb4)
                     FROM information_schema.STATISTICS
                     WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
                     GROUP BY INDEX_NAME
                     ORDER BY INDEX_NAME",
                    &[database, name],
                )
                .await?;
            Some(metadata::index_rows(rows))
        } else {
            None
        };
        // The server's own CREATE TABLE, for a dump. The statement is one row
        // of two columns: the header, then the statement.
        let shown = self
            .query_text(
                &format!(
                    "SHOW CREATE TABLE {}",
                    quote::table(Engine::MySql, database, "", name)
                ),
                &[],
            )
            .await
            .ok()
            .and_then(|rows| {
                rows.into_iter().next().and_then(|mut row| {
                    row.resize(2, None);
                    let mut cells = row.into_iter();
                    match (cells.next(), cells.next()) {
                        (Some(_), Some(Some(create))) => Some(create),
                        _ => None,
                    }
                })
            });
        let constraints = if kind == ObjectKind::Table {
            let mut rows = self
                .query_text(
                    &metadata::constraints_sql(Engine::MySql, database),
                    &[database, name, database, name],
                )
                .await?;
            // Older servers have no check constraints to read; a table there
            // must still open, or it can't be edited at all.
            if let Ok(checks) = self
                .query_text(metadata::mysql_checks_sql(), &[database, name])
                .await
            {
                rows.extend(checks);
            }
            Some(metadata::constraint_rows(rows))
        } else {
            None
        };
        Ok(ObjectStructure {
            constraints,
            columns: metadata::column_rows(columns),
            indexes,
            show_create: shown,
        })
    }

    fn preview_sql(&self, database: &str, _schema: &str, name: &str, limit: usize) -> String {
        format!(
            "SELECT * FROM {} LIMIT {limit}",
            quote::qualified(Engine::MySql, &[database, name])
        )
    }

    async fn close(self) -> Result<(), AppError> {
        self.connection
            .close()
            .await
            .map_err(|_| AppError::connection_close_failed())
    }
}

/// Binary column types. sqlx also gives `_bin`-collated text (for example
/// information_schema names) these names, so the type alone can't decide.
fn is_binary_type(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "BINARY" | "VARBINARY" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "GEOMETRY"
    )
}

/// Bytes that decoded as UTF-8 but still aren't text: control characters
/// other than tab and line breaks only appear in binary data.
fn looks_binary(text: &str) -> bool {
    text.chars()
        .any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
}

fn cell(row: &sqlx::mysql::MySqlRow, index: usize) -> Cell {
    use sqlx::{Column, Row, TypeInfo};
    let bytes = || match row.try_get_unchecked::<Option<Vec<u8>>, _>(index) {
        Ok(Some(bytes)) => Cell::binary(&bytes),
        _ => Cell::Null,
    };
    match row.try_get_unchecked::<Option<String>, _>(index) {
        Ok(Some(text))
            if looks_binary(&text) && is_binary_type(row.columns()[index].type_info().name()) =>
        {
            Cell::binary(text.as_bytes())
        }
        Ok(Some(text)) => Cell::Value(text),
        Ok(None) => Cell::Null,
        Err(_) => bytes(),
    }
}

/// A value from a bound query, by its column's type.
///
/// A query with parameters goes through MySQL's prepared protocol, which
/// sends numbers, dates and times in binary rather than as text. Read as text,
/// 42 came out as `*\0\0\0\0\0\0\0` and a date as a blob. Each type is
/// read as itself and written the way the text protocol would show it, so a
/// column reads the same with or without a parameter.
fn bound_cell(row: &sqlx::mysql::MySqlRow, index: usize) -> Cell {
    use sqlx::{Column, Row, TypeInfo};
    let type_name = row.columns()[index].type_info().name().to_ascii_uppercase();
    macro_rules! read {
        ($type:ty, $render:expr) => {
            match row.try_get_unchecked::<Option<$type>, _>(index) {
                Ok(Some(value)) => Cell::Value(($render)(value)),
                Ok(None) => Cell::Null,
                Err(_) => cell(row, index),
            }
        };
    }
    match type_name.as_str() {
        name if name.ends_with("UNSIGNED") && !name.starts_with("DECIMAL") => {
            read!(u64, |v: u64| v.to_string())
        }
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "BIGINT" | "BOOLEAN" | "YEAR" => {
            read!(i64, |v: i64| v.to_string())
        }
        "DOUBLE" => read!(f64, |v: f64| v.to_string()),
        "FLOAT" => read!(f32, |v: f32| v.to_string()),
        "DATE" => read!(chrono::NaiveDate, |v: chrono::NaiveDate| v.to_string()),
        "DATETIME" | "TIMESTAMP" => {
            read!(chrono::NaiveDateTime, |v: chrono::NaiveDateTime| v
                .to_string())
        }
        "TIME" => match row.try_get_unchecked::<Option<Vec<u8>>, _>(index) {
            Ok(Some(bytes)) => Cell::Value(binary_time(&bytes)),
            Ok(None) => Cell::Null,
            Err(_) => cell(row, index),
        },
        // Decimals, text, enums and JSON arrive as text in both protocols.
        _ => cell(row, index),
    }
}

/// MySQL's binary TIME: sign, days, hours, minutes, seconds, microseconds.
/// It can be negative and longer than a day, so it isn't a time of day.
fn binary_time(bytes: &[u8]) -> String {
    if bytes.len() < 8 {
        return "00:00:00".to_owned();
    }
    let negative = bytes[0] == 1;
    let days = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
    let hours = u64::from(days) * 24 + u64::from(bytes[5]);
    let mut text = format!(
        "{}{:02}:{:02}:{:02}",
        if negative { "-" } else { "" },
        hours,
        bytes[6],
        bytes[7]
    );
    if bytes.len() >= 12 {
        let micros = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        if micros > 0 {
            text.push_str(&format!(".{micros:06}"));
        }
    }
    text
}

#[cfg(test)]
mod binary_tests {
    use super::looks_binary;

    #[test]
    fn binary_times_can_be_negative_and_longer_than_a_day() {
        use super::binary_time;
        assert_eq!(binary_time(&[0, 1, 0, 0, 0, 2, 3, 4]), "26:03:04");
        assert_eq!(
            binary_time(&[1, 0, 0, 0, 0, 0, 30, 0, 64, 226, 1, 0]),
            "-00:30:00.123456"
        );
    }

    #[test]
    fn control_characters_mark_binary() {
        assert!(looks_binary("\u{0}\u{1}abc"));
        assert!(!looks_binary("Zoë\tline\nnext"));
    }
}
