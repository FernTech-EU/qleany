use crate::use_cases::common::rust_code_generator::{GenerationOps, GenerationReadOps};
use crate::use_cases::generate_rust_files_uc::{
    GenerateRustFilesUnitOfWorkFactoryTrait, GenerateRustFilesUnitOfWorkTrait,
};
use anyhow::{Ok, Result};
use common::database::QueryUnitOfWork;
use common::database::{db_context::DbContext, transactions::Transaction};
use common::entities::UserInterface;
use common::entities::Workspace;
use common::entities::{
    Dto, DtoField, Entity, Feature, Field, File, Global, Relationship, Root, System, UseCase,
};
use common::event::EventHub;
use common::types::EntityId;
use std::sync::Arc;
use std::sync::Mutex;

pub struct GenerateRustFilesUnitOfWork {
    context: DbContext,
    #[allow(dead_code)]
    event_hub: Arc<EventHub>,
    transaction: Mutex<Option<Transaction>>,
}

impl GenerateRustFilesUnitOfWork {
    pub fn new(db_context: &DbContext, event_hub: &Arc<EventHub>) -> Self {
        GenerateRustFilesUnitOfWork {
            context: db_context.clone(),
            event_hub: event_hub.clone(),
            transaction: Mutex::new(None),
        }
    }
}

impl QueryUnitOfWork for GenerateRustFilesUnitOfWork {
    fn begin_transaction(&self) -> Result<()> {
        let mut transaction = self.transaction.lock().unwrap();
        *transaction = Some(Transaction::begin_read_transaction(&self.context)?);
        Ok(())
    }

    fn end_transaction(&self) -> Result<()> {
        let mut transaction = self.transaction.lock().unwrap();
        transaction
            .take()
            .ok_or_else(|| anyhow::anyhow!("No active transaction"))?
            .end_read_transaction()?;
        Ok(())
    }
}

#[macros::uow_action(entity = "Root", action = "GetRelationshipRO", thread_safe = true)]
#[macros::uow_action(entity = "Root", action = "GetAllRO", thread_safe = true)]
#[macros::uow_action(entity = "System", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Workspace", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Workspace", action = "GetRelationshipRO", thread_safe = true)]
#[macros::uow_action(entity = "File", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Global", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "UserInterface", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Feature", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Feature", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "UseCase", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "UseCase", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Dto", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "DtoField", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Entity", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Entity", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Field", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Relationship", action = "GetMultiRO", thread_safe = true)]
impl GenerationReadOps for GenerateRustFilesUnitOfWork {}

#[macros::uow_action(entity = "Root", action = "GetRelationshipRO", thread_safe = true)]
#[macros::uow_action(entity = "Root", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Global", action = "GetMultiRO", thread_safe = true)]
impl GenerateRustFilesUnitOfWorkTrait for GenerateRustFilesUnitOfWork {}

#[macros::uow_action(entity = "Root", action = "GetRelationshipRO", thread_safe = true)]
#[macros::uow_action(entity = "Root", action = "GetAllRO", thread_safe = true)]
#[macros::uow_action(entity = "System", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Workspace", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Workspace", action = "GetRelationshipRO", thread_safe = true)]
#[macros::uow_action(entity = "File", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Global", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "UserInterface", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Feature", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Feature", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "UseCase", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "UseCase", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Dto", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "DtoField", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Entity", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Entity", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Field", action = "GetMultiRO", thread_safe = true)]
#[macros::uow_action(entity = "Relationship", action = "GetMultiRO", thread_safe = true)]
impl GenerationOps for GenerateRustFilesUnitOfWork {}

pub struct GenerateRustFilesUnitOfWorkFactory {
    context: DbContext,
    event_hub: Arc<EventHub>,
}

impl GenerateRustFilesUnitOfWorkFactory {
    pub fn new(db_context: &DbContext, event_hub: &Arc<EventHub>) -> Self {
        GenerateRustFilesUnitOfWorkFactory {
            context: db_context.clone(),
            event_hub: event_hub.clone(),
        }
    }
}

impl GenerateRustFilesUnitOfWorkFactoryTrait for GenerateRustFilesUnitOfWorkFactory {
    fn create(&self) -> Box<dyn GenerateRustFilesUnitOfWorkTrait> {
        Box::new(GenerateRustFilesUnitOfWork::new(
            &self.context,
            &self.event_hub,
        ))
    }
}
