//! One DTO pane: the DTO itself, its fields, and the selected field's form.
//!
//! Constructed **twice**, once for the input DTO and once for the output one. That
//! is why it owns its handles rather than taking them from the generated `Session`:
//! the session mints one `SingleDto`, one `DtoFieldsListModel` and one
//! `SingleDtoField`, and `wire_all` wires those. A second pane taking the same three
//! would show the same DTO in both halves of the screen; a second pane taking fresh
//! ones that nobody wired would be permanently deaf and merely look empty. Each pane
//! therefore builds its own and wires them from its own `wire`.

use std::rc::Rc;

use teksilo::data::{KeyedSelectionModel, SelectionMode};
use teksilo::prelude::*;
use teksilo::text_document::TextDocument;
use teksilo::widgets::ValidationState;

use frontend::AppContext;
use frontend::EntityId;
use frontend::commands::{dto_commands, undo_redo_commands, use_case_commands};
use frontend::common::direct_access::use_case::UseCaseRelationshipField;
use frontend::common::entities::DtoFieldType;
use frontend::direct_access::{CreateDtoDto, CreateDtoFieldDto, UseCaseRelationshipDto};

use crate::app_ids::AppIds;
use crate::edit::{UndoAction, labeled};
use crate::models::{DtoFieldsListModel, DtoFieldsRow};
use crate::shared::settle::Settle;
use crate::shared::validation::{is_pascal_case, is_snake_case, required_cased};
use crate::singles::{SingleDto, SingleDtoField};

/// The name a new DTO field is given.
const NEW_FIELD_NAME: &str = "new_field";

/// The eight types a DTO field may have.
///
/// No `Entity`: a DTO crosses a boundary, and carrying an entity reference across
/// one is what the DTO exists to avoid. The enum the backend declares has exactly
/// these eight, so this list is the whole of it rather than a filtered view.
pub const DTO_FIELD_TYPES: [DtoFieldType; 8] = [
    DtoFieldType::Boolean,
    DtoFieldType::Integer,
    DtoFieldType::UInteger,
    DtoFieldType::Float,
    DtoFieldType::String,
    DtoFieldType::Uuid,
    DtoFieldType::DateTime,
    DtoFieldType::Enum,
];

/// What the manifest calls a DTO field type. The file format, not display text.
pub fn dto_field_type_name(field_type: &DtoFieldType) -> &'static str {
    match field_type {
        DtoFieldType::Boolean => "Boolean",
        DtoFieldType::Integer => "Integer",
        DtoFieldType::UInteger => "UInteger",
        DtoFieldType::Float => "Float",
        DtoFieldType::String => "String",
        DtoFieldType::Uuid => "Uuid",
        DtoFieldType::DateTime => "DateTime",
        DtoFieldType::Enum => "Enum",
    }
}

/// Which of a use case's two DTOs a pane edits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DtoSide {
    In,
    Out,
}

impl DtoSide {
    pub fn relationship(self) -> UseCaseRelationshipField {
        match self {
            DtoSide::In => UseCaseRelationshipField::DtoIn,
            DtoSide::Out => UseCaseRelationshipField::DtoOut,
        }
    }

    pub fn heading(self) -> LocalizedString {
        match self {
            DtoSide::In => tr!(dto_in_heading()),
            DtoSide::Out => tr!(dto_out_heading()),
        }
    }

    pub fn enable_label(self) -> LocalizedString {
        match self {
            DtoSide::In => tr!(dto_in_enable()),
            DtoSide::Out => tr!(dto_out_enable()),
        }
    }

    pub fn disabled_hint(self) -> LocalizedString {
        match self {
            DtoSide::In => tr!(dto_in_disabled_hint()),
            DtoSide::Out => tr!(dto_out_disabled_hint()),
        }
    }

    /// What a DTO created for `use_case` is called.
    ///
    /// The generator derives type names from these, so the convention is part of the
    /// output rather than a display choice: `LoadDto` takes the input of `load`, and
    /// `LoadReturnDto` gives back its result.
    pub fn default_name(self, use_case: &str) -> String {
        match self {
            DtoSide::In => format!("{}Dto", heck::AsPascalCase(use_case)),
            DtoSide::Out => format!("{}ReturnDto", heck::AsPascalCase(use_case)),
        }
    }
}

