use crate::use_cases::common::rust_code_generator::GenerationOps;
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
