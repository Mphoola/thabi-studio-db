use super::{
    Capabilities, Cell, ColumnMeta, ConnectParams, DatabaseSession, DbObject, Engine, ObjectKind,
    ObjectStructure, QueryResult, ServerInfo, TextRows, TlsMode, error, first_column, metadata,
    quote, values,
};
use crate::errors::AppError;
use futures_util::TryStreamExt;
use tiberius::{
    AuthMethod, Client, ColumnData, Config, EncryptionLevel, FromSql, QueryItem,
    time::chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime},
};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub struct SqlServerSession {
    client: Client<Compat<TcpStream>>,
    info: ServerInfo,
    host: String,
    port: u16,
}

fn context(params: &ConnectParams) -> error::Context<'_> {
    error::Context {
        engine: Engine::SqlServer,
        host: &params.host,
        port: params.port,
        database: params.database.as_deref(),
        password: params.password.as_deref(),
        tls: params.tls,
    }
}

impl DatabaseSession for SqlServerSession {
    async fn connect(params: &ConnectParams) -> Result<Self, AppError> {
        let ctx = context(params);
        let mut config = Config::new();
        config.host(&params.host);
        config.port(params.port);
        config.application_name("Thabi Studio");
        config.authentication(AuthMethod::sql_server(
            &params.username,
            params.password.as_deref().unwrap_or_default(),
        ));
        if let Some(database) = &params.database {
            config.database(database);
        }
        match params.tls {
            TlsMode::Verify => {
                config.encryption(EncryptionLevel::Required);
                if let Some(ca) = &params.ca_path {
                    config.trust_cert_ca(ca.to_string_lossy());
                }
            }
            TlsMode::EncryptOnly => {
                config.encryption(EncryptionLevel::Required);
                config.trust_cert();
            }
            TlsMode::Off => config.encryption(EncryptionLevel::NotSupported),
        }

        // Mutual TLS: Tiberius reads PEM or DER files with the rustls backend
        // this build uses. Required by TDS 8.0 strict connections.
        if let (Some(cert), Some(key)) = (&params.client_cert, &params.client_key) {
            config.client_certificate(
                cert.to_string_lossy().into_owned(),
                key.to_string_lossy().into_owned(),
            );
        }

        // Through a tunnel, TCP goes to the local end while TLS still checks
        // the real host name (config.host above).
        let tcp = match params.connect_to {
            Some(address) => TcpStream::connect(address).await,
            None => TcpStream::connect((params.host.as_str(), params.port)).await,
        }
        .map_err(|e| error::from_io(e.kind(), &e.to_string(), ctx))?;
        tcp.set_nodelay(true)
            .map_err(|e| error::from_io(e.kind(), &e.to_string(), ctx))?;
        let client = Client::connect(config, tcp.compat_write())
            .await
            .map_err(|e| error::from_tiberius(e, ctx))?;

        let mut session = Self {
            client,
            info: ServerInfo {
                engine: Engine::SqlServer,
                version: String::new(),
                database: None,
            },
            host: params.host.clone(),
            port: params.port,
        };
        let row = session
            .query_text("SELECT @@VERSION, DB_NAME()", &[])
            .await?;
        let mut cells = row.into_iter().next().unwrap_or_default().into_iter();
        if let Some(version) = cells.next().flatten() {
            // "Microsoft SQL Server 2022 (RTM-CU14) (KB…) - 16.0.4135.4 (X64) …"
            let product = version.split(" (").next().unwrap_or(&version).trim();
            session.info.version = product.to_owned();
        }
        session.info.database = cells.next().flatten();
        Ok(session)
    }

    fn server_info(&self) -> &ServerInfo {
        &self.info
    }

    fn capabilities(&self) -> Capabilities {
        // Tiberius has no query cancellation, and simple queries don't report
        // rows affected.
        Capabilities {
            cancel_query: false,
            schemas: true,
            rows_affected: false,
            multiple_result_sets: false,
        }
    }

    async fn ping(&mut self) -> Result<(), AppError> {
        self.execute("SELECT 1", 1).await.map(|_| ())
    }

    async fn list_databases(&mut self) -> Result<Vec<String>, AppError> {
        let result = self
            .execute(
                "SELECT name FROM sys.databases WHERE HAS_DBACCESS(name) = 1 ORDER BY name",
                10_000,
            )
            .await?;
        Ok(first_column(result))
    }

