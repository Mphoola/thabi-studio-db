//! Users, roles and what they may do (Phase 15).
//!
//! Each engine's own model: PostgreSQL roles (some can log in), MySQL
//! accounts as user@host with MySQL 8 roles, SQL Server logins with a user
//! in each database. Changes are statements shown before they run, with the
//! password masked in what is shown; the password itself is only ever in the
//! statement sent, and is scrubbed from any error the server returns.

use super::{Cell, Engine, Session, quote, values::Param};
use crate::errors::AppError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub name: String,
    /// MySQL accounts are user@host; empty elsewhere.
    pub host: String,
    /// "role", "login", "user" (each engine's word for what this is).
    pub kind: String,
    pub can_login: bool,
    pub superuser: bool,
    pub roles: Vec<String>,
}

fn text(row: &[Cell], i: usize) -> String {
    match row.get(i) {
        Some(Cell::Value(v)) => v.clone(),
        _ => String::new(),
    }
}

fn truthy(row: &[Cell], i: usize) -> bool {
    matches!(text(row, i).as_str(), "1" | "t" | "true" | "Y")
}

fn list(row: &[Cell], i: usize) -> Vec<String> {
    let value = text(row, i);
    value
        .split(super::metadata::SEP)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Everyone who can be given rights: logins, users and roles.
pub async fn list_users(session: &mut Session, database: &str) -> Result<Vec<UserInfo>, AppError> {
    let engine = session.server_info().engine;
    let sql = match engine {
        Engine::Postgres => "SELECT r.rolname::text, '', CASE WHEN r.rolcanlogin THEN 'login' ELSE 'role' END,
                    r.rolcanlogin::text, r.rolsuper::text,
                    (SELECT string_agg(b.rolname, chr(31) ORDER BY b.rolname)
                     FROM pg_auth_members m JOIN pg_roles b ON b.oid = m.roleid WHERE m.member = r.oid)
             FROM pg_roles r WHERE r.rolname !~ '^pg_' ORDER BY 1"
            .to_owned(),
        Engine::MySql => "SELECT CONVERT(u.User USING utf8mb4), CONVERT(u.Host USING utf8mb4),
                    IF(u.account_locked = 'Y' AND u.authentication_string = '', 'role', 'user'),
                    IF(u.account_locked = 'Y', 0, 1), IF(u.Super_priv = 'Y', 1, 0),
                    (SELECT CONVERT(GROUP_CONCAT(e.FROM_USER ORDER BY e.FROM_USER SEPARATOR 0x1f) USING utf8mb4)
                     FROM mysql.role_edges e WHERE e.TO_USER = u.User AND e.TO_HOST = u.Host)
             FROM mysql.user u
             WHERE u.User NOT IN ('mysql.sys', 'mysql.session', 'mysql.infoschema')
             ORDER BY 1, 2"
            .to_owned(),
        Engine::SqlServer => {
            let db = quote::ident(engine, database);
            format!(
                "SELECT p.name, '', LOWER(p.type_desc), IIF(sp.is_disabled = 0, 1, 0),
                        IIF(IS_SRVROLEMEMBER('sysadmin', sp.name) = 1, 1, 0),
                        (SELECT STRING_AGG(r.name, CHAR(31)) WITHIN GROUP (ORDER BY r.name)
                         FROM {db}.sys.database_role_members m
                         JOIN {db}.sys.database_principals r ON r.principal_id = m.role_principal_id
                         WHERE m.member_principal_id = p.principal_id)
                 FROM {db}.sys.database_principals p
                 LEFT JOIN sys.server_principals sp ON sp.sid = p.sid
                 WHERE p.type IN ('S', 'U', 'G', 'E', 'X', 'R') AND p.is_fixed_role = 0
                   AND p.name NOT IN ('guest', 'INFORMATION_SCHEMA', 'sys', 'public')
                   AND p.name NOT LIKE '##%'
                 ORDER BY 1"
            )
        }
    };
    let result = session.execute(&sql, 10_000).await?;
    Ok(result
        .rows
        .iter()
        .map(|row| UserInfo {
            name: text(row, 0),
            host: text(row, 1),
            kind: text(row, 2).replace("_user", "").replace("sql_", ""),
            can_login: truthy(row, 3),
            superuser: truthy(row, 4),
            roles: list(row, 5),
        })
        .collect())
}

/// What a user may do, as the GRANT statements that would give it.
pub async fn grants(
    session: &mut Session,
    database: &str,
    name: &str,
    host: &str,
) -> Result<Vec<String>, AppError> {
    let engine = session.server_info().engine;
    let (sql, params): (String, Vec<Param>) = match engine {
        Engine::MySql => (format!("SHOW GRANTS FOR {}", account(name, host)), vec![]),
        Engine::Postgres => (
            "SELECT format('GRANT %s ON %s TO %I;', string_agg(privilege_type, ', ' ORDER BY privilege_type),
                           format('%I.%I', table_schema, table_name), grantee)
             FROM information_schema.role_table_grants WHERE grantee = $1
             GROUP BY table_schema, table_name, grantee
             UNION ALL
             SELECT format('GRANT %s ON DATABASE %I TO %I;', a.privilege_type, d.datname, $1)
             FROM pg_database d, aclexplode(d.datacl) a
             WHERE a.grantee = (SELECT oid FROM pg_roles WHERE rolname = $1)
             UNION ALL
             SELECT format('GRANT %I TO %I;', b.rolname, $1)
             FROM pg_auth_members m JOIN pg_roles b ON b.oid = m.roleid
             WHERE m.member = (SELECT oid FROM pg_roles WHERE rolname = $1)
             ORDER BY 1"
                .into(),
            vec![Param::Text(name.into())],
        ),
        Engine::SqlServer => {
            let db = quote::ident(engine, database);
            (
                format!(
                    // Catalog text comes in more than one collation; each
                    // piece is pinned to one so they can be joined.
                    "SELECT CONCAT(p.state_desc COLLATE DATABASE_DEFAULT, ' ',
                                   p.permission_name COLLATE DATABASE_DEFAULT,
                                   CASE p.class WHEN 0 THEN ''
                                     WHEN 3 THEN CONCAT(' ON SCHEMA::', QUOTENAME(SCHEMA_NAME(p.major_id)) COLLATE DATABASE_DEFAULT)
                                     ELSE CONCAT(' ON ', QUOTENAME(OBJECT_SCHEMA_NAME(p.major_id, DB_ID({lit}))) COLLATE DATABASE_DEFAULT, '.',
                                                 QUOTENAME(OBJECT_NAME(p.major_id, DB_ID({lit}))) COLLATE DATABASE_DEFAULT) END,
                                   ' TO ', QUOTENAME(u.name) COLLATE DATABASE_DEFAULT, ';')
                     FROM {db}.sys.database_permissions p
                     JOIN {db}.sys.database_principals u ON u.principal_id = p.grantee_principal_id
                     WHERE u.name = @P1 AND p.permission_name <> 'CONNECT'
                     UNION ALL
                     SELECT CONCAT('ALTER ROLE ', QUOTENAME(r.name) COLLATE DATABASE_DEFAULT, ' ADD MEMBER ',
                                   QUOTENAME(u.name) COLLATE DATABASE_DEFAULT, ';')
                     FROM {db}.sys.database_role_members m
                     JOIN {db}.sys.database_principals r ON r.principal_id = m.role_principal_id
                     JOIN {db}.sys.database_principals u ON u.principal_id = m.member_principal_id
                     WHERE u.name = @P1",
                    lit = quote::literal(engine, database)
                ),
                vec![Param::Text(name.into())],
            )
        }
    };
    let result = if params.is_empty() {
        session.execute(&sql, 10_000).await?
    } else {
        session.execute_bound(&sql, &params, 10_000).await?
    };
    // Sorted here: SQL Server can't order text built across catalogs with
    // different collations.
    let mut out: Vec<String> = result.rows.iter().map(|row| text(row, 0)).collect();
    out.sort();
    Ok(out)
}

