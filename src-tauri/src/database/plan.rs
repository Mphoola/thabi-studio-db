//! Query plans (EXPLAIN) read into one tree shape for all three engines.
//!
//! PostgreSQL: `EXPLAIN (FORMAT JSON)`, optionally with ANALYZE inside a
//! transaction that is always rolled back, so analysing an UPDATE changes
//! nothing. MySQL: `EXPLAIN FORMAT=JSON`. SQL Server: the estimated plan
//! from `SET SHOWPLAN_XML ON`, which never runs the statement.

use serde::Serialize;

use super::{Cell, Engine, Session};
use crate::errors::AppError;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanNode {
    /// The operation, e.g. "Seq Scan on orders" or "Nested loop".
    pub label: String,
    /// Conditions and other details worth showing under the label.
    pub detail: String,
    /// The engine's own cost estimate for this node and below.
    pub cost: Option<f64>,
    /// Estimated (or, with ANALYZE, actual) rows.
    pub rows: Option<f64>,
    pub children: Vec<PlanNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub nodes: Vec<PlanNode>,
    /// The plan exactly as the server returned it.
    pub raw: String,
    pub analyzed: bool,
}

fn first_text(result: &super::QueryResult) -> Option<String> {
    result.rows.iter().find_map(|row| match row.first() {
        Some(Cell::Value(text)) => Some(text.clone()),
        _ => None,
    })
}

fn no_plan() -> AppError {
    AppError::new(
        "no_plan",
        "The server didn't return a plan for this statement.",
    )
}

/// Explains one statement on `session`. `analyze` (PostgreSQL only) runs
/// it for real timings inside a transaction that is rolled back; the
/// caller gives it a fresh session, so a transaction the user opened in
/// their tab is never ended by that rollback.
pub async fn explain(
    session: &mut Session,
    engine: Engine,
    sql: &str,
    analyze: bool,
) -> Result<Plan, AppError> {
    let sql = sql.trim().trim_end_matches(';');
    match engine {
        Engine::Postgres => {
            let options = if analyze {
                "ANALYZE, BUFFERS, FORMAT JSON"
            } else {
                "FORMAT JSON"
            };
            let statement = format!("EXPLAIN ({options}) {sql}");
            let result = if analyze {
                session.execute("BEGIN", 1).await?;
                let outcome = session.execute(&statement, 1).await;
                // Always undo: ANALYZE really runs the statement.
                session.execute("ROLLBACK", 1).await?;
                outcome?
            } else {
                session.execute(&statement, 1).await?
            };
            let raw = first_text(&result).ok_or_else(no_plan)?;
            Ok(Plan {
                nodes: postgres_nodes(&raw)?,
                raw,
                analyzed: analyze,
            })
        }
        Engine::MySql => {
            let result = session
                .execute(&format!("EXPLAIN FORMAT=JSON {sql}"), 1)
                .await?;
            let raw = first_text(&result).ok_or_else(no_plan)?;
            Ok(Plan {
                nodes: mysql_nodes(&raw)?,
                raw,
                analyzed: false,
            })
        }
        Engine::SqlServer => {
            session.execute("SET SHOWPLAN_XML ON", 1).await?;
            let outcome = session.execute(sql, 1).await;
            session.execute("SET SHOWPLAN_XML OFF", 1).await?;
            let raw = first_text(&outcome?).ok_or_else(no_plan)?;
            Ok(Plan {
                nodes: sqlserver_nodes(&raw),
                raw,
                analyzed: false,
            })
        }
    }
}

fn unreadable() -> AppError {
    AppError::new("no_plan", "The plan couldn't be read.")
}

fn number(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

/// PostgreSQL's JSON plan: `[{"Plan": {...}}]`.
pub fn postgres_nodes(raw: &str) -> Result<Vec<PlanNode>, AppError> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| unreadable())?;
    let plan = value
        .get(0)
        .and_then(|v| v.get("Plan"))
        .ok_or_else(unreadable)?;
    Ok(vec![postgres_node(plan)])
}