    /// Not yet: binding needs the prepared protocol, which returns typed
    /// values rather than text, and this engine's grid values come from the
    /// simple protocol on purpose. Saying so is better than running the
    /// statement with the values missing.
    async fn execute_bound(
        &mut self,
        sql: &str,
        params: &[values::Param],
        max_rows: usize,
    ) -> Result<QueryResult, AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::SqlServer, &host, self.port);
        // `@p1`, `@p2`… are bound through sp_executesql: the values never
        // become part of the statement. Rows come back typed, exactly as the
        // simple path's do, so both share one reader.
        let mut query = tiberius::Query::new(sql.to_owned());
        for param in params {
            match param {
                values::Param::Text(text) => query.bind(text.clone()),
                values::Param::Number(text) => match numeric(text) {
                    Some(value) => query.bind(value),
                    None => {
                        return Err(AppError::new(
                            "invalid_parameter",
                            format!("“{text}” isn't a number."),
                        ));
                    }
                },
                values::Param::Null => query.bind(Option::<String>::None),
            }
        }
        let stream = query
            .query(&mut self.client)
            .await
            .map_err(|e| error::from_tiberius(e, ctx))?;
        let sink = values::RowSink::shared(max_rows);
        read_stream(stream, &sink, ctx).await?;
        Ok(sink.lock().expect("row sink").take())
    }

    async fn execute_into(&mut self, sql: &str, sink: &values::SharedSink) -> Result<(), AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::SqlServer, &host, self.port);
        let stream = self
            .client
            .simple_query(sql)
            .await
            .map_err(|e| error::from_tiberius(e, ctx))?;
        read_stream(stream, sink, ctx).await
    }

    async fn query_text(&mut self, sql: &str, params: &[&str]) -> Result<TextRows, AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::SqlServer, &host, self.port);
        let params: Vec<&dyn tiberius::ToSql> =
            params.iter().map(|p| p as &dyn tiberius::ToSql).collect();
        let rows = self
            .client
            .query(sql, &params)
            .await
            .map_err(|e| error::from_tiberius(e, ctx))?
            .into_first_result()
            .await
            .map_err(|e| error::from_tiberius(e, ctx))?;
        Ok(rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|data| match cell(&data) {
                        Cell::Value(text) => Some(text),
                        _ => None,
                    })
                    .collect()
            })
            .collect())
    }

    async fn list_schemas(&mut self, database: &str) -> Result<Option<Vec<String>>, AppError> {
        let db = quote::ident(Engine::SqlServer, database);
        let rows = self
            .query_text(
                &format!(
                    "SELECT name FROM {db}.sys.schemas
                     WHERE name NOT IN ('sys', 'INFORMATION_SCHEMA', 'guest')
                       AND name NOT LIKE 'db[_]%'
                     ORDER BY name"
                ),
                &[],
            )
            .await?;
        Ok(Some(metadata::names(rows)))
    }

    async fn list_objects(
        &mut self,
        database: &str,
        schema: &str,
    ) -> Result<Vec<DbObject>, AppError> {
        let db = quote::ident(Engine::SqlServer, database);
        let rows = self
            .query_text(
                &format!(
                    "SELECT o.name, CASE o.type WHEN 'V' THEN 'view' ELSE 'table' END
                     FROM {db}.sys.objects o JOIN {db}.sys.schemas s ON s.schema_id = o.schema_id
                     WHERE s.name = @P1 AND o.type IN ('U', 'V')
                     ORDER BY o.name"
                ),
                &[schema],
            )
            .await?;
        Ok(metadata::object_rows(rows))
    }

    async fn describe(
        &mut self,
        database: &str,
        schema: &str,
        name: &str,
        kind: ObjectKind,
    ) -> Result<ObjectStructure, AppError> {
        let db = quote::ident(Engine::SqlServer, database);
        let columns = self
            .query_text(
                &format!(
                    "SELECT c.name,
                            t.name + CASE
                              WHEN t.name IN ('varchar', 'char', 'varbinary', 'binary')
                                THEN '(' + IIF(c.max_length = -1, 'max', CAST(c.max_length AS varchar(10))) + ')'
                              WHEN t.name IN ('nvarchar', 'nchar')
                                THEN '(' + IIF(c.max_length = -1, 'max', CAST(c.max_length / 2 AS varchar(10))) + ')'
                              WHEN t.name IN ('decimal', 'numeric')
                                THEN '(' + CAST(c.precision AS varchar(10)) + ',' + CAST(c.scale AS varchar(10)) + ')'
                              ELSE '' END,
                            IIF(c.is_nullable = 1, 'true', 'false'),
                            dc.definition,
                            CAST(ic.key_ordinal AS varchar(10)),
                            NULL,
                            (SELECT TOP 1 CONCAT(rs.name, CHAR(31), ro.name, CHAR(31), rc.name)
                             FROM {db}.sys.foreign_key_columns fkc
                             JOIN {db}.sys.objects ro ON ro.object_id = fkc.referenced_object_id
                             JOIN {db}.sys.schemas rs ON rs.schema_id = ro.schema_id
                             JOIN {db}.sys.columns rc
                               ON rc.object_id = fkc.referenced_object_id AND rc.column_id = fkc.referenced_column_id
                             WHERE fkc.parent_object_id = o.object_id AND fkc.parent_column_id = c.column_id
                               AND (SELECT COUNT(*) FROM {db}.sys.foreign_key_columns x
                                    WHERE x.constraint_object_id = fkc.constraint_object_id) = 1)
                     FROM {db}.sys.columns c
                     JOIN {db}.sys.objects o ON o.object_id = c.object_id
                     JOIN {db}.sys.schemas s ON s.schema_id = o.schema_id
                     JOIN {db}.sys.types t ON t.user_type_id = c.user_type_id
                     LEFT JOIN {db}.sys.default_constraints dc ON dc.object_id = c.default_object_id
                     LEFT JOIN {db}.sys.indexes i ON i.object_id = o.object_id AND i.is_primary_key = 1
                     LEFT JOIN {db}.sys.index_columns ic
                       ON ic.object_id = i.object_id AND ic.index_id = i.index_id AND ic.column_id = c.column_id
                     WHERE s.name = @P1 AND o.name = @P2
                     ORDER BY c.column_id"
                ),
                &[schema, name],
            )
            .await?;
        // Views can be indexed in SQL Server, so ask for both kinds.
        let _ = kind;
        let rows = self
            .query_text(
                &format!(
                    "SELECT i.name, IIF(i.is_unique = 1, 'true', 'false'),
                            IIF(i.is_primary_key = 1, 'true', 'false'),
                            STRING_AGG(c.name, ', ') WITHIN GROUP (ORDER BY ic.key_ordinal)
                     FROM {db}.sys.indexes i
                     JOIN {db}.sys.objects o ON o.object_id = i.object_id
                     JOIN {db}.sys.schemas s ON s.schema_id = o.schema_id
                     JOIN {db}.sys.index_columns ic
                       ON ic.object_id = i.object_id AND ic.index_id = i.index_id AND ic.is_included_column = 0
                     JOIN {db}.sys.columns c ON c.object_id = ic.object_id AND c.column_id = ic.column_id
                     WHERE s.name = @P1 AND o.name = @P2 AND i.type > 0
                     GROUP BY i.name, i.is_unique, i.is_primary_key
                     ORDER BY i.name"
                ),
                &[schema, name],
            )
            .await?;
        Ok(ObjectStructure {
            columns: metadata::column_rows(columns),
            indexes: Some(metadata::index_rows(rows)),
        })
    }

    fn preview_sql(&self, database: &str, schema: &str, name: &str, limit: usize) -> String {
        format!(
            "SELECT TOP ({limit}) * FROM {}",
            quote::qualified(Engine::SqlServer, &[database, schema, name])
        )
    }

    async fn close(self) -> Result<(), AppError> {
        self.client
            .close()
            .await
            .map_err(|_| AppError::connection_close_failed())
    }
}