/// `'name'@'host'`, as MySQL names an account.
fn account(name: &str, host: &str) -> String {
    format!(
        "{}@{}",
        quote::literal(Engine::MySql, name),
        quote::literal(Engine::MySql, if host.is_empty() { "%" } else { host })
    )
}

/// Where privileges apply: a whole database, a schema, or one table.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub database: String,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub table: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UserAction {
    CreateUser {
        name: String,
        #[serde(default)]
        host: String,
        password: String,
        /// SQL Server: the database to make the user in, for the login.
        #[serde(default)]
        database: String,
    },
    DropUser {
        name: String,
        #[serde(default)]
        host: String,
    },
    SetPassword {
        name: String,
        #[serde(default)]
        host: String,
        password: String,
    },
    GrantRole {
        name: String,
        #[serde(default)]
        host: String,
        role: String,
    },
    RevokeRole {
        name: String,
        #[serde(default)]
        host: String,
        role: String,
    },
    Grant {
        name: String,
        #[serde(default)]
        host: String,
        privileges: Vec<String>,
        scope: Scope,
    },
    Revoke {
        name: String,
        #[serde(default)]
        host: String,
        privileges: Vec<String>,
        scope: Scope,
    },
}

/// The privileges offered, which every engine spells the same way.
const PRIVILEGES: &[&str] = &[
    "SELECT",
    "INSERT",
    "UPDATE",
    "DELETE",
    "ALL",
    "ALL PRIVILEGES",
    "EXECUTE",
    "CREATE",
    "ALTER",
    "DROP",
    "REFERENCES",
    "TRIGGER",
    "USAGE",
    "CONNECT",
    "TEMPORARY",
];