fn postgres_node(plan: &serde_json::Value) -> PlanNode {
    let text = |key: &str| plan.get(key).and_then(|v| v.as_str());
    let mut label = text("Node Type").unwrap_or("Step").to_owned();
    if let Some(join) = text("Join Type") {
        label = format!("{join} {label}");
    }
    if let Some(relation) = text("Relation Name") {
        label.push_str(&format!(" on {relation}"));
        if let Some(alias) = text("Alias").filter(|a| *a != relation) {
            label.push_str(&format!(" {alias}"));
        }
    }
    if let Some(index) = text("Index Name") {
        label.push_str(&format!(" using {index}"));
    }
    let detail = [
        "Index Cond",
        "Hash Cond",
        "Merge Cond",
        "Join Filter",
        "Filter",
        "Sort Key",
        "Group Key",
    ]
    .iter()
    .filter_map(|key| {
        plan.get(*key).map(|v| match v {
            serde_json::Value::Array(items) => format!(
                "{key}: {}",
                items
                    .iter()
                    .filter_map(|i| i.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            other => format!("{key}: {}", other.as_str().unwrap_or_default()),
        })
    })
    .collect::<Vec<_>>()
    .join(" · ");
    PlanNode {
        label,
        detail,
        cost: plan.get("Total Cost").and_then(number),
        rows: plan
            .get("Actual Rows")
            .or_else(|| plan.get("Plan Rows"))
            .and_then(number),
        children: plan
            .get("Plans")
            .and_then(|p| p.as_array())
            .map(|plans| plans.iter().map(postgres_node).collect())
            .unwrap_or_default(),
    }
}

/// MySQL's JSON plan: a `query_block` of nested operations and tables.
pub fn mysql_nodes(raw: &str) -> Result<Vec<PlanNode>, AppError> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| unreadable())?;
    let block = value.get("query_block").ok_or_else(unreadable)?;
    let mut root = PlanNode {
        label: "Query".into(),
        cost: block
            .get("cost_info")
            .and_then(|c| c.get("query_cost"))
            .and_then(number),
        ..PlanNode::default()
    };
    root.children = mysql_children(block);
    Ok(vec![root])
}

fn mysql_children(value: &serde_json::Value) -> Vec<PlanNode> {
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    let mut nodes = Vec::new();
    for (key, child) in object {
        let label = match key.as_str() {
            "table" => {
                nodes.push(mysql_table(child));
                continue;
            }
            "nested_loop" => "Nested loop",
            "ordering_operation" => "Sort",
            "grouping_operation" => "Group",
            "duplicates_removal" => "Remove duplicates",
            "union_result" => "Union",
            "windowing" => "Window",
            "query_block" => "Subquery",
            _ => {
                // Arrays and objects under other keys can still hold tables.
                if let Some(items) = child.as_array() {
                    nodes.extend(items.iter().flat_map(mysql_children));
                } else if child.is_object() && key != "cost_info" {
                    nodes.extend(mysql_children(child));
                }
                continue;
            }
        };
        let children = match child.as_array() {
            Some(items) => items.iter().flat_map(mysql_children).collect(),
            None => mysql_children(child),
        };
        nodes.push(PlanNode {
            label: label.into(),
            detail: if child.get("using_filesort").and_then(|v| v.as_bool()) == Some(true) {
                "using filesort".into()
            } else {
                String::new()
            },
            children,
            ..PlanNode::default()
        });
    }
    nodes
}

fn mysql_table(table: &serde_json::Value) -> PlanNode {
    let text = |key: &str| table.get(key).and_then(|v| v.as_str()).unwrap_or_default();
    let access = text("access_type");
    let mut label = format!(
        "{} on {}",
        if access.is_empty() { "Read" } else { access },
        text("table_name")
    );
    if !text("key").is_empty() {
        label.push_str(&format!(" using {}", text("key")));
    }
    PlanNode {
        label,
        detail: text("attached_condition").to_owned(),
        cost: table
            .get("cost_info")
            .and_then(|c| c.get("prefix_cost"))
            .and_then(number),
        rows: table.get("rows_examined_per_scan").and_then(number),
        children: Vec::new(),
    }
}

/// SQL Server's showplan XML: nested `<RelOp>` elements.
pub fn sqlserver_nodes(raw: &str) -> Vec<PlanNode> {
    let mut stack: Vec<PlanNode> = Vec::new();
    let mut roots: Vec<PlanNode> = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find('<') {
        let Some(end) = rest[start..].find('>') else {
            break;
        };
        let tag = &rest[start + 1..start + end];
        rest = &rest[start + end + 1..];
        if let Some(name) = tag.strip_prefix('/') {
            if name.trim() == "RelOp"
                && let Some(done) = stack.pop()
            {
                match stack.last_mut() {
                    Some(parent) => parent.children.push(done),
                    None => roots.push(done),
                }
            }
            continue;
        }
        let name = tag.split_whitespace().next().unwrap_or_default();
        if name == "RelOp" {
            let physical = attribute(tag, "PhysicalOp").unwrap_or_default();
            let logical = attribute(tag, "LogicalOp").unwrap_or_default();
            let node = PlanNode {
                label: physical.clone(),
                detail: if logical != physical {
                    logical
                } else {
                    String::new()
                },
                cost: attribute(tag, "EstimatedTotalSubtreeCost").and_then(|v| v.parse().ok()),
                rows: attribute(tag, "EstimateRows").and_then(|v| v.parse().ok()),
                children: Vec::new(),
            };
            if tag.trim_end().ends_with('/') {
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => roots.push(node),
                }
            } else {
                stack.push(node);
            }
        } else if name == "Object"
            && let Some(current) = stack.last_mut()
            && !current.label.contains(" on ")
            && let Some(table) = attribute(tag, "Table")
        {
            current
                .label
                .push_str(&format!(" on {}", table.trim_matches(['[', ']'])));
            if let Some(index) = attribute(tag, "Index") {
                current
                    .label
                    .push_str(&format!(" using {}", index.trim_matches(['[', ']'])));
            }
        }
    }
    roots
}

