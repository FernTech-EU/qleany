//! The field list and the field form, and the rules that keep a field coherent.
//!
//! A field's settings constrain each other: an Entity field has a relationship and
//! no list flag, a to-many relationship cannot be optional, a many-to-many cannot
//! cascade. The Slint UI expressed those rules as fifteen callbacks that each
//! reached into the widget tree and wrote both the entity and the widget state, so
//! the rules could only be exercised by clicking. They live here as a pure function
//! over a plain struct instead, which is what makes every one of them testable.

use std::cell::Cell;
use std::rc::Rc;

use teksilo::data::{KeyedSelectionModel, SelectionMode};
use teksilo::prelude::*;
use teksilo::text_document::TextDocument;
use teksilo::widgets::ValidationState;

use frontend::AppContext;
use frontend::EntityId;
use frontend::commands::{field_commands, undo_redo_commands};
use frontend::common::direct_access::field::FieldRelationshipField;
use frontend::common::entities::{FieldRelationshipType, FieldType};
use frontend::direct_access::{CreateFieldDto, FieldRelationshipDto};

use crate::app_ids::AppIds;
use crate::models::{EntityFieldsListModel, EntityFieldsRow};
use crate::shared::settle::Settle;
use crate::shared::validation::{is_snake_case, required_cased};
use crate::singles::SingleField;

/// The name a new field is given: valid snake_case, so the form it opens is not
/// already showing an error the user did not cause.
const NEW_FIELD_NAME: &str = "new_field";

/// What `list_model_displayed_field` is seeded with when the flag goes on.
///
/// Every entity in practice has a `name`, and the alternative, an empty required
/// field, is a validation error the moment the box is ticked.
const DEFAULT_DISPLAYED_FIELD: &str = "name";

/// The nine field types, in the order the combo offers them.
pub const FIELD_TYPES: [FieldType; 9] = [
    FieldType::Boolean,
    FieldType::Integer,
    FieldType::UInteger,
    FieldType::Float,
    FieldType::String,
    FieldType::Uuid,
    FieldType::DateTime,
    FieldType::Entity,
    FieldType::Enum,
];

/// The relationships an Entity field may have.
///
/// `many_to_one` is absent on purpose, matching the Slint UI: it is the *backward*
/// half of a one-to-many, which the generator derives rather than the user
/// declaring. The rules below still handle it, because a manifest written by hand
/// can carry it.
pub const RELATIONSHIP_TYPES: [FieldRelationshipType; 4] = [
    FieldRelationshipType::OneToOne,
    FieldRelationshipType::OneToMany,
    FieldRelationshipType::OrderedOneToMany,
    FieldRelationshipType::ManyToMany,
];

/// What the manifest calls a field type. These strings are the file format, not
/// display text, which is why they are not translated.
pub fn field_type_name(field_type: &FieldType) -> &'static str {
    match field_type {
        FieldType::Boolean => "Boolean",
        FieldType::Integer => "Integer",
        FieldType::UInteger => "UInteger",
        FieldType::Float => "Float",
        FieldType::String => "String",
        FieldType::Uuid => "Uuid",
        FieldType::DateTime => "DateTime",
        FieldType::Entity => "Entity",
        FieldType::Enum => "Enum",
    }
}

/// What the manifest calls a relationship. See [`field_type_name`].
pub fn relationship_name(relationship: &FieldRelationshipType) -> &'static str {
    match relationship {
        FieldRelationshipType::OneToOne => "one_to_one",
        FieldRelationshipType::OneToMany => "one_to_many",
        FieldRelationshipType::OrderedOneToMany => "ordered_one_to_many",
        FieldRelationshipType::ManyToOne => "many_to_one",
        FieldRelationshipType::ManyToMany => "many_to_many",
    }
}

/// Everything about a field that the rules act on.
///
/// A plain value, deliberately: the rules are about how these settings constrain
/// each other, and expressing them over signals would mean the only way to check
/// one is to build a widget tree and click.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldShape {
    pub field_type: FieldType,
    pub entity: Option<EntityId>,
    pub relationship: FieldRelationshipType,
    pub optional: bool,
    pub is_list: bool,
    pub strong: bool,
    pub list_model: bool,
    pub list_model_displayed_field: Option<String>,
    pub enum_name: Option<String>,
    pub enum_values: Vec<String>,
}