fn privileges(list: &[String]) -> Result<String, AppError> {
    if list.is_empty() {
        return Err(AppError::new(
            "validation",
            "Choose at least one privilege.",
        ));
    }
    let mut out = Vec::new();
    for privilege in list {
        let upper = privilege.trim().to_ascii_uppercase();
        if !PRIVILEGES.contains(&upper.as_str()) {
            return Err(AppError::new(
                "validation",
                format!("{privilege} isn't a privilege this app grants."),
            ));
        }
        out.push(upper);
    }
    Ok(out.join(", "))
}

fn on(engine: Engine, scope: &Scope) -> String {
    let schema = scope.schema.as_deref().unwrap_or("");
    match (engine, &scope.table) {
        (Engine::MySql, Some(table)) => quote::table(engine, &scope.database, "", table),
        (Engine::MySql, None) => format!("{}.*", quote::ident(engine, &scope.database)),
        (Engine::Postgres, Some(table)) => {
            format!(
                "TABLE {}",
                quote::table(engine, &scope.database, schema, table)
            )
        }
        (Engine::Postgres, None) if !schema.is_empty() => {
            format!("ALL TABLES IN SCHEMA {}", quote::ident(engine, schema))
        }
        (Engine::Postgres, None) => format!("DATABASE {}", quote::ident(engine, &scope.database)),
        (Engine::SqlServer, Some(table)) => format!(
            "OBJECT::{}.{}",
            quote::ident(engine, if schema.is_empty() { "dbo" } else { schema }),
            quote::ident(engine, table)
        ),
        (Engine::SqlServer, None) if !schema.is_empty() => {
            format!("SCHEMA::{}", quote::ident(engine, schema))
        }
        // The whole database: the grant itself, with no ON.
        (Engine::SqlServer, None) => String::new(),
    }
}

fn who(engine: Engine, name: &str, host: &str) -> String {
    match engine {
        Engine::MySql => account(name, host),
        _ => quote::ident(engine, name),
    }
}