fn text<T: ToString>(value: Option<T>) -> Cell {
    value.map_or(Cell::Null, |v| Cell::Value(v.to_string()))
}

fn convert<'a, T: FromSql<'a>>(
    data: &'a ColumnData<'static>,
    format: impl Fn(T) -> String,
) -> Cell {
    match T::from_sql(data) {
        Ok(Some(value)) => Cell::Value(format(value)),
        _ => Cell::Null,
    }
}

/// Converts one SQL Server value to its display text.
fn cell(data: &ColumnData<'static>) -> Cell {
    match data {
        ColumnData::U8(v) => text(*v),
        ColumnData::I16(v) => text(*v),
        ColumnData::I32(v) => text(*v),
        ColumnData::I64(v) => text(*v),
        ColumnData::F32(v) => text(*v),
        ColumnData::F64(v) => text(*v),
        ColumnData::Bit(v) => text(v.map(u8::from)),
        ColumnData::String(v) => text(v.as_deref()),
        ColumnData::Guid(v) => text(v.map(|g| g.to_string().to_uppercase())),
        ColumnData::Binary(v) => v.as_deref().map_or(Cell::Null, Cell::binary),
        ColumnData::Numeric(v) => text(*v),
        ColumnData::Xml(v) => text(v.as_deref()),
        ColumnData::DateTime(_) | ColumnData::SmallDateTime(_) | ColumnData::DateTime2(_) => {
            convert(data, |v: NaiveDateTime| {
                v.format("%Y-%m-%d %H:%M:%S%.f").to_string()
            })
        }
        ColumnData::Date(_) => convert(data, |v: NaiveDate| v.to_string()),
        ColumnData::Time(_) => convert(data, |v: NaiveTime| v.to_string()),
        ColumnData::DateTimeOffset(_) => convert(data, |v: DateTime<FixedOffset>| {
            v.format("%Y-%m-%d %H:%M:%S%.f %:z").to_string()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn converts_common_values_to_text() {
        assert_eq!(
            cell(&ColumnData::I64(Some(9_007_199_254_740_993))),
            Cell::Value("9007199254740993".into())
        );
        assert_eq!(cell(&ColumnData::Bit(Some(true))), Cell::Value("1".into()));
        assert_eq!(cell(&ColumnData::String(None)), Cell::Null);
        assert_eq!(
            cell(&ColumnData::String(Some(Cow::Borrowed("Zoë")))),
            Cell::Value("Zoë".into())
        );
        assert!(matches!(
            cell(&ColumnData::Binary(Some(Cow::Owned(vec![1, 2, 3])))),
            Cell::Binary { bytes: 3, .. }
        ));
    }
}

/// A decimal string as a SQL Server NUMERIC, keeping every digit.
fn numeric(text: &str) -> Option<tiberius::numeric::Numeric> {
    let text = text.trim();
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if fraction.len() > 37 || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let value: i128 = format!("{whole}{fraction}").parse().ok()?;
    Some(tiberius::numeric::Numeric::new_with_scale(
        value,
        fraction.len() as u8,
    ))
}

/// Reads a result stream into the sink: every row set is kept. Tiberius
/// needs the stream read to the end before the next query, so rows past the
/// limit are read and dropped.
async fn read_stream(
    mut stream: tiberius::QueryStream<'_>,
    sink: &values::SharedSink,
    ctx: error::Context<'_>,
) -> Result<(), AppError> {
    while let Some(item) = stream
        .try_next()
        .await
        .map_err(|e| error::from_tiberius(e, ctx))?
    {
        let mut sink = sink.lock().expect("row sink");
        match item {
            QueryItem::Metadata(meta) => {
                if sink.is_open() {
                    sink.end();
                }
                sink.begin(
                    meta.columns()
                        .iter()
                        .map(|c| ColumnMeta {
                            name: c.name().to_owned(),
                            type_name: format!("{:?}", c.column_type()),
                        })
                        .collect(),
                );
            }
            QueryItem::Row(row) => {
                if sink.wants_row() {
                    sink.push(row.into_iter().map(|data| cell(&data)).collect());
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod numeric_tests {
    use super::numeric;

    #[test]
    fn decimals_keep_every_digit() {
        let n = numeric("12345678901234567890.125").unwrap();
        assert_eq!(n.value(), 12345678901234567890125);
        assert_eq!(n.scale(), 3);
        assert_eq!(numeric("-7").unwrap().value(), -7);
        assert!(numeric("1e5").is_none());
        assert!(numeric("abc").is_none());
    }
}