/// What a DTO field row says under its name.
pub fn dto_field_subtitle(row: &DtoFieldsRow) -> String {
    let optional = if row.optional { " (opt)" } else { "" };
    let list = if row.is_list { " (list)" } else { "" };
    match row.field_type {
        DtoFieldType::Enum => format!(
            "{}: {}{}{}",
            dto_field_type_name(&row.field_type),
            row.enum_name.as_deref().unwrap_or("?"),
            optional,
            list
        ),
        _ => format!(
            "{}{}{}",
            dto_field_type_name(&row.field_type),
            optional,
            list
        ),
    }
}

#[derive(Clone)]
pub struct DtoViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    side: DtoSide,
    /// The use case this pane belongs to.
    use_case: Signal<Option<EntityId>>,
    /// The use case's name, for the DTO's default name.
    use_case_name: Signal<String>,
    /// The DTO itself. Its own handle, not the session's: see the module note.
    dto: SingleDto,
    /// Which DTO the pane is showing, `None` when the use case has none.
    dto_id: Signal<Option<EntityId>>,
    /// What the enable checkbox shows.
    ///
    /// Its own writable signal rather than a `map` of `dto_id`, for a reason the
    /// compiler does not catch: a `Checkbox` **writes** the signal it is given, and
    /// a derived signal panics when written. Bound to a `map`, the box would take
    /// the app down on the first click. It is kept in step with `dto_id` by
    /// `refresh_dto_id`, and the confirmation puts it back when the user declines.
    enabled: Signal<bool>,
    fields: DtoFieldsListModel,
    field: SingleDtoField,
    selection: KeyedSelectionModel<EntityId>,
    enum_document: TextDocument,
    enum_settle: Settle,
}

impl std::fmt::Debug for DtoViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DtoViewModel")
            .field("side", &self.side)
            .field("dto", &self.dto_id.get())
            .finish_non_exhaustive()
    }
}

impl DtoViewModel {
    pub fn new(
        app_ctx: Rc<AppContext>,
        ids: AppIds,
        side: DtoSide,
        use_case: Signal<Option<EntityId>>,
        use_case_name: Signal<String>,
    ) -> Self {
        let dto_id: Signal<Option<EntityId>> = Signal::new(None);
        Self {
            dto: SingleDto::new(app_ctx.clone()),
            fields: DtoFieldsListModel::new(app_ctx.clone(), dto_id.clone()),
            field: SingleDtoField::new(app_ctx.clone()),
            app_ctx,
            ids,
            side,
            use_case,
            use_case_name,
            dto_id,
            enabled: Signal::new(false),
            selection: KeyedSelectionModel::new(SelectionMode::Single),
            enum_document: TextDocument::new(),
            enum_settle: Settle::new(),
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn side(&self) -> DtoSide {
        self.side
    }

    /// Whether this use case has this DTO.
    pub fn enabled(&self) -> Signal<bool> {
        self.enabled.clone()
    }

    pub fn dto_id(&self) -> Option<EntityId> {
        self.dto_id.get()
    }

    pub fn name(&self) -> Signal<String> {
        self.dto.name()
    }

    pub fn name_validation(&self) -> Signal<ValidationState> {
        required_cased(
            &self.dto.name(),
            tr!(dto_name_required()),
            tr!(dto_name_pascal_case()),
            is_pascal_case,
        )
    }

    pub fn fields(&self) -> DtoFieldsListModel {
        self.fields.clone()
    }

    pub fn selection(&self) -> KeyedSelectionModel<EntityId> {
        self.selection.clone()
    }

    pub fn selected_field(&self) -> Option<EntityId> {
        self.selection.selected_keys().first().copied()
    }

    pub fn field_name(&self) -> Signal<String> {
        self.field.name()
    }

    pub fn field_name_validation(&self) -> Signal<ValidationState> {
        required_cased(
            &self.field.name(),
            tr!(dto_field_name_required()),
            tr!(dto_field_name_snake_case()),
            is_snake_case,
        )
    }

    pub fn field_type(&self) -> Signal<DtoFieldType> {
        self.field.field_type()
    }

    pub fn field_optional(&self) -> Signal<bool> {
        self.field.optional()
    }

    pub fn field_is_list(&self) -> Signal<bool> {
        self.field.is_list()
    }

    pub fn field_enum_name(&self) -> Signal<String> {
        self.field
            .enum_name()
            .map(|v| v.clone().unwrap_or_default())
    }

    pub fn field_enum_name_validation(&self) -> Signal<ValidationState> {
        required_cased(
            &self.field_enum_name(),
            tr!(dto_field_enum_name_required()),
            tr!(dto_field_enum_name_pascal_case()),
            is_pascal_case,
        )
    }

    /// Whether the enum rows are shown.
    pub fn shows_enum(&self) -> bool {
        self.field.field_type().get() == DtoFieldType::Enum
    }

    pub fn enum_document(&self) -> TextDocument {
        self.enum_document.clone()
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Create the DTO and link it, as one undoable step.
    ///
    /// Two writes, and the Slint UI did them unbracketed with a hand-rolled cleanup
    /// on failure, so one Ctrl+Z after a successful enable left an orphan DTO in the
    /// store that nothing referenced. A composite takes both back together, and a
    /// failed link cancels it rather than unpicking it.
    pub fn enable(&self) {
        let Some(use_case) = self.use_case.get() else {
            return;
        };
        if self.dto_id.get().is_some() {
            return;
        }

        if let Err(e) = undo_redo_commands::begin_composite_labeled(
            &self.app_ctx,
            self.stack(),
            Some(UndoAction::EnableDto.label()),
        ) {
            log::error!("could not start a composite edit: {e}");
            return;
        }

        let name = self.side.default_name(&self.use_case_name.get());
        let created = match dto_commands::create_orphan_dto(
            &self.app_ctx,
            self.stack(),
            &CreateDtoDto {
                name,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                ..Default::default()
            },
        ) {
            Ok(dto) => dto,
            Err(e) => {
                log::error!("could not create the {:?} DTO: {e}", self.side);
                undo_redo_commands::cancel_composite(&self.app_ctx);
                self.refresh_dto_id();
                return;
            }
        };

        if let Err(e) = self.link(use_case, vec![created.id]) {
            log::error!("could not link the {:?} DTO: {e}", self.side);
            // The composite is cancelled rather than ended, which takes the orphan
            // DTO back with it: there is nothing left to clean up by hand.
            undo_redo_commands::cancel_composite(&self.app_ctx);
            self.refresh_dto_id();
            return;
        }

        undo_redo_commands::end_composite(&self.app_ctx);
        self.refresh_dto_id();
    }

    /// Put the checkbox back in step with the store.
    ///
    /// The box writes itself on a click, so anything that refuses or fails has to
    /// re-read rather than assume.
    pub fn resync_enabled(&self) {
        self.refresh_dto_id();
    }

    /// Unlink the DTO and delete it, as one undoable step.
    ///
    /// The caller confirms first: this destroys the DTO's fields with it, and a
    /// stray click on a checkbox is not consent for that.
    pub fn disable(&self) {
        let (Some(use_case), Some(dto)) = (self.use_case.get(), self.dto_id.get()) else {
            return;
        };

        if let Err(e) = undo_redo_commands::begin_composite_labeled(
            &self.app_ctx,
            self.stack(),
            Some(UndoAction::DisableDto.label()),
        ) {
            log::error!("could not start a composite edit: {e}");
            return;
        }
        if let Err(e) = self.link(use_case, Vec::new()) {
            log::error!("could not unlink the {:?} DTO: {e}", self.side);
            undo_redo_commands::cancel_composite(&self.app_ctx);
            self.refresh_dto_id();
            return;
        }
        if let Err(e) = dto_commands::remove_dto(&self.app_ctx, self.stack(), &dto) {
            log::error!("could not delete the {:?} DTO: {e}", self.side);
            undo_redo_commands::cancel_composite(&self.app_ctx);
            self.refresh_dto_id();
            return;
        }
        undo_redo_commands::end_composite(&self.app_ctx);
        self.selection.clear();
        self.refresh_dto_id();
    }

    /// How many fields the DTO has, for the confirmation that precedes `disable`.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    pub fn commit_name(&self) {
        let stack = self.stack();
        labeled(&self.app_ctx, stack, UndoAction::EditDto, || {
            self.dto.save(stack)
        });
    }

    pub fn add_field(&self) {
        let dto = CreateDtoFieldDto {
            name: NEW_FIELD_NAME.to_string(),
            field_type: DtoFieldType::String,
            ..Default::default()
        };
        let stack = self.stack();
        let created = labeled(&self.app_ctx, stack, UndoAction::AddDtoField, || {
            self.fields.create(&dto, -1, stack)
        });
        if let Some(id) = created {
            self.selection.select(id);
        }
    }

    pub fn remove_field(&self, id: EntityId) {
        let stack = self.stack();
        labeled(&self.app_ctx, stack, UndoAction::RemoveDtoField, || {
            self.fields.remove(id, stack)
        });
        if self.selected_field() == Some(id) {
            self.selection.clear();
        }
    }

    pub fn select_field(&self, id: EntityId) {
        self.selection.select(id);
    }

    pub fn commit_field(&self) {
        let stack = self.stack();
        labeled(&self.app_ctx, stack, UndoAction::EditDtoField, || {
            self.field.save(stack)
        });
        self.fields.refresh();
    }

    /// Change the field's type, clearing the enum when leaving it.
    pub fn set_field_type(&self, field_type: DtoFieldType) {
        if self.field.field_type().get() == field_type {
            return;
        }
        let leaving_enum = field_type != DtoFieldType::Enum;
        self.field.set_field_type(field_type);
        if leaving_enum {
            self.field.set_enum_name(None);
            self.field.set_enum_values(Vec::new());
        }
        self.commit_field();
    }

    /// Optional and list are exclusive, in both directions: `Option<Vec<T>>` says
    /// nothing `Vec<T>` does not, and the generator has no spelling for it.
    pub fn set_field_optional(&self, value: bool) {
        self.field.set_optional(value);
        if value {
            self.field.set_is_list(false);
        }
        self.commit_field();
    }

    pub fn set_field_is_list(&self, value: bool) {
        self.field.set_is_list(value);
        if value {
            self.field.set_optional(false);
        }
        self.commit_field();
    }

    pub fn set_field_enum_name(&self, value: &str) {
        self.field
            .set_enum_name((!value.trim().is_empty()).then(|| value.to_string()));
        self.commit_field();
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    pub fn wire(&self, ctx: &mut BuildContext) {
        // This pane's own handles. The session wired its one of each; these are the
        // second set, and nothing else will wire them.
        self.dto.wire(ctx);
        self.fields.wire(ctx);
        self.field.wire(ctx);

        let me = self.clone();
        let use_case = self.use_case.clone();
        ctx.effect(&use_case, move |_| {
            me.selection.clear();
            me.refresh_dto_id();
        });
        self.refresh_dto_id();

        let me = self.clone();
        let dto_id = self.dto_id.clone();
        ctx.effect(&dto_id, move |id| {
            if me.dto.id() != *id {
                me.dto.set_id(*id);
            }
        });
        if self.dto.id() != self.dto_id.get() {
            self.dto.set_id(self.dto_id.get());
        }

        let me = self.clone();
        let selection = self.selection.selection_signal();
        ctx.effect(&selection, move |_| me.point_at_field());
        self.point_at_field();

        let me = self.clone();
        let version = self.fields.version_signal();
        ctx.effect(&version, move |_| me.prune_selection());

        // The enum editor, exactly as the entity field form does it: a document
        // revision and no blur, so the commit waits for typing to stop.
        let wake = ctx.wake_at_handle();
        let tick = ctx.frame_tick();
        let last_revision = Rc::new(std::cell::Cell::new(self.enum_document.content_revision()));
        let me = self.clone();
        ctx.effect(&tick, move |_| me.poll_enum_editor(&last_revision, &wake));

        let me = self.clone();
        let values = self.field.enum_values();
        ctx.effect(&values, move |_| me.seed_enum_document());
        self.seed_enum_document();
    }

    fn link(&self, use_case: EntityId, dtos: Vec<EntityId>) -> anyhow::Result<()> {
        use_case_commands::set_use_case_relationship(
            &self.app_ctx,
            self.stack(),
            &UseCaseRelationshipDto {
                id: use_case,
                field: self.side.relationship(),
                right_ids: dtos,
            },
        )
    }

    /// Which DTO, if any, the use case has on this side.
    fn refresh_dto_id(&self) {
        let Some(use_case) = self.use_case.get() else {
            self.dto_id.set_if_changed(None);
            self.enabled.set_if_changed(false);
            return;
        };
        match use_case_commands::get_use_case_relationship(
            &self.app_ctx,
            &use_case,
            &self.side.relationship(),
        ) {
            Ok(ids) => {
                let dto = ids.first().copied();
                self.dto_id.set_if_changed(dto);
                self.enabled.set_if_changed(dto.is_some());
            }
            Err(e) => log::error!("could not read the {:?} DTO of {use_case}: {e}", self.side),
        }
    }

    fn point_at_field(&self) {
        let id = self.selected_field();
        if self.field.id() == id {
            return;
        }
        self.enum_settle.forget();
        self.field.set_id(id);
        self.seed_enum_document();
    }

    fn prune_selection(&self) {
        let live: Vec<EntityId> = self.fields.rows().iter().map(|r| r.id).collect();
        self.selection.prune_missing(|id| live.contains(id));
    }

    fn poll_enum_editor(
        &self,
        last_revision: &std::cell::Cell<u64>,
        wake: &Rc<std::cell::Cell<Option<std::time::Instant>>>,
    ) {
        let revision = self.enum_document.content_revision();
        if revision != last_revision.get() {
            last_revision.set(revision);
            self.enum_settle.touch();
            wake.set(Some(std::time::Instant::now()));
        }
        if self.enum_settle.tick() {
            self.commit_enum_values();
        }
    }

    fn commit_enum_values(&self) {
        let Ok(text) = self.enum_document.to_plain_text() else {
            log::error!("could not read the enum editor");
            return;
        };
        let values = crate::entities::field_vm::split_enum_values(&text);
        if values == self.field.enum_values().get() {
            return;
        }
        self.field.set_enum_values(values);
        self.commit_field();
    }

    fn seed_enum_document(&self) {
        let wanted = self.field.enum_values().get();
        if let Ok(current) = self.enum_document.to_plain_text()
            && crate::entities::field_vm::split_enum_values(&current) == wanted
        {
            return;
        }
        let joined = crate::entities::field_vm::join_enum_values(&wanted);
        if let Err(e) = self.enum_document.set_plain_text(&joined) {
            log::error!("could not fill the enum editor: {e}");
        }
    }

    /// Move a row, as one named undo entry.
    ///
    /// Here rather than in the page: the reorder adapter outlives the frame that
    /// built it, and what it needs is a command, not a model and a stack signal it
    /// would have to name the operation from itself.
    pub fn reorder_field(&self, id: EntityId, index: i32) {
        let stack = self.stack();
        labeled(&self.app_ctx, stack, UndoAction::ReorderDtoFields, || {
            self.fields.move_to(id, index, stack)
        });
    }

    fn stack(&self) -> Option<u64> {
        self.ids.features_stack.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// US-DTO-01: the names the generator will turn into type names.
    #[test]
    fn a_dto_is_named_after_its_use_case() {
        assert_eq!(DtoSide::In.default_name("load"), "LoadDto");
        assert_eq!(DtoSide::Out.default_name("load"), "LoadReturnDto");
        assert_eq!(
            DtoSide::In.default_name("fill_rust_files"),
            "FillRustFilesDto"
        );
        assert_eq!(
            DtoSide::Out.default_name("fill_rust_files"),
            "FillRustFilesReturnDto"
        );
    }

    /// US-DTO-07: a DTO carries primitives, and there is no Entity among them.
    #[test]
    fn a_dto_field_cannot_be_an_entity() {
        assert_eq!(DTO_FIELD_TYPES.len(), 8);
        let names: Vec<&str> = DTO_FIELD_TYPES.iter().map(dto_field_type_name).collect();
        assert_eq!(
            names,
            vec![
                "Boolean", "Integer", "UInteger", "Float", "String", "Uuid", "DateTime", "Enum"
            ]
        );
        assert!(!names.contains(&"Entity"));
    }

    /// The two sides write to different relationships, which is what keeps one pane
    /// from overwriting the other.
    #[test]
    fn the_two_sides_are_different_relationships() {
        assert_eq!(DtoSide::In.relationship(), UseCaseRelationshipField::DtoIn);
        assert_eq!(
            DtoSide::Out.relationship(),
            UseCaseRelationshipField::DtoOut
        );
    }

    /// US-DTO-05: the subtitle shapes.
    #[test]
    fn a_dto_field_subtitle_names_its_type() {
        let row = |f: fn(&mut DtoFieldsRow)| {
            let mut row = DtoFieldsRow {
                id: 1,
                name: "value".to_string(),
                field_type: DtoFieldType::String,
                ..Default::default()
            };
            f(&mut row);
            row
        };
        assert_eq!(dto_field_subtitle(&row(|_| {})), "String");
        assert_eq!(
            dto_field_subtitle(&row(|r| r.optional = true)),
            "String (opt)"
        );
        assert_eq!(
            dto_field_subtitle(&row(|r| r.is_list = true)),
            "String (list)"
        );
        assert_eq!(
            dto_field_subtitle(&row(|r| {
                r.field_type = DtoFieldType::Enum;
                r.enum_name = Some("Status".to_string());
            })),
            "Enum: Status"
        );
    }
}