/// An attribute's value in an XML start tag, with entities decoded.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(
        tag[start..end]
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_postgres_join_plan() {
        let raw = r#"[{"Plan": {"Node Type": "Hash Join", "Join Type": "Inner", "Total Cost": 42.5, "Plan Rows": 10,
            "Hash Cond": "(o.customer_id = c.id)",
            "Plans": [{"Node Type": "Seq Scan", "Relation Name": "orders", "Alias": "o", "Total Cost": 20.0, "Plan Rows": 100},
                      {"Node Type": "Hash", "Total Cost": 10.0, "Plans": [{"Node Type": "Index Scan", "Relation Name": "customers", "Alias": "c", "Index Name": "customers_pkey", "Total Cost": 8.0}]}]}}]"#;
        let nodes = postgres_nodes(raw).unwrap();
        assert_eq!(nodes[0].label, "Inner Hash Join");
        assert!(
            nodes[0]
                .detail
                .contains("Hash Cond: (o.customer_id = c.id)")
        );
        assert_eq!(nodes[0].cost, Some(42.5));
        assert_eq!(nodes[0].children[0].label, "Seq Scan on orders o");
        assert_eq!(
            nodes[0].children[1].children[0].label,
            "Index Scan on customers c using customers_pkey"
        );
    }

    #[test]
    fn reads_a_mysql_join_plan() {
        let raw = r#"{"query_block": {"select_id": 1, "cost_info": {"query_cost": "3.20"},
            "nested_loop": [
              {"table": {"table_name": "o", "access_type": "ALL", "rows_examined_per_scan": 4, "cost_info": {"prefix_cost": "0.65"}}},
              {"table": {"table_name": "c", "access_type": "eq_ref", "key": "PRIMARY", "rows_examined_per_scan": 1, "cost_info": {"prefix_cost": "2.05"}}}
            ]}}"#;
        let nodes = mysql_nodes(raw).unwrap();
        assert_eq!(nodes[0].cost, Some(3.2));
        let join = &nodes[0].children[0];
        assert_eq!(join.label, "Nested loop");
        assert_eq!(join.children[0].label, "ALL on o");
        assert_eq!(join.children[1].label, "eq_ref on c using PRIMARY");
        assert_eq!(join.children[1].cost, Some(2.05));
    }

    #[test]
    fn reads_a_sqlserver_showplan() {
        let raw = r#"<ShowPlanXML><RelOp NodeId="0" PhysicalOp="Hash Match" LogicalOp="Inner Join" EstimateRows="10" EstimatedTotalSubtreeCost="0.5">
            <Hash><RelOp NodeId="1" PhysicalOp="Clustered Index Scan" LogicalOp="Clustered Index Scan" EstimateRows="100" EstimatedTotalSubtreeCost="0.2">
              <IndexScan><Object Database="[master]" Schema="[dbo]" Table="[orders]" Index="[PK_orders]" /></IndexScan></RelOp>
            <RelOp NodeId="2" PhysicalOp="Table Scan" LogicalOp="Table Scan" EstimateRows="5" EstimatedTotalSubtreeCost="0.1">
              <TableScan><Object Table="[a&amp;b]" /></TableScan></RelOp></Hash></RelOp></ShowPlanXML>"#;
        let nodes = sqlserver_nodes(raw);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].label, "Hash Match");
        assert_eq!(nodes[0].detail, "Inner Join");
        assert_eq!(nodes[0].children.len(), 2);
        assert_eq!(
            nodes[0].children[0].label,
            "Clustered Index Scan on orders using PK_orders"
        );
        assert_eq!(nodes[0].children[1].label, "Table Scan on a&b");
        assert_eq!(nodes[0].children[0].rows, Some(100.0));
    }
}