impl Default for FieldShape {
    fn default() -> Self {
        Self {
            field_type: FieldType::String,
            entity: None,
            relationship: FieldRelationshipType::OneToOne,
            optional: false,
            is_list: false,
            strong: false,
            list_model: false,
            list_model_displayed_field: None,
            enum_name: None,
            enum_values: Vec::new(),
        }
    }
}

impl FieldShape {
    /// Whether this relationship points at many rows.
    fn to_many(&self) -> bool {
        matches!(
            self.relationship,
            FieldRelationshipType::OneToMany
                | FieldRelationshipType::OrderedOneToMany
                | FieldRelationshipType::ManyToMany
        )
    }

    // ── the rules ────────────────────────────────────────────────────────────

    /// Change the type, dropping whatever the old type carried.
    ///
    /// Leaving the old settings in place is what produces a manifest with a String
    /// field that has a relationship and a referenced entity: harmless in the YAML,
    /// and a generator error later that names a file nobody edited.
    pub fn with_type(mut self, field_type: FieldType) -> Self {
        self.field_type = field_type;
        if self.field_type != FieldType::Entity {
            self.entity = None;
            self.relationship = FieldRelationshipType::OneToOne;
            self.strong = false;
            self.list_model = false;
            self.list_model_displayed_field = None;
        }
        if self.field_type != FieldType::Enum {
            self.enum_name = None;
            self.enum_values = Vec::new();
        }
        self
    }

    /// Change the relationship, and with it everything the new one rules out.
    pub fn with_relationship(mut self, relationship: FieldRelationshipType) -> Self {
        self.relationship = relationship;
        // Nothing cascades across a many-to-many or up a many-to-one: the row on the
        // other side has other owners, or is the owner.
        if matches!(
            self.relationship,
            FieldRelationshipType::ManyToOne | FieldRelationshipType::ManyToMany
        ) {
            self.strong = false;
        }
        // "Optional" means the reference may be absent. A to-many is a list, and an
        // empty list already says that.
        if self.to_many() {
            self.optional = false;
        }
        // A list model is a list. A to-one has nothing to list.
        if matches!(
            self.relationship,
            FieldRelationshipType::OneToOne | FieldRelationshipType::ManyToOne
        ) {
            self.list_model = false;
            self.list_model_displayed_field = None;
        }
        self
    }

    /// Optional and list are exclusive: `Option<Vec<T>>` says nothing `Vec<T>` does
    /// not already say, and the generator has no spelling for it.
    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        if optional {
            self.is_list = false;
        }
        self
    }

    /// The other half of [`FieldShape::with_optional`].
    pub fn with_is_list(mut self, is_list: bool) -> Self {
        self.is_list = is_list;
        if is_list {
            self.optional = false;
        }
        self
    }

    /// Turning on the list model seeds the displayed field, because an empty
    /// required field is not a useful thing to hand someone who just ticked a box.
    pub fn with_list_model(mut self, list_model: bool) -> Self {
        self.list_model = list_model;
        self.list_model_displayed_field = list_model.then(|| DEFAULT_DISPLAYED_FIELD.to_string());
        self
    }

    pub fn with_strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }

    pub fn with_entity(mut self, entity: Option<EntityId>) -> Self {
        self.entity = entity;
        self
    }

    pub fn with_enum_name(mut self, name: Option<String>) -> Self {
        self.enum_name = name;
        self
    }

    pub fn with_enum_values(mut self, values: Vec<String>) -> Self {
        self.enum_values = values;
        self
    }

    // ── what the form shows ──────────────────────────────────────────────────

    pub fn shows_referenced_entity(&self) -> bool {
        self.field_type == FieldType::Entity
    }

    pub fn shows_relationship(&self) -> bool {
        self.field_type == FieldType::Entity
    }

    pub fn shows_enum(&self) -> bool {
        self.field_type == FieldType::Enum
    }

    /// Everything but an Entity field may be optional; an Entity field may only be
    /// optional when it points at one row.
    pub fn shows_optional(&self) -> bool {
        self.field_type != FieldType::Entity
            || matches!(
                self.relationship,
                FieldRelationshipType::OneToOne | FieldRelationshipType::ManyToOne
            )
    }

    /// An Entity field's plurality comes from its relationship, and an enum's
    /// variants are not a list of values of the field.
    pub fn shows_is_list(&self) -> bool {
        !matches!(self.field_type, FieldType::Entity | FieldType::Enum)
    }

    pub fn shows_list_model(&self) -> bool {
        self.field_type == FieldType::Entity && self.to_many()
    }

    pub fn shows_displayed_field(&self) -> bool {
        self.shows_list_model() && self.list_model
    }

    pub fn shows_strong(&self) -> bool {
        self.field_type == FieldType::Entity
            && !matches!(
                self.relationship,
                FieldRelationshipType::ManyToMany | FieldRelationshipType::ManyToOne
            )
    }
}