/// The statements for an action, and the same with the password masked,
/// for showing. Runs on a session in the action's database (SQL Server
/// grants and users live in a database).
pub fn user_script(
    engine: Engine,
    action: &UserAction,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let name_check = |name: &str| {
        if name.trim().is_empty() || name.chars().any(char::is_control) {
            Err(AppError::new("validation", "Give the user a name."))
        } else {
            Ok(())
        }
    };
    let secret = |password: &str| -> Result<String, AppError> {
        if password.is_empty() {
            return Err(AppError::new("validation", "Give the user a password."));
        }
        Ok(quote::literal(engine, password))
    };
    let masked = |sql: &str, password: &str| {
        sql.replace(
            &quote::literal(engine, password),
            &quote::literal(engine, "••••••••"),
        )
    };
    let (statements, password): (Vec<String>, Option<&str>) = match action {
        UserAction::CreateUser {
            name,
            host,
            password,
            database,
        } => {
            name_check(name)?;
            let p = secret(password)?;
            (
                match engine {
                    Engine::Postgres => vec![format!(
                        "CREATE ROLE {} WITH LOGIN PASSWORD {p};",
                        quote::ident(engine, name)
                    )],
                    Engine::MySql => vec![format!(
                        "CREATE USER {} IDENTIFIED BY {p};",
                        account(name, host)
                    )],
                    Engine::SqlServer => vec![
                        format!(
                            "CREATE LOGIN {} WITH PASSWORD = {p};",
                            quote::ident(engine, name)
                        ),
                        format!(
                            "EXEC {}.sys.sp_executesql {};",
                            quote::ident(
                                engine,
                                if database.is_empty() {
                                    "master"
                                } else {
                                    database
                                }
                            ),
                            quote::literal(
                                engine,
                                &format!(
                                    "CREATE USER {} FOR LOGIN {}",
                                    quote::ident(engine, name),
                                    quote::ident(engine, name)
                                )
                            )
                        ),
                    ],
                },
                Some(password.as_str()),
            )
        }
        UserAction::DropUser { name, host } => {
            name_check(name)?;
            (
                match engine {
                    // A role that owns objects or holds privileges can't be
                    // dropped: what it owns here goes to you, its privileges
                    // are revoked, then it goes.
                    Engine::Postgres => {
                        let role = quote::ident(engine, name);
                        vec![
                            format!("REASSIGN OWNED BY {role} TO CURRENT_USER;"),
                            format!("DROP OWNED BY {role};"),
                            format!("DROP ROLE {role};"),
                        ]
                    }
                    Engine::MySql => vec![format!("DROP USER {};", account(name, host))],
                    Engine::SqlServer => vec![
                        format!("DROP USER IF EXISTS {};", quote::ident(engine, name)),
                        format!(
                            "IF SUSER_ID({}) IS NOT NULL DROP LOGIN {};",
                            quote::literal(engine, name),
                            quote::ident(engine, name)
                        ),
                    ],
                },
                None,
            )
        }
        UserAction::SetPassword {
            name,
            host,
            password,
        } => {
            name_check(name)?;
            let p = secret(password)?;
            (
                vec![match engine {
                    Engine::Postgres => {
                        format!(
                            "ALTER ROLE {} WITH PASSWORD {p};",
                            quote::ident(engine, name)
                        )
                    }
                    Engine::MySql => {
                        format!("ALTER USER {} IDENTIFIED BY {p};", account(name, host))
                    }
                    Engine::SqlServer => {
                        format!(
                            "ALTER LOGIN {} WITH PASSWORD = {p};",
                            quote::ident(engine, name)
                        )
                    }
                }],
                Some(password.as_str()),
            )
        }
        UserAction::GrantRole { name, host, role }
        | UserAction::RevokeRole { name, host, role } => {
            name_check(name)?;
            name_check(role)?;
            let grant = matches!(action, UserAction::GrantRole { .. });
            let role_name = quote::ident(engine, role);
            let user = who(engine, name, host);
            (
                vec![match (engine, grant) {
                    (Engine::SqlServer, true) => {
                        format!("ALTER ROLE {role_name} ADD MEMBER {user};")
                    }
                    (Engine::SqlServer, false) => {
                        format!("ALTER ROLE {role_name} DROP MEMBER {user};")
                    }
                    (_, true) => format!("GRANT {role_name} TO {user};"),
                    (_, false) => format!("REVOKE {role_name} FROM {user};"),
                }],
                None,
            )
        }
        UserAction::Grant {
            name,
            host,
            privileges: list,
            scope,
        }
        | UserAction::Revoke {
            name,
            host,
            privileges: list,
            scope,
        } => {
            name_check(name)?;
            let grant = matches!(action, UserAction::Grant { .. });
            let what = privileges(list)?;
            let target = on(engine, scope);
            let on_clause = if target.is_empty() {
                String::new()
            } else {
                format!(" ON {target}")
            };
            let user = who(engine, name, host);
            (
                vec![if grant {
                    format!("GRANT {what}{on_clause} TO {user};")
                } else {
                    format!("REVOKE {what}{on_clause} FROM {user};")
                }],
                None,
            )
        }
    };
    let shown = match password {
        Some(password) => statements.iter().map(|s| masked(s, password)).collect(),
        None => statements.clone(),
    };
    Ok((statements, shown))
}

