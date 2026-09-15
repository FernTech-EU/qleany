use crate::use_cases::common::cpp_qt_code_generator::GenerationOps;
use common::entities::Workspace;
use common::types::EntityId;

pub fn get_system_id(uow: &dyn GenerationOps) -> anyhow::Result<EntityId> {
    use anyhow::anyhow;
    let roots = uow.get_all_root()?;
    let root = roots
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Root entity not found"))?;

    let all_system_ids = uow.get_root_relationship(
        &root.id,
        &common::direct_access::root::RootRelationshipField::System,
    )?;

    let system_id = all_system_ids
        .first()
        .cloned()
        .ok_or(anyhow!("No system found"))?;
    Ok(system_id)
}

pub fn get_workspace_id(uow: &dyn GenerationOps) -> anyhow::Result<EntityId> {
    use anyhow::anyhow;
    let roots = uow.get_all_root()?;
    let root = roots
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Root entity not found"))?;

    let all_workspace_ids = uow.get_root_relationship(
        &root.id,
        &common::direct_access::root::RootRelationshipField::Workspace,
    )?;

    let workspace_id = all_workspace_ids
        .first()
        .cloned()
        .ok_or(anyhow!("No workspace found"))?;
    Ok(workspace_id)
}

pub fn get_workspace(uow: &dyn GenerationOps) -> anyhow::Result<Workspace> {
    use anyhow::anyhow;
    let roots = uow.get_all_root()?;
    let root = roots
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Root entity not found"))?;

    let all_workspace_ids = uow.get_root_relationship(
        &root.id,
        &common::direct_access::root::RootRelationshipField::Workspace,
    )?;

    let workspace_id = all_workspace_ids
        .first()
        .cloned()
        .ok_or(anyhow!("No workspace found"))?;

    let workspace = uow
        .get_workspace(&workspace_id)?
        .ok_or_else(|| anyhow!("Workspace entity not found"))?;
    Ok(workspace)
}

pub fn strip_leading_and_trailing_slashes(path: &str) -> String {
    let trimmed = path.trim_matches(|c: char| c == '/' || c == '\\' || c.is_whitespace());
    trimmed.to_string()
}

pub fn to_sql_safe_identifier(name: &str) -> String {
    // Replace spaces and special characters with underscores
    let mut identifier = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>();

    // Ensure it doesn't start with a digit
    if identifier
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
    {
        identifier = format!("_{}", identifier);
    }

    // avoid reserved keywords
    let reserved_keywords = [
        "ABORT",
        "ACTION",
        "ADD",
        "AFTER",
        "ALL",
        "ALTER",
        "ALWAYS",
        "ANALYZE",
        "AND",
        "AS",
        "ASC",
        "ATTACH",
        "AUTOINCREMENT",
        "BEFORE",
        "BEGIN",
        "BETWEEN",
        "BY",
        "CASCADE",
        "CASE",
        "CAST",
        "CHECK",
        "COLLATE",
        "COLUMN",
        "COMMIT",
        "CONFLICT",
        "CONSTRAINT",
        "CREATE",
        "CROSS",
        "CURRENT",
        "CURRENT_DATE",
        "CURRENT_TIME",
        "CURRENT_TIMESTAMP",
        "DATABASE",
        "DEFAULT",
        "DEFERRABLE",
        "DEFERRED",
        "DELETE",
        "DESC",
        "DETACH",
        "DISTINCT",
        "DO",
        "DROP",
        "EACH",
        "ELSE",
        "END",
        "ESCAPE",
        "EXCEPT",
        "EXCLUDE",
        "EXCLUSIVE",
        "EXISTS",
        "EXPLAIN",
        "FAIL",
        "FILTER",
        "FIRST",
        "FOLLOWING",
        "FOR",
        "FOREIGN",
        "FROM",
        "FULL",
        "GENERATED",
        "GLOB",
        "GROUP",
        "GROUPS",
        "HAVING",
        "IF",
        "IGNORE",
        "IMMEDIATE",
        "IN",
        "INDEX",
        "INDEXED",
        "INITIALLY",
        "INNER",
        "INSERT",
        "INSTEAD",
        "INTERSECT",
        "INTO",
        "IS",
        "ISNULL",
        "JOIN",
        "KEY",
        "LAST",
        "LEFT",
        "LIKE",
        "LIMIT",
        "MATCH",
        "MATERIALIZED",
        "NATURAL",
        "NO",
        "NOT",
        "NOTHING",
        "NOTNULL",
        "NULL",
        "NULLS",
        "OF",
        "OFFSET",
        "ON",
        "OR",
        "ORDER",
        "OTHERS",
        "OUTER",
        "OVER",
        "PARTITION",
        "PLAN",
        "PRAGMA",
        "PRECEDING",
        "PRIMARY",
        "QUERY",
        "RAISE",
        "RANGE",
        "RECURSIVE",
        "REFERENCES",
        "REGEXP",
        "REINDEX",
        "RELEASE",
        "RENAME",
        "REPLACE",
        "RESTRICT",
        "RETURNING",
        "RIGHT",
        "ROLLBACK",
        "ROW",
        "ROWS",
        "SAVEPOINT",
        "SELECT",
        "SET",
        "TABLE",
        "TEMP",
        "TEMPORARY",
        "THEN",
        "TIES",
        "TO",
        "TRANSACTION",
        "TRIGGER",
        "UNBOUNDED",
        "UNION",
        "UNIQUE",
        "UPDATE",
        "USING",
        "VACUUM",
        "VALUES",
        "VIEW",
        "VIRTUAL",
        "WHEN",
        "WHERE",
        "WINDOW",
        "WITH",
        "WITHOUT",
    ];

    if reserved_keywords.contains(&identifier.to_uppercase().as_str()) {
        identifier = format!("_{}", identifier);
    }
    identifier
}