/// What a field row says under its name.
///
/// Built from the manifest's own vocabulary rather than translated: these are the
/// words that will appear in the YAML, and a user matching the list against the file
/// needs to see the same ones.
pub fn field_subtitle(row: &EntityFieldsRow) -> String {
    let optional = if row.optional { " (opt)" } else { "" };
    let list = if row.is_list { " (list)" } else { "" };
    match row.field_type {
        FieldType::Entity => format!(
            "{} ({}){}",
            field_type_name(&row.field_type),
            relationship_name(&row.relationship),
            optional
        ),
        FieldType::Enum => format!(
            "{}: {}{}{}",
            field_type_name(&row.field_type),
            row.enum_name.as_deref().unwrap_or("?"),
            optional,
            list
        ),
        _ => format!("{}{}{}", field_type_name(&row.field_type), optional, list),
    }
}

/// Split the enum editor's text into variants.
///
/// On newlines only, never on commas or spaces: a variant may carry a whole struct
/// body, and `Image { name: String, width: i64 }` is one variant with a comma in it.
pub fn split_enum_values(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Put them back for the editor.
pub fn join_enum_values(values: &[String]) -> String {
    values.join("\n")
}

#[derive(Clone)]
pub struct FieldViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    list: EntityFieldsListModel,
    single: SingleField,
    selection: KeyedSelectionModel<EntityId>,
    /// Which entity's fields are listed. Written by the Entities view-model's
    /// selection, read by the list model as its owner.
    owner: Signal<Option<EntityId>>,
    /// The enum editor's document.
    ///
    /// Owned here rather than built by the view, because the view is rebuilt on
    /// every change of shape and a document built there would lose the caret, the
    /// selection and the editor's own undo history each time.
    enum_document: TextDocument,
    /// When the enum editor's typing has stopped. It reports a document version and
    /// no blur, so this is what turns a burst of keystrokes into one write.
    enum_settle: Settle,
}

impl std::fmt::Debug for FieldViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldViewModel")
            .field("rows", &self.list.len())
            .field("selected", &self.selected_id())
            .finish_non_exhaustive()
    }
}

impl FieldViewModel {
    pub fn new(
        app_ctx: Rc<AppContext>,
        ids: AppIds,
        list: EntityFieldsListModel,
        single: SingleField,
        owner: Signal<Option<EntityId>>,
    ) -> Self {
        Self {
            app_ctx,
            ids,
            list,
            single,
            selection: KeyedSelectionModel::new(SelectionMode::Single),
            owner,
            enum_document: TextDocument::new(),
            enum_settle: Settle::new(),
        }
    }