/// The password, scrubbed from an error the server sent back.
pub fn scrub(mut error: AppError, action: &UserAction) -> AppError {
    let password = match action {
        UserAction::CreateUser { password, .. } | UserAction::SetPassword { password, .. } => {
            Some(password)
        }
        _ => None,
    };
    if let Some(password) = password.filter(|p| !p.is_empty()) {
        error.message = error.message.replace(password.as_str(), "••••••••");
        error.detail = error
            .detail
            .map(|d| d.replace(password.as_str(), "••••••••"));
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords_are_masked_in_what_is_shown() {
        let action = UserAction::CreateUser {
            name: "report".into(),
            host: "%".into(),
            password: "it's s3cret".into(),
            database: "shop".into(),
        };
        for engine in [Engine::Postgres, Engine::MySql, Engine::SqlServer] {
            let (run, shown) = user_script(engine, &action).unwrap();
            assert!(
                run.iter().any(|s| s.contains("it''s s3cret")),
                "{engine:?}: {run:?}"
            );
            assert!(
                shown.iter().all(|s| !s.contains("s3cret")),
                "{engine:?}: {shown:?}"
            );
        }
    }

    #[test]
    fn grants_per_engine() {
        let scope = Scope {
            database: "shop".into(),
            schema: Some("sales".into()),
            table: None,
        };
        let action = UserAction::Grant {
            name: "report".into(),
            host: "%".into(),
            privileges: vec!["select".into(), "insert".into()],
            scope,
        };
        assert_eq!(
            user_script(Engine::Postgres, &action).unwrap().0,
            ["GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA \"sales\" TO \"report\";"]
        );
        assert_eq!(
            user_script(Engine::MySql, &action).unwrap().0,
            ["GRANT SELECT, INSERT ON `shop`.* TO 'report'@'%';"]
        );
        assert_eq!(
            user_script(Engine::SqlServer, &action).unwrap().0,
            ["GRANT SELECT, INSERT ON SCHEMA::[sales] TO [report];"]
        );
        let bad = UserAction::Grant {
            name: "x".into(),
            host: String::new(),
            privileges: vec!["SELECT; DROP TABLE t".into()],
            scope: Scope {
                database: "d".into(),
                schema: None,
                table: None,
            },
        };
        assert!(
            user_script(Engine::MySql, &bad).is_err(),
            "only known privileges"
        );
    }

    #[test]
    fn errors_lose_the_password() {
        let action = UserAction::SetPassword {
            name: "u".into(),
            host: String::new(),
            password: "hunter2".into(),
        };
        let error = scrub(
            AppError::new("server", "x").with_detail(Some("near 'hunter2'".into())),
            &action,
        );
        assert!(!error.detail.unwrap().contains("hunter2"));
    }
}
