use crate::use_cases::common::cpp_qt_code_generator::{GenerationOps, GenerationReadOps};
use crate::use_cases::generate_cpp_qt_files_uc::{
    GenerateCppQtFilesUnitOfWorkFactoryTrait, GenerateCppQtFilesUnitOfWorkTrait,
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

pub struct GenerateCppQtFilesUnitOfWork {
    context: DbContext,
    #[allow(dead_code)]
    event_hub: Arc<EventHub>,
    transaction: Mutex<Option<Transaction>>,
}

impl GenerateCppQtFilesUnitOfWork {
    pub fn new(db_context: &DbContext, event_hub: &Arc<EventHub>) -> Self {
        GenerateCppQtFilesUnitOfWork {
            context: db_context.clone(),
            event_hub: event_hub.clone(),
            transaction: Mutex::new(None),
        }
    }
}

impl QueryUnitOfWork for GenerateCppQtFilesUnitOfWork {
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
#[macros::uow_action(entity = "UserInterface", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "File", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Global", action = "GetRO", thread_safe = true)]
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
impl GenerationReadOps for GenerateCppQtFilesUnitOfWork {}

#[macros::uow_action(
    entity = "Root",
    action = "GetRelationshipRO",
    thread_safe = true,
    thread_safe = true
)]
#[macros::uow_action(
    entity = "Root",
    action = "GetMultiRO",
    thread_safe = true,
    thread_safe = true
)]
#[macros::uow_action(
    entity = "Global",
    action = "GetMultiRO",
    thread_safe = true,
    thread_safe = true
)]
impl GenerateCppQtFilesUnitOfWorkTrait for GenerateCppQtFilesUnitOfWork {}
#[macros::uow_action(entity = "Root", action = "GetRelationshipRO", thread_safe = true)]
#[macros::uow_action(entity = "Root", action = "GetAllRO", thread_safe = true)]
#[macros::uow_action(entity = "System", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Workspace", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Workspace", action = "GetRelationshipRO", thread_safe = true)]
#[macros::uow_action(entity = "UserInterface", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "File", action = "GetRO", thread_safe = true)]
#[macros::uow_action(entity = "Global", action = "GetRO", thread_safe = true)]
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
impl GenerationOps for GenerateCppQtFilesUnitOfWork {}

pub struct GenerateCppQtFilesUnitOfWorkFactory {
    context: DbContext,
    event_hub: Arc<EventHub>,
}

impl GenerateCppQtFilesUnitOfWorkFactory {
    pub fn new(db_context: &DbContext, event_hub: &Arc<EventHub>) -> Self {
        GenerateCppQtFilesUnitOfWorkFactory {
            context: db_context.clone(),
            event_hub: event_hub.clone(),
        }
    }
}

impl GenerateCppQtFilesUnitOfWorkFactoryTrait for GenerateCppQtFilesUnitOfWorkFactory {
    fn create(&self) -> Box<dyn GenerateCppQtFilesUnitOfWorkTrait> {
        Box::new(GenerateCppQtFilesUnitOfWork::new(
            &self.context,
            &self.event_hub,
        ))
    }
}