    /// The enum editor's document, seeded with the selected field's variants.
    pub fn enum_document(&self) -> TextDocument {
        self.enum_document.clone()
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn list(&self) -> EntityFieldsListModel {
        self.list.clone()
    }

    pub fn selection(&self) -> KeyedSelectionModel<EntityId> {
        self.selection.clone()
    }

    pub fn selected_id(&self) -> Option<EntityId> {
        self.selection.selected_keys().first().copied()
    }

    pub fn has_selection(&self) -> Signal<bool> {
        self.selection.selection_signal().map(|k| !k.is_empty())
    }

    pub fn name(&self) -> Signal<String> {
        self.single.name()
    }

    pub fn name_validation(&self) -> Signal<ValidationState> {
        required_cased(
            &self.single.name(),
            tr!(fields_name_required()),
            tr!(fields_name_snake_case()),
            is_snake_case,
        )
    }

    pub fn field_type(&self) -> Signal<FieldType> {
        self.single.field_type()
    }

    pub fn relationship(&self) -> Signal<FieldRelationshipType> {
        self.single.relationship()
    }

    pub fn optional(&self) -> Signal<bool> {
        self.single.optional()
    }

    pub fn is_list(&self) -> Signal<bool> {
        self.single.is_list()
    }

    pub fn strong(&self) -> Signal<bool> {
        self.single.strong()
    }

    pub fn list_model(&self) -> Signal<bool> {
        self.single.list_model()
    }

    pub fn displayed_field(&self) -> Signal<String> {
        self.single
            .list_model_displayed_field()
            .map(|v| v.clone().unwrap_or_default())
    }

    pub fn enum_name(&self) -> Signal<String> {
        self.single
            .enum_name()
            .map(|v| v.clone().unwrap_or_default())
    }

    pub fn enum_values_text(&self) -> Signal<String> {
        self.single.enum_values().map(|v| join_enum_values(v))
    }

    pub fn enum_name_validation(&self) -> Signal<ValidationState> {
        required_cased(
            &self.enum_name(),
            tr!(fields_enum_name_required()),
            tr!(fields_enum_name_pascal_case()),
            crate::shared::validation::is_pascal_case,
        )
    }

    /// The selected field's settings, as the rules see them.
    pub fn shape(&self) -> FieldShape {
        FieldShape {
            field_type: self.single.field_type().get(),
            entity: self.single.dto().and_then(|dto| dto.entity),
            relationship: self.single.relationship().get(),
            optional: self.single.optional().get(),
            is_list: self.single.is_list().get(),
            strong: self.single.strong().get(),
            list_model: self.single.list_model().get(),
            list_model_displayed_field: self.single.list_model_displayed_field().get(),
            enum_name: self.single.enum_name().get(),
            enum_values: self.single.enum_values().get(),
        }
    }

    // ── commands ─────────────────────────────────────────────────────────────

    pub fn add(&self) {
        let dto = CreateFieldDto {
            name: NEW_FIELD_NAME.to_string(),
            field_type: FieldType::String,
            ..Default::default()
        };
        if let Some(id) = self.list.create(&dto, -1, self.stack()) {
            self.select(id);
        }
    }

    pub fn remove(&self, id: EntityId) {
        self.list.remove(id, self.stack());
        if self.selected_id() == Some(id) {
            self.selection.clear();
        }
    }

    pub fn select(&self, id: EntityId) {
        self.selection.select(id);
    }

    /// Write the name back, on Enter or on blur.
    pub fn commit(&self) {
        self.single.save(self.stack());
    }

    pub fn set_displayed_field(&self, value: &str) {
        self.single
            .set_list_model_displayed_field((!value.trim().is_empty()).then(|| value.to_string()));
    }

    pub fn set_enum_name(&self, value: &str) {
        self.single
            .set_enum_name((!value.trim().is_empty()).then(|| value.to_string()));
    }

    /// Run one of the rules and write the result.
    ///
    /// Every setting on the form goes through here, so a change can never write the
    /// field it touched without also writing whatever that change ruled out.
    pub fn apply(&self, rule: impl FnOnce(FieldShape) -> FieldShape) {
        let before = self.shape();
        let after = rule(before.clone());
        if after == before {
            return;
        }
        let Some(id) = self.selected_id() else {
            return;
        };

        // The referenced entity is a relationship, and the generated `save` writes
        // scalars only, so a change that moves both is two backend calls. Bracketed
        // so one Ctrl+Z takes back the whole rule rather than half of it, which
        // would leave exactly the incoherent field the rule exists to prevent.
        let composite = after.entity != before.entity && scalars_differ(&before, &after);
        if composite
            && let Err(e) = undo_redo_commands::begin_composite(&self.app_ctx, self.stack())
        {
            log::error!("could not start a composite edit: {e}");
            return;
        }

        self.single.set_field_type(after.field_type);
        self.single.set_relationship(after.relationship);
        self.single.set_optional(after.optional);
        self.single.set_is_list(after.is_list);
        self.single.set_strong(after.strong);
        self.single.set_list_model(after.list_model);
        self.single
            .set_list_model_displayed_field(after.list_model_displayed_field.clone());
        self.single.set_enum_name(after.enum_name.clone());
        self.single.set_enum_values(after.enum_values.clone());
        self.commit();

        if after.entity != before.entity {
            let dto = FieldRelationshipDto {
                id,
                field: FieldRelationshipField::Entity,
                right_ids: after.entity.into_iter().collect(),
            };
            if let Err(e) =
                field_commands::set_field_relationship(&self.app_ctx, self.stack(), &dto)
            {
                log::error!("could not set the referenced entity of field {id}: {e}");
            } else {
                self.single.refresh();
            }
        }

        if composite {
            undo_redo_commands::end_composite(&self.app_ctx);
        }
        self.list.refresh();
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    pub fn wire(&self, ctx: &mut BuildContext) {
        let list = self.list.clone();
        let owner = self.owner.clone();
        ctx.effect(&owner, move |id| list.set_owner_id(*id));
        self.list.set_owner_id(owner.get());

        // The enum editor reports a document revision and no blur, so its commit is
        // driven from the frame loop: mark each revision, and write on the first
        // frame that brings none. `wake_at` as well as the tick, because teksilo
        // pumps a frame only when something asks for one and a burst that ended
        // would otherwise wait for whatever woke the window next.
        let wake = ctx.wake_at_handle();
        let tick = ctx.frame_tick();
        let last_revision = Rc::new(Cell::new(self.enum_document.content_revision()));
        let me = self.clone();
        ctx.effect(&tick, move |_| me.poll_enum_editor(&last_revision, &wake));

        // And the other direction: a field that is selected, reloaded or undone puts
        // its variants into the editor.
        let me = self.clone();
        let values = self.single.enum_values();
        ctx.effect(&values, move |_| me.seed_enum_document());
        self.seed_enum_document();

        // A different entity means a different set of fields, and the one that was
        // selected belongs to the entity being left.
        let selection = self.selection.clone();
        let owner_changed = self.owner.clone();
        ctx.effect(&owner_changed, move |_| selection.clear());

        let me = self.clone();
        let selection = self.selection.selection_signal();
        ctx.effect(&selection, move |_| me.point_at_selection());
        self.point_at_selection();

        let me = self.clone();
        let version = self.list.version_signal();
        ctx.effect(&version, move |_| me.prune_selection());
    }

    fn point_at_selection(&self) {
        let id = self.selected_id();
        if self.single.id() == id {
            return;
        }
        // Anything half-typed in the enum editor belonged to the field being left.
        // Writing it now would write it to the wrong field; `set_id` is about to
        // reload the editor anyway.
        self.enum_settle.forget();
        self.single.set_id(id);
        self.seed_enum_document();
    }

    /// One frame of the enum editor's commit.
    fn poll_enum_editor(
        &self,
        last_revision: &Cell<u64>,
        wake: &Rc<Cell<Option<std::time::Instant>>>,
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

    /// Read the editor and write its variants, as one undo entry per burst.
    fn commit_enum_values(&self) {
        let Ok(text) = self.enum_document.to_plain_text() else {
            log::error!("could not read the enum editor");
            return;
        };
        let values = split_enum_values(&text);
        if values == self.single.enum_values().get() {
            return;
        }
        self.single.set_enum_values(values);
        self.commit();
        self.list.refresh();
    }

    /// Put the selected field's variants into the editor.
    ///
    /// Compared after splitting, not as raw text: a commit rewrites the signal with
    /// the same variants the document already holds, and re-seeding from that would
    /// throw away the caret and any trailing newline the user had just typed.
    fn seed_enum_document(&self) {
        let wanted = self.single.enum_values().get();
        if let Ok(current) = self.enum_document.to_plain_text()
            && split_enum_values(&current) == wanted
        {
            return;
        }
        if let Err(e) = self
            .enum_document
            .set_plain_text(&join_enum_values(&wanted))
        {
            log::error!("could not fill the enum editor: {e}");
        }
    }

    fn prune_selection(&self) {
        let live: Vec<EntityId> = self.list.rows().iter().map(|r| r.id).collect();
        self.selection.prune_missing(|id| live.contains(id));
    }

    fn stack(&self) -> Option<u64> {
        self.ids.entities_stack.get()
    }

    /// The screen's undo stack, for anything that has to read it later rather than
    /// now. A reorder adapter outlives the frame that built it, so it captures this
    /// signal rather than the value.
    pub fn stack_signal(&self) -> Signal<Option<u64>> {
        self.ids.entities_stack.clone()
    }
}

/// Whether anything the scalar write covers actually changed.
///
/// A change that only moves the referenced entity needs no scalar write, and
/// therefore no composite: one backend call is already atomic.
fn scalars_differ(before: &FieldShape, after: &FieldShape) -> bool {
    before.field_type != after.field_type
        || before.relationship != after.relationship
        || before.optional != after.optional
        || before.is_list != after.is_list
        || before.strong != after.strong
        || before.list_model != after.list_model
        || before.list_model_displayed_field != after.list_model_displayed_field
        || before.enum_name != after.enum_name
        || before.enum_values != after.enum_values
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fully populated Entity field, so every rule below has something to clear.
    fn entity_field() -> FieldShape {
        FieldShape {
            field_type: FieldType::Entity,
            entity: Some(7),
            relationship: FieldRelationshipType::OrderedOneToMany,
            optional: false,
            is_list: false,
            strong: true,
            list_model: true,
            list_model_displayed_field: Some("name".to_string()),
            enum_name: None,
            enum_values: Vec::new(),
        }
    }

    /// US-FLD-09: leaving Entity clears everything only an Entity field has.
    #[test]
    fn leaving_the_entity_type_clears_what_only_an_entity_field_has() {
        let after = entity_field().with_type(FieldType::String);
        assert_eq!(after.entity, None);
        assert_eq!(after.relationship, FieldRelationshipType::OneToOne);
        assert!(!after.strong);
        assert!(!after.list_model);
        assert_eq!(after.list_model_displayed_field, None);
    }

    /// And leaving Enum clears the enum.
    #[test]
    fn leaving_the_enum_type_clears_the_enum() {
        let shape = FieldShape {
            field_type: FieldType::Enum,
            enum_name: Some("Status".to_string()),
            enum_values: vec!["Draft".to_string(), "Published".to_string()],
            ..FieldShape::default()
        };
        let after = shape.with_type(FieldType::String);
        assert_eq!(after.enum_name, None);
        assert!(after.enum_values.is_empty());
    }

    /// Staying on Entity keeps what it carries: the rule fires on a change of type,
    /// not on every write.
    #[test]
    fn staying_on_the_entity_type_keeps_its_settings() {
        let before = entity_field();
        let after = before.clone().with_type(FieldType::Entity);
        assert_eq!(after, before);
    }

    /// US-FLD-06: nothing cascades across a many-to-many.
    #[test]
    fn a_many_to_many_cannot_cascade() {
        let after = entity_field().with_relationship(FieldRelationshipType::ManyToMany);
        assert!(!after.strong);
        let after = entity_field().with_relationship(FieldRelationshipType::ManyToOne);
        assert!(!after.strong);
    }

    /// A to-many is a list, and an empty list already means "absent".
    #[test]
    fn a_to_many_relationship_is_never_optional() {
        for relationship in [
            FieldRelationshipType::OneToMany,
            FieldRelationshipType::OrderedOneToMany,
            FieldRelationshipType::ManyToMany,
        ] {
            let before = FieldShape {
                field_type: FieldType::Entity,
                optional: true,
                ..FieldShape::default()
            };
            assert!(
                !before.with_relationship(relationship.clone()).optional,
                "{relationship:?}"
            );
        }
    }

    /// US-FLD-07: a to-one has nothing to list.
    #[test]
    fn a_to_one_relationship_drops_the_list_model() {
        for relationship in [
            FieldRelationshipType::OneToOne,
            FieldRelationshipType::ManyToOne,
        ] {
            let after = entity_field().with_relationship(relationship.clone());
            assert!(!after.list_model, "{relationship:?}");
            assert_eq!(after.list_model_displayed_field, None, "{relationship:?}");
        }
    }

    /// US-FLD-09: optional and list are exclusive, in both directions.
    #[test]
    fn optional_and_list_cannot_both_be_on() {
        let listed = FieldShape {
            is_list: true,
            ..FieldShape::default()
        };
        assert!(!listed.clone().with_optional(true).is_list);

        let optional = FieldShape {
            optional: true,
            ..FieldShape::default()
        };
        assert!(!optional.with_is_list(true).optional);

        // Turning either one *off* leaves the other alone.
        assert!(listed.with_optional(false).is_list);
    }

    /// US-FLD-07: ticking the list model seeds the displayed field, unticking clears
    /// it, so the manifest never carries a displayed field for a field that has no
    /// list model.
    #[test]
    fn the_list_model_flag_carries_its_displayed_field_with_it() {
        let on = entity_field().with_list_model(true);
        assert_eq!(on.list_model_displayed_field.as_deref(), Some("name"));

        let off = entity_field().with_list_model(false);
        assert_eq!(off.list_model_displayed_field, None);
    }

    /// The form's visibility rules, which the Slint UI spelled out inline in the
    /// markup and could therefore only be checked by looking at the window.
    #[test]
    fn the_form_shows_what_the_type_makes_meaningful() {
        let entity = entity_field();
        assert!(entity.shows_referenced_entity());
        assert!(entity.shows_relationship());
        assert!(entity.shows_list_model());
        assert!(entity.shows_displayed_field());
        assert!(
            !entity.shows_is_list(),
            "plurality comes from the relationship"
        );
        assert!(!entity.shows_optional(), "a to-many is never optional");
        assert!(entity.shows_strong());

        let plain = FieldShape::default();
        assert!(!plain.shows_referenced_entity());
        assert!(!plain.shows_relationship());
        assert!(!plain.shows_enum());
        assert!(plain.shows_is_list());
        assert!(plain.shows_optional());
        assert!(!plain.shows_strong());

        let one_to_one = entity_field().with_relationship(FieldRelationshipType::OneToOne);
        assert!(
            one_to_one.shows_optional(),
            "a to-one reference may be absent"
        );
        assert!(!one_to_one.shows_list_model());

        let many_to_many = entity_field().with_relationship(FieldRelationshipType::ManyToMany);
        assert!(!many_to_many.shows_strong());

        let enumerated = FieldShape::default().with_type(FieldType::Enum);
        assert!(enumerated.shows_enum());
        assert!(!enumerated.shows_is_list());
    }

    /// US-FLD-08: newlines only. A variant may carry a struct body with commas in
    /// it, and splitting on those would turn one variant into three.
    #[test]
    fn enum_values_split_on_newlines_only() {
        let text = "Draft\nImage { name: String, width: i64 }\n\n  Published  \n";
        assert_eq!(
            split_enum_values(text),
            vec![
                "Draft".to_string(),
                "Image { name: String, width: i64 }".to_string(),
                "Published".to_string(),
            ]
        );
    }

    #[test]
    fn enum_values_round_trip_through_the_editor() {
        let values = vec![
            "Draft".to_string(),
            "Image { name: String, width: i64 }".to_string(),
        ];
        assert_eq!(split_enum_values(&join_enum_values(&values)), values);
    }

    /// US-FLD-01: the three subtitle shapes.
    #[test]
    fn a_row_subtitle_names_the_type_and_what_qualifies_it() {
        let row = |f: fn(&mut EntityFieldsRow)| {
            let mut row = EntityFieldsRow {
                id: 1,
                name: "thing".to_string(),
                field_type: FieldType::String,
                relationship: FieldRelationshipType::OneToOne,
                ..Default::default()
            };
            f(&mut row);
            row
        };

        assert_eq!(field_subtitle(&row(|_| {})), "String");
        assert_eq!(field_subtitle(&row(|r| r.optional = true)), "String (opt)");
        assert_eq!(field_subtitle(&row(|r| r.is_list = true)), "String (list)");
        assert_eq!(
            field_subtitle(&row(|r| {
                r.field_type = FieldType::Entity;
                r.relationship = FieldRelationshipType::OneToMany;
            })),
            "Entity (one_to_many)"
        );
        assert_eq!(
            field_subtitle(&row(|r| {
                r.field_type = FieldType::Enum;
                r.enum_name = Some("Status".to_string());
                r.is_list = true;
            })),
            "Enum: Status (list)"
        );
        // An enum with no name yet still renders, rather than showing an empty gap.
        assert_eq!(
            field_subtitle(&row(|r| r.field_type = FieldType::Enum)),
            "Enum: ?"
        );
    }

    /// The names that end up in the YAML. A rename here is a manifest format change,
    /// so it should fail a test rather than quietly write a file nothing can read.
    #[test]
    fn the_manifest_vocabulary_is_pinned() {
        assert_eq!(field_type_name(&FieldType::UInteger), "UInteger");
        assert_eq!(field_type_name(&FieldType::DateTime), "DateTime");
        assert_eq!(
            relationship_name(&FieldRelationshipType::OrderedOneToMany),
            "ordered_one_to_many"
        );
        assert_eq!(
            relationship_name(&FieldRelationshipType::ManyToOne),
            "many_to_one"
        );
    }

    /// Only a relationship change needs no composite: one backend call is already
    /// atomic, and wrapping it would push an empty composite onto the stack.
    #[test]
    fn a_change_that_moves_only_the_reference_needs_no_composite() {
        let before = entity_field();
        let after = before.clone().with_entity(Some(9));
        assert!(!scalars_differ(&before, &after));

        let both = before.clone().with_type(FieldType::String);
        assert!(scalars_differ(&before, &both));
        assert_ne!(both.entity, before.entity);
    }
}
