use crate::use_cases::common::cpp_qt_code_generator::{GenerationOps, GenerationReadOps};
use crate::use_cases::generate_cpp_qt_code_uc::{
    GenerateCppQtCodeUnitOfWorkFactoryTrait, GenerateCppQtCodeUnitOfWorkTrait,
};
use anyhow::{Ok, Result};
use common::database::QueryUnitOfWork;
use common::database::{db_context::DbContext, transactions::Transaction};
use common::entities::Dto;
use common::entities::DtoField;
use common::entities::Entity;
use common::entities::Feature;
use common::entities::Field;
use common::entities::File;
use common::entities::Global;
use common::entities::Relationship;
use common::entities::Root;
use common::entities::System;
use common::entities::UseCase;
use common::entities::UserInterface;
use common::entities::Workspace;
use common::event::EventHub;
use common::types::EntityId;
use std::cell::RefCell;
use std::sync::Arc;

pub struct GenerateCppQtCodeUnitOfWork {
    context: DbContext,
    transaction: RefCell<Option<Transaction>>,
    #[allow(dead_code)]
    event_hub: Arc<EventHub>,
}

impl GenerateCppQtCodeUnitOfWork {
    pub fn new(db_context: &DbContext, event_hub: &Arc<EventHub>) -> Self {
        GenerateCppQtCodeUnitOfWork {
            context: db_context.clone(),
            transaction: RefCell::new(None),
            event_hub: event_hub.clone(),
        }
    }
}

impl QueryUnitOfWork for GenerateCppQtCodeUnitOfWork {
    fn begin_transaction(&self) -> Result<()> {
        self.transaction
            .replace(Some(Transaction::begin_read_transaction(&self.context)?));
        Ok(())
    }

    fn end_transaction(&self) -> Result<()> {
        self.transaction
            .take()
            .ok_or_else(|| anyhow::anyhow!("No active transaction"))?
            .end_read_transaction()?;
        Ok(())
    }
}

#[macros::uow_action(entity = "Root", action = "GetRelationshipRO")]
#[macros::uow_action(entity = "Root", action = "GetAllRO")]
#[macros::uow_action(entity = "System", action = "GetRO")]
#[macros::uow_action(entity = "Workspace", action = "GetRO")]
#[macros::uow_action(entity = "Workspace", action = "GetRelationshipRO")]
#[macros::uow_action(entity = "UserInterface", action = "GetRO")]
#[macros::uow_action(entity = "File", action = "GetRO")]
#[macros::uow_action(entity = "Global", action = "GetRO")]
#[macros::uow_action(entity = "Feature", action = "GetRO")]
#[macros::uow_action(entity = "Feature", action = "GetMultiRO")]
#[macros::uow_action(entity = "UseCase", action = "GetRO")]
#[macros::uow_action(entity = "UseCase", action = "GetMultiRO")]
#[macros::uow_action(entity = "Dto", action = "GetRO")]
#[macros::uow_action(entity = "DtoField", action = "GetMultiRO")]
#[macros::uow_action(entity = "Entity", action = "GetRO")]
#[macros::uow_action(entity = "Entity", action = "GetMultiRO")]
#[macros::uow_action(entity = "Field", action = "GetMultiRO")]
#[macros::uow_action(entity = "Relationship", action = "GetMultiRO")]
impl GenerationReadOps for GenerateCppQtCodeUnitOfWork {}

impl GenerateCppQtCodeUnitOfWorkTrait for GenerateCppQtCodeUnitOfWork {}

#[macros::uow_action(entity = "Root", action = "GetRelationshipRO")]
#[macros::uow_action(entity = "Root", action = "GetAllRO")]
#[macros::uow_action(entity = "System", action = "GetRO")]
#[macros::uow_action(entity = "Workspace", action = "GetRO")]
#[macros::uow_action(entity = "Workspace", action = "GetRelationshipRO")]
#[macros::uow_action(entity = "UserInterface", action = "GetRO")]
#[macros::uow_action(entity = "File", action = "GetRO")]
#[macros::uow_action(entity = "Global", action = "GetRO")]
#[macros::uow_action(entity = "Feature", action = "GetRO")]
#[macros::uow_action(entity = "Feature", action = "GetMultiRO")]
#[macros::uow_action(entity = "UseCase", action = "GetRO")]
#[macros::uow_action(entity = "UseCase", action = "GetMultiRO")]
#[macros::uow_action(entity = "Dto", action = "GetRO")]
#[macros::uow_action(entity = "DtoField", action = "GetMultiRO")]
#[macros::uow_action(entity = "Entity", action = "GetRO")]
#[macros::uow_action(entity = "Entity", action = "GetMultiRO")]
#[macros::uow_action(entity = "Field", action = "GetMultiRO")]
#[macros::uow_action(entity = "Relationship", action = "GetMultiRO")]
impl GenerationOps for GenerateCppQtCodeUnitOfWork {}

pub struct GenerateCppQtCodeUnitOfWorkFactory {
    context: DbContext,
    event_hub: Arc<EventHub>,
}

impl GenerateCppQtCodeUnitOfWorkFactory {
    pub fn new(db_context: &DbContext, event_hub: &Arc<EventHub>) -> Self {
        GenerateCppQtCodeUnitOfWorkFactory {
            context: db_context.clone(),
            event_hub: event_hub.clone(),
        }
    }
}

impl GenerateCppQtCodeUnitOfWorkFactoryTrait for GenerateCppQtCodeUnitOfWorkFactory {
    fn create(&self) -> Box<dyn GenerateCppQtCodeUnitOfWorkTrait> {
        Box::new(GenerateCppQtCodeUnitOfWork::new(
            &self.context,
            &self.event_hub,
        ))
    }
}
