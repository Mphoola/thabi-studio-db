use super::{
    Capabilities, Cell, ColumnMeta, ConnectParams, DatabaseSession, Engine, QueryResult,
    ServerInfo, TlsMode, error, first_column,
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
            },
            host: params.host.clone(),
            port: params.port,
        };
        let version = first_column(session.execute("SELECT @@VERSION", 1).await?);
        if let Some(version) = version.first() {
            // "Microsoft SQL Server 2022 (RTM-CU14) (KB…) - 16.0.4135.4 (X64) …"
            let product = version.split(" (").next().unwrap_or(version).trim();
            session.info.version = product.to_owned();
        }
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

    async fn execute(&mut self, sql: &str, max_rows: usize) -> Result<QueryResult, AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::SqlServer, &host, self.port);
        let mut stream = self
            .client
            .simple_query(sql)
            .await
            .map_err(|e| error::from_tiberius(e, ctx))?;
        let mut result = QueryResult::default();
        let mut row_sets = 0usize;
        // Tiberius needs the stream read to the end before the next query,
        // so rows past the limit are read and dropped.
        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| error::from_tiberius(e, ctx))?
        {
            match item {
                QueryItem::Metadata(meta) => {
                    row_sets += 1;
                    if row_sets == 1 {
                        result.columns = meta
                            .columns()
                            .iter()
                            .map(|c| ColumnMeta {
                                name: c.name().to_owned(),
                                type_name: format!("{:?}", c.column_type()),
                            })
                            .collect();
                    }
                }
                QueryItem::Row(row) => {
                    if row_sets > 1 {
                        continue;
                    }
                    if result.rows.len() >= max_rows {
                        result.truncated = true;
                        continue;
                    }
                    result
                        .rows
                        .push(row.into_iter().map(|data| cell(&data)).collect());
                }
            }
        }
        result.extra_result_sets = row_sets.saturating_sub(1);
        Ok(result)
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
