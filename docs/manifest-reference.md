# Manifest Reference

Everything in Qleany is defined in `qleany.yaml`. This document covers all manifest options. The UI is still the primary
way to create and edit manifests, but this reference helps when you need to edit the file directly. Or if you are curious.

## Example Manifests

Real-world manifests you can reference:

| Project | Language | Frontend | Link |
|---------|----------|----------|------|
| Skribisto | C++20 / Qt6 | QtQuick | [qleany.yaml](https://github.com/jacquetc/skribisto/blob/master/qleany.yaml) |
| Qleany | Rust 2024 | Slint + CLI | [qleany.yaml](https://github.com/ferntech-eu/qleany/blob/main/qleany.yaml) |

## Basic Structure

```yaml
schema:
  version: 5

global:
  language: cpp-qt          # rust, cpp-qt
  application_name: MyApp
  organisation:
    name: myorg
    domain: myorg.com
  prefix_path: src

entities:
  - name: EntityBase
    # ...

features:
  - name: my_feature
    # ...
```

## Required Base Entity

> All entities must have `id`, `created_at`, and `updated_at` fields. These are essential for identity, caching, and change tracking.

To simplify this, Qleany provides `EntityBase` — a heritable entity with these three fields pre-defined. When you create a new manifest, Qleany automatically generates:
- `EntityBase` with `id`, `created_at`, `updated_at`
- An empty `Root` entity inheriting from `EntityBase`

All your entities should inherit from `EntityBase` using the `inherits_from` field.

```yaml
entities:
  # EntityBase provides the necessary id, created_at, updated_at
  - name: EntityBase
    only_for_heritage: true
    fields:
      - name: id
        type: uinteger
      - name: created_at
        type: datetime
      - name: updated_at
        type: datetime
        
  - name: Root
    inherits_from: EntityBase
    undoable: false
    fields:
      # Your root-level relationships here
```

## Complete Example

```yaml
schema:
  version: 5

global:
  language: cpp-qt
  application_name: MyApp
  organisation:
    name: myorg
    domain: myorg.com
  prefix_path: src

entities:
  - name: EntityBase
    only_for_heritage: true
    fields:
      - name: id
        type: uinteger
      - name: created_at
        type: datetime
      - name: updated_at
        type: datetime

  - name: Root
    inherits_from: EntityBase
    undoable: false
    fields:
      - name: works
        type: entity
        entity: Work
        relationship: ordered_one_to_many
        strong: true
        list_model: true
        list_model_displayed_field: title

  - name: Work
    inherits_from: EntityBase
    undoable: true
    single_model: true
    fields:
      - name: title
        type: string
      - name: binders
        type: entity
        entity: Binder
        relationship: ordered_one_to_many
        strong: true
        list_model: true
        list_model_displayed_field: name
      - name: tags
        type: entity
        entity: BinderTag
        relationship: one_to_many
        strong: true

  - name: Binder
    inherits_from: EntityBase
    undoable: true
    fields:
      - name: name
        type: string
      - name: items
        type: entity
        entity: BinderItem
        relationship: ordered_one_to_many
        strong: true
        list_model: true
        list_model_displayed_field: title

  - name: BinderItem
    inherits_from: EntityBase
    undoable: true
    fields:
      - name: title
        type: string
      - name: parentItem
        type: entity
        entity: BinderItem
        relationship: many_to_one
      - name: tags
        type: entity
        entity: BinderTag
        relationship: many_to_many

features:
  - name: work_management
    use_cases:
      - name: load_work
        undoable: false
        entities: [Root, Work, Binder, BinderItem]
        dto_in:
          name: LoadWorkDto
          fields:
            - name: file_path
              type: string
        dto_out:
          name: LoadWorkReturnDto
          fields:
            - name: work_id
              type: integer
```

---

## Entity Options

| Option                | Type   | Default  | Description                                         |
|-----------------------|--------|----------|-----------------------------------------------------|
| `name`                | string | required | Entity name (PascalCase)                            |
| `inherits_from`       | string | none     | Parent entity for inheritance                       |
| `only_for_heritage`   | bool   | false    | Entity used only as base class                      |
| `undoable`            | bool   | false    | Enable undo/redo for this entity's controller       |
| `single_model`        | bool   | false    | Generate `Single{Entity}` QML wrapper (C++/Qt only) |

---
## Field options

| Option                       | Type   | Default  | Description                                                                                        |
|------------------------------|--------|----------|----------------------------------------------------------------------------------------------------|
| `name`                       | string | required | Field name (snake_case)                                                                            |
| `type`                       | string | required | Field type (see below)                                                                             |
| `entity`                     | string | none     | For `entity` type, name of the entity                                                              |
| `relationship`               | string | none     | For `entity` type, relationship type (see below)                                                   |
| `optional`                   | bool   | false    | For `one_to_one` and `many_to_one`                                                                 |
| `is_list`                    | bool   | false    | Field is a list/array. Cannot be used with `entity` or `enum` types, and cannot be combined with `optional` |
| `strong`                     | bool   | false    | For `one_to_one`, `one_to_many`, and `ordered_one_to_many`, enable cascade deletion                |
| `list_model`                 | bool   | false    | For C++/Qt only, generate a C++ QAbstractListModel and its QML wrapper for this relationship field |
| `list_model_displayed_field` | string | none     | For C++/Qt only, default display role for the generated ListModel                                  |
| `enum_name`                  | string | none     | For `enum` type, name of the enum (PascalCase)                                                     |
| `enum_values`                | array  | none     | For `enum` type, list of enum values (see Enum Fields section for complex variant syntax)           |


---

## Field Types

| Type       | Description                    | Example                             |
|------------|--------------------------------|-------------------------------------|
| `boolean`  | True/false value               | `is_active: true`                   |
| `integer`  | Whole number                   | `count: 42`                         |
| `float`    | Decimal number                 | `price: 19.99`                      |
| `string`   | Text                           | `name: "Alice"`                     |
| `uuid`     | Unique identifier              | `id: "550e8400-..."`                |
| `datetime` | Date and time                  | `created_at: "2024-01-15T10:30:00"` |
| `entity`   | Relationship to another entity | See relationship section            |
| `enum`     | Enumerated value               | See enum section                    |




### Enum Fields

```yaml
- name: status
  type: enum
  enum_name: CarStatus
  enum_values:
    - Available
    - Reserved
    - Sold
```

Like entities, the enum name should be PascalCase. Enum variant names should also be PascalCase. And the name must be unique.

#### Complex Enum Variants (Rust Only)

Rust's algebraic enums are too good to ignore. This is an escape hatch for when simple flat enums are not enough, typically when you would otherwise need multiple entities or DTOs to model the same concept. Prefer simple enums when they do the job.

Enum values can carry data using tuple or struct syntax. You write actual Rust types directly (`i64`, `String`, `bool`, etc.).

```yaml
- name: content
  type: enum
  enum_name: ContentBlock
  enum_values:
    - Empty
    - "Text(String)"
    - "Image { name: String, width: i64, height: i64 }"
    - "Tags(Vec<String>)"
    - "OptionalNote(Option<String>)"
```

Quote complex variant strings in YAML when they contain `{}` or `<>`. In the GUI, just type one variant per line.

The first variant must always be simple (no data). If Use `None`, `Empty`, or `Nothing`. The generated code puts `#[derive(Default)]` with `#[default]` on it, and Rust only allows that on unit variants.

Three variant forms:

| Form | Syntax | Example |
|------|--------|---------|
| Simple | `Name` | `Active` |
| Tuple | `Name(Type, ...)` | `"Text(String)"`, `"Pair(i64, String)"` |
| Struct | `Name { field: Type, ... }` | `"Image { name: String, width: i64 }"` |

For inner types you can use:
- Rust scalars: `bool`, `i8`–`i128`, `u8`–`u128`, `f32`, `f64`, `String`
- Shorthands: `Uuid` → `uuid::Uuid`, `DateTime` → `chrono::DateTime<chrono::Utc>`, `EntityId`
- Wrappers: `Option<T>`, `Vec<T>` (nestable, e.g. `Option<Vec<String>>`)
- Any PascalCase name that matches an existing enum or entity. The check command validates that the name exists.

Only available when the manifest language is `rust`. The check command will tell you if you try to use them with a C++ target.

If you enabled the mobile bridge (iOS/Android), these work too. UniFFI's `#[derive(uniffi::Enum)]` handles tuple and struct variants natively, generating proper Swift/Kotlin bindings. Keep in mind that UniFFI copies enums by value across FFI, so avoid putting large data in variants if performance matters.

### List Fields

Entity fields can be declared as lists of primitive values using `is_list: true`. This works the same way as list fields in DTOs.

```yaml
- name: labels
  type: string
  is_list: true

- name: scores
  type: float
  is_list: true

- name: version_ids
  type: uuid
  is_list: true
```

**Constraints:**
- `is_list` can only be used with primitive types: `boolean`, `integer`, `uinteger`, `float`, `string`, `uuid`, `datetime`.
- `is_list` cannot be used with `entity` or `enum` field types.
- `is_list` and `optional` are mutually exclusive on the same field.

**Generated types:**

| Field type   | Rust type              | C++/Qt type         |
|--------------|------------------------|---------------------|
| `string`     | `Vec<String>`          | `QList<QString>`    |
| `integer`    | `Vec<i32>`             | `QList<int>`        |
| `uinteger`   | `Vec<u32>`             | `QList<uint>`       |
| `float`      | `Vec<f32>`             | `QList<float>`      |
| `boolean`    | `Vec<bool>`            | `QList<bool>`       |
| `uuid`       | `Vec<Uuid>`            | `QList<QUuid>`      |
| `datetime`   | `Vec<DateTime<Utc>>`   | `QList<QDateTime>`  |

**Storage:**
- **Rust**: stored as plain `Vec<T>` in the entity struct within the in-memory HashMap store (same as all other fields).
- **C++/Qt**: serialized as JSON arrays in SQLite TEXT columns. `QList<QUuid>` and `QList<QDateTime>` have registered `QMetaType` converters for QVariantList round-tripping (QML interop).

---

## Relationship Fields

This section will seem to be a bit repetitive, but for those not familiar with database relationships, it's important to get all the details right. And some people have different ways of understanding relationships, so I want to be as clear as possible. Bear with me.

When `type: entity`, additional options define the relationship:

### Relationship Types

| Relationship          | Junction Type       | Return Type (C++ / Rust)             |
|-----------------------|---------------------|--------------------------------------|
| `one_to_one`          | OneToOne            | `std::optional<int>` / `Option<i64>` |
| `many_to_one`         | ManyToOne           | `std::optional<int>` / `Option<i64>` |
| `one_to_many`         | UnorderedOneToMany  | `QList<int>` / `Vec<i64>`            |
| `ordered_one_to_many` | OrderedOneToMany    | `QList<int>` / `Vec<i64>`            |
| `many_to_many`        | UnorderedManyToMany | `QList<int>` / `Vec<i64>`            |

### Relationship Flags

| Flag       | Valid for                                          | Effect                                              |
|------------|----------------------------------------------------|-----------------------------------------------------|
| `optional` | `one_to_one`, `many_to_one`                        | Validated on create/update (0..1 instead of 1..1)   |
| `strong`   | `one_to_one`, `one_to_many`, `ordered_one_to_many` | Cascade deletion — removing parent removes children |

### QML Generation Flags (C++/Qt only)

| Flag                         | Effect                                                 |
|------------------------------|--------------------------------------------------------|
| `list_model`                 | Generate `{Entity}ListModelFrom{Parent}{Relationship}` |
| `list_model_displayed_field` | Default display role for the list model                |

### Validation Rules

| Flag       | `one_to_one`   | `many_to_one` | `one_to_many` | `ordered_one_to_many` | `many_to_many` |
|------------|----------------|---------------|---------------|-----------------------|----------------|
| `optional` | ✓/✗            | ✓/✗           | N.A.          | N.A.                  | N.A.           |
| `strong`   | ✓/✗ (see note) | N.A.          | ✓/✗           | ✓/✗                   | N.A.           |

N.A.: Not applicable for this relationship type. There will be no change in generated code, or don't use them. When you write code, an empty list expresses the intent to hold no relationship.

Note: If one_to_one holds a weak relationship (`strong: false`), it couldn't be required (so, it must be `optional: true`). There is the risk of a dangling reference if the entity targeted by the reference is deleted.

Some invalid combinations are rejected at manifest parsing. This validation step is still in the works.

---

## Understanding Relationships

Database relationships describe how entities connect. Two concepts matter:

**Cardinality** — How many entities can participate on each side?
- **One** — At most one entity (0..1 or exactly 1)
- **Many** — Zero or more entities (0..*)

**Direction** — Which side "owns" the relationship?
- The **parent** side holds the list of children
- The **child** side holds a reference back to its parent (in Qleany case, this reference is hidden from the user)

The reality in Qleany is a bit more nuanced, but this mental model helps understand how to model your data.
```
┌─────────────────────────────────────────────────────────────┐
│                     RELATIONSHIP TYPES                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ONE-TO-ONE (1:1)                                          │
│   ┌───────┐         ┌───────┐                               │
│   │ User  │─────────│Profile│   Each user has one profile   │
│   └───────┘         └───────┘   Each profile belongs to     │
│                                 one user                    │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ONE-TO-MANY (1:N)                                         │
│   ┌───────┐         ┌───────┐                               │
│   │Group  │────────<│ Item  │   One group  has many items   │
│   └───────┘         └───────┘   Binder.items: [1, 2, 3]     │
│                                                             │
│   MANY-TO-ONE (N:1)                                         │
│   ┌───────┐         ┌───────┐                               │
│   │ Car   │>────────│Brand  │   Many items belong to one    │
│   └───────┘         └───────┘   brand.  Car.brand: 5        │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   MANY-TO-MANY (N:M)                                        │
│   ┌───────┐         ┌───────┐                               │
│   │ Item  │>───────<│  Tag  │   Items have many tags        │
│   └───────┘         └───────┘   Tags apply to many items    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Reality in Qleany

Like said earlier, the reality is a bit more nuanced:
- Special junction tables are used for all relationships (even the simpler ones) and "sit" between parent and child tables
- These junction tables can be accessed by parent and child tables equally.
- This means that for every relationship, both sides can see each other (no true "back-reference" concept)
- The relationship type defines how the junction table behaves, and how the parent and child entities see each other.
- In the deeper code, there is always the mentions of a left entity and a right entity (parent and child respectively in the mental model).

This may be easier to understand: all relationships are defined from the perspective of the entity holding the field. This means that:
- For `one_to_one`, the entity with the field is one side, the referenced entity is the other side. `Car.brand: EntityId`
- For `many_to_one`, the entity with the field is the "many" side (several entities of the same type), the referenced entity is the "one" side. `Car.brand: EntityId`
- For `one_to_many` and `ordered_one_to_many`, the entity with the field is the "one" side, the referenced entity is the "many" side. `Car.brand: Vec<EntityId>`
- For `many_to_many`, both sides are "many". `Car.brand: Vec<EntityId>`

**Why this design?** The manifest describes *meaning*: ownership, cardinality, ordering, whether deletion cascades. The generated code handles *implementation*: junction tables, column schemas, query patterns, cache invalidation. You think about your domain; the generator thinks about the database. These are deliberately separated.

The cost is storage overhead. A `one_to_one` that could be a single foreign key column gets its own junction table instead. For a desktop or mobile application with hundreds or thousands of entities, you will never notice. For a web backend serving millions of rows, you would care, but that's not what Qleany targets.

The payoff is uniformity. Every relationship, regardless of type, goes through the same junction table infrastructure. This means one code path for snapshot/restore (which is what makes undo/redo work on entity trees), one code path for bidirectional navigation (both sides can always see each other), and one code path for the generator to produce. No special cases, no "this relationship is simple enough for a foreign key but that one needs a junction table." The complexity stays in the infrastructure, not in your head.

Yes, database engineers might cringe at this, but this greatly simplifies the code generation and the overall mental model when designing your entities. They can cringe more when I say there is no notion of foreign keys in Qleany internal database.

**When to use each:**

| Relationship          | Use when...                           | Example                           |
|-----------------------|---------------------------------------|-----------------------------------|
| `one_to_one`          | Exactly one related entity, exclusive | User → Profile                    |
| `many_to_one`         | Many entities reference one child     | Car → Brand, Comment → Post       |
| `one_to_many`         | Parent owns a collection of children  | Binder → Items, Post → Comments   |
| `ordered_one_to_many` | Same as above, but order matters      | Book → Chapters, Playlist → Songs |
| `many_to_many`        | Entities share references both ways   | Items ↔ Tags, Students ↔ Courses  |

There is no `ordered_many_to_many` because I'm not mad enough to handle that complexity.

### Relationship Examples

```yaml
# Exclusive single reference (0..1) — each side has at most one. UserProfile is owned.
- name: profile
  type: entity
  entity: UserProfile
  relationship: one_to_one
  strong: true
  optional: true

# Back-reference to parent (N:1) — many children point to one parent
- name: parentItem
  type: entity
  entity: BinderItem
  relationship: many_to_one

# Required back-reference
- name: binder
  type: entity
  entity: Binder
  relationship: many_to_one
  # implied:
  optional: false

# Unordered children with cascade delete (1:N)
- name: tags
  type: entity
  entity: BinderTag
  relationship: one_to_many
  strong: true

# Ordered children (1:N with order)
- name: chapters
  type: entity
  entity: BinderItem
  relationship: ordered_one_to_many
  strong: true

# Shared references (N:M)
- name: tags
  type: entity
  entity: BinderTag
  relationship: many_to_many
```

### Weak Relationships

Both `many_to_one` and `many_to_many` are always weak — they reference entities owned elsewhere. They cannot have `strong: true`. In Qleany, the owning side (with a `strong` relationship) controls cascade deletion. For `many_to_one` and `many_to_many`, theoretically, it would mean that a child would be deleted only if there was no parent left to "own" it. Too difficult to implement, so no.

Dev note: theoretically, you can play with the junction table code base to support many_to_one with strong ownership, but that would be a nightmare to maintain and reason about.

```yaml
entities:
  - name: Work
    fields:
      - name: tags                        # Owns the tags (strong one-to-many)
        type: entity
        entity: BinderTag
        relationship: one_to_many
        strong: true

  - name: Binder
    fields:
      - name: items                       # Owns the items (strong ordered)
        type: entity
        entity: BinderItem
        relationship: ordered_one_to_many
        strong: true

  - name: BinderItem
    fields:
      - name: binder                      # Back-reference (weak many-to-one)
        type: entity
        entity: Binder
        relationship: many_to_one

      - name: tags                        # Shared reference (weak many-to-many)
        type: entity
        entity: BinderTag
        relationship: many_to_many
```

> **Rule of thumb:** Every entity referenced by a weak relationship (`many_to_one` or `many_to_many`) must be strongly owned somewhere else in your entity graph. Without strong ownership, entities become orphans with no lifecycle management.

---

## Features and Use Cases

Features group related use cases together.

```yaml
features:
  - name: file_management
    use_cases:
      - name: load_file
        # ...
      - name: save_file
        # ...
```

### Use Case Options

| Option           | Type   | Default  | Description                                   |
|------------------|--------|----------|-----------------------------------------------|
| `name`           | string | required | Use case name (snake_case)                    |
| `undoable`       | bool   | false    | Generate undo/redo command scaffolding        |
| `read_only`      | bool   | false    | No data modification (affects generated code) |
| `long_operation` | bool   | false    | Async execution with progress                 |
| `entities`       | list   | []       | Entities this use case works with             |
| `dto_in`         | object | none     | Input DTO for this use case                   |
| `dto_out`        | object | none     | Output DTO for this use case                  |


In Rust, `entities` are doing a bit of the legwork for you to define which repositories are injected into the use case struct and prepare the use of a special macro `macros::uow_action` to simplify unit of work handling. These macro lines must be adapted in your use cases files, and the exact same macros must be repeated in these use cases' unit of work files. Commentary lines will be generated to help you find and adapt these lines.

Similar macros are offered on the C++/Qt side.

### DTOs

Each use case can have input and output DTOs, or only one, or none at all.

```yaml
use_cases:
  - name: import_inventory
    dto_in:
      name: ImportInventoryDto
      fields:
        - name: file_path
          type: string
        - name: skip_header
          type: boolean
        - name: inventory_type
          type: enum
          enum_name: InventoryType
          enum_values:
            - Full
            - Incremental
    dto_out:
      name: ImportReturnDto
      fields:
        - name: imported_count
          type: integer
        - name: error_messages
          type: string
          is_list: true
```

You can't put entities in DTOs. Only primitive types are allowed because entities are tied to the database and business logic, while DTOs are simple data carriers. Think of it as a way to control the data flow between the UI and the business logic. You don't want to expose a password to the user UI just because it's in an entity.

### DTO Field Options

| Option        | Type   | Default  | Description                                                  |
|---------------|--------|----------|--------------------------------------------------------------|
| `name`        | string | required | Field name (snake_case)                                      |
| `type`        | string | required | Field type (boolean, integer, float, string, uuid, datetime) |
| `is_list`     | bool   | false    | Field is a list/array                                        |
| `optional`    | bool   | false    | Field can be Option<>/std::optional                          |
| `enum_name`   | string | none     | For `enum` type, name of the enum                            |
| `enum_values` | list   | none     | For `enum` type, list of values (supports complex variants for Rust, see Enum Fields) |


### User Interface Options

```yaml

  ui:
    rust_cli: true
    rust_slint: true
    rust_ios: false
    rust_android: false
    cpp_qt_qtwidgets: false
    cpp_qt_qtquick: false


```

These options allow the generation of scaffolding for the UI. They will each live in their own separate folders, also separate from the common backend code.

One backend, many frontends.

| Flag | Effect |
|------|--------|
| `rust_cli` | Generates a Clap CLI crate |
| `rust_slint` | Generates a Slint desktop UI crate |
| `rust_ios` | Generates `mobile_bridge` crate + Swift async wrappers + iOS README |
| `rust_android` | Generates `mobile_bridge` crate + Kotlin suspend wrappers + Android README |
| `cpp_qt_qtwidgets` | Generates C++/Qt Widgets scaffolding |
| `cpp_qt_qtquick` | Generates C++/Qt Quick/QML scaffolding |

Either `rust_ios` or `rust_android` triggers generation of the `mobile_bridge` crate with UniFFI bindings. See `qleany docs mobile` for details.