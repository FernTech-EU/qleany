# Qleany Quick Start - Rust

This guide walks you through creating a complete desktop application for a car dealership using Qleany. By the end, you'll have generated architecture with entities, repositories, controllers, and undo/redo infrastructure. After generation, the only code you write is inside the use cases (your business logic) and the UI. Everything else compiles and works out of the box.

For C++ / Qt, see [Qleany Quick Start - C++/Qt](quick-start-cpp-qt.md). The differences are minor.

The qleany.yaml of this example is available [here](../examples/rust/quick_start_carlot/qleany.yaml).

---

## Step 1: Think About Your Domain

Before touching any tool, grab paper or open a diagramming tool. This is the most important step.

**Ask yourself:**
- What are the core "things" in my business? These become entities.
- What actions do users perform? These become use cases.
- Which use cases belong together? These become features.

### Example: CarLot — A Car Dealership App

**Entities** (the nouns):

| Entity     | Purpose                                  | Key Fields                            |
|------------|------------------------------------------|---------------------------------------|
| EntityBase | Base class for all entities              | id, created_at, updated_at            |
| Root       | Application entry point, owns everything | cars, customers, sales                |
| Car        | Vehicle in inventory                     | make, model, year, price, status      |
| Customer   | Potential or actual buyer                | name, email, phone                    |
| Sale       | Completed transaction                    | sale_date, final_price, car, customer |

**Relationships:**

- Root owns many Cars (inventory)
- Root owns many Customers (contacts)
- Root owns many Sales (history)
- Sale references one Car (what was sold)
- Sale references one Customer (who bought it)

**Features and Use Cases** (the verbs):

| Feature              | Use Case         | What it does                          |
|----------------------|------------------|---------------------------------------|
| inventory_management | import_inventory | Parse CSV file, populate Car entities |
| inventory_management | export_inventory | Generate CSV from current inventory   |

### Draw It First

Sketch your entities and relationships before using Qleany. Use paper, whiteboard, or [Mermaid](https://mermaid.live/).

Deeper explanations about relationships are available in the [Manifest Reference](manifest-reference.md#understanding-relationships).

```mermaid
erDiagram
    EntityBase {
        EntityId id
        datetime created_at
        datetime updated_at
    }

    Root {
        EntityId id
        datetime created_at
        datetime updated_at
        # relationships:
        Vec<EntityId> cars
        Vec<EntityId> customers
        Vec<EntityId> sales
    }
    
    Car {
        EntityId id
        datetime created_at
        datetime updated_at
        string make
        string model
        int year
        float price
        enum status
    }
    
    Customer {
        EntityId id
        datetime created_at
        datetime updated_at
        string name
        string email
        string phone
    }
    
    Sale {
        EntityId id
        datetime created_at
        datetime updated_at
        datetime sale_date
        float final_price
        int car_id
        int customer_id
        # relationships:
        EntityId car
        EntityId customer
    }

    EntityBase ||--o{ Root : "inherits"
    EntityBase ||--o{ Car : "inherits"
    EntityBase ||--o{ Customer : "inherits"
    EntityBase ||--o{ Sale : "inherits"
    Root ||--o{ Car : "owns (strong)"
    Root ||--o{ Customer : "owns (strong)"
    Root ||--o{ Sale : "owns (strong)"
    Sale }o--|| Car : "optionally references"  # Many-to-One (a sale may exist without a car, e.g., if the car was deleted)
    Sale }o--|| Customer : "optionally references" # Many-to-One 
```

**Why draw first?** Changing a diagram is free. Changing generated code is work. Get the model right before generating.

`EntityBase` is a common pattern: it provides shared fields like `id`, `created_at`, and `updated_at`, like an inheritance. Other entities can explicitly inherit from it. This is not an entity. It will never be generated. All your entities can inherit from it to avoid repetition.

> Note: You can note the relationships on the diagram too. Qleany supports various relationship types (one-to-one, one-to-many, many-to-one, many-to-many) and cascade delete (strong relationships). Defining these upfront helps you configure them correctly in the manifest. Unlike typical ER diagrams, the relationships appear as fields. Forget the notion of foreign keys here. Qleany's relationships are directional and can be configured with additional options (e.g., ordered vs unordered, strong vs weak, optional or not (only for some relationship types)). Plan these carefully to ensure the generated code matches your intended data model.

**WRONG**: I only need a few entities without any "owner" relationships. I can just create them in Qleany and skip the Root entity.

**RIGHT**: I want a clear ownership structure. Root owns all Cars, Customers, and Sales. This makes it easy to manage the lifecycle of entities. It avoids orphan entities and simplifies the generated code. Even if Root has few fields, it provides a clear parent-child structure. Think like a tree: Root is the trunk, Cars/Customers/Sales are branches. This is a common pattern in Qleany projects.

---

## Step 2: Create a New Manifest

### Using the GUI

Launch Qleany. You'll land on the **Home** tab.

1. Click **New Manifest** — a creation wizard opens
2. **Step 1 — Language**: Select **Rust**
3. **Step 2 — Project**: Enter your application name (PascalCase, e.g. `CarLot`) and organisation name (e.g. `MyCompany`)
4. **Step 3 — Template**: Choose a starting template:
   - **Blank** — EntityBase + empty Root (start from scratch)
   - **Minimal** — Root with one entity (Item). Hello world equivalent
   - **Document Editor** — Documents > Sections with load/save use cases
   - **Data Management** — Items, Categories, Tags with import/export use cases
5. **Step 4 — UI Options**: Enable **Teksilo (Desktop GUI, recommended)**, **CLI**, and/or **Slint (Desktop GUI)**
6. Click **Create**, then choose where to save `qleany.yaml` (your project root)

### Using the CLI

```bash
qleany new /path/to/project \
  --language rust \
  --name CarLot \
  --org-name MyCompany \
  --template blank \
  --options rust_cli
```

All flags are optional — if omitted, the CLI prompts interactively. Use `--force` to overwrite an existing manifest.

### What gets created

Qleany creates a manifest pre-configured with:
- Your chosen language, application name, and organisation
- `EntityBase` (provides id, created_at, updated_at)
- `Root` entity inheriting from EntityBase (plus more entities if you chose a template other than Blank)
- Your selected UI options

---

## Step 3: Configure Project Settings

Click **Project** in the sidebar to review and adjust settings. The wizard already filled in the language, application name, and organisation name. You can still change:

| Field               | Value         |
|---------------------|---------------|
| Organisation Domain | com.mycompany |
| Prefix Path         | crates        |

Organisation Domain is used for some installed file names, like the icon name.

Changes save. The header shows "Save Manifest" when there are unsaved changes.

---

## Step 4: Define Entities

Click **Entities** in the sidebar. You'll see a three-column layout.

### 4.1 Create the Car Entity

1. Click the **+** button next to "Entities"
2. A new entity appears — click it to select
3. In the details panel:
   - **Name**: `Car`
   - **Inherits from**: `EntityBase`

Now add fields. In the "Fields" section:

1. Click **+** to add a field
2. Select the new field, then configure:

| Name   | Type    | Notes                                                                          |
|--------|---------|--------------------------------------------------------------------------------|
| make   | String  | —                                                                              |
| model  | String  | —                                                                              |
| year   | Integer | —                                                                              |
| price  | Float   | —                                                                              |
| status | Enum    | Enum Name: `CarStatus`, Values: `Available`, `Reserved`, `Sold` (one per line) |

### 4.2 Create the Customer Entity

1. Click **+** next to "Entities"
2. **Name**: `Customer`
3. **Inherits from**: `EntityBase`
4. Add fields:

| Name  | Type   |
|-------|--------|
| name  | String |
| email | String |
| phone | String |

### 4.3 Create the Sale Entity

1. Click **+** next to "Entities"
2. **Name**: `Sale`
3. **Inherits from**: `EntityBase`
4. Add fields:

| Name        | Type     | Configuration                                              |
|-------------|----------|------------------------------------------------------------|
| sale_date   | DateTime | —                                                          |
| final_price | Float    | —                                                          |
| car         | Entity   | Referenced Entity: `Car`, Relationship: `many_to_one`      |
| customer    | Entity   | Referenced Entity: `Customer`, Relationship: `many_to_one` |

### 4.4 Configure Root Relationships

Select the **Root** entity. Add relationship fields:

| Name      | Type   | Configuration                                                                 |
|-----------|--------|-------------------------------------------------------------------------------|
| cars      | Entity | Referenced Entity: `Car`, Relationship: `ordered_one_to_many`, Strong: ✓      |
| customers | Entity | Referenced Entity: `Customer`, Relationship: `ordered_one_to_many`, Strong: ✓ |
| sales     | Entity | Referenced Entity: `Sale`, Relationship: `ordered_one_to_many`, Strong: ✓     |

**Key concepts:**
- **Strong relationship**: Deleting Root cascades to delete all Cars, Customers, Sales

---

## Step 5: Define Features and Use Cases

Click **Features** in the sidebar. You'll see a four-column layout.

### 5.1 Create the Feature

1. Click **+** next to "Features"
2. Select it and set **Name**: `inventory_management`

### 5.2 Create the Import Use Case

1. Click **+** next to "Use Cases"
2. Configure:

| Field          | Value                                        |
|----------------|----------------------------------------------|
| Name           | import_inventory                             |
| Undoable       | ✗ *(file imports typically aren't undoable)* |
| Read Only      | ✗ *(it will update the internal database)*   |
| Long Operation | ✓ *(parsing files can take time)*            |

3. Switch to the **DTO In** tab:
   - Enable the checkbox
   - **Name**: `ImportInventoryDto`
   - Add field: `file_path` (String)

4. Switch to the **DTO Out** tab:
   - Enable the checkbox
   - **Name**: `ImportReturnDto`
   - Add fields: `imported_count` (Integer), `error_messages` (String, List: ✓)

5. Switch to the **Entities** tab:
   - Check: `Root`, `Car`

### 5.3 Create the Export Use Case

1. Click **+** next to "Use Cases"
2. Configure:

| Field          | Value                            |
|----------------|----------------------------------|
| Name           | export_inventory                 |
| Undoable       | ✗                                |
| Read Only      | ✓ *(just reading internal data)* |
| Long Operation | ✗                                |

3. **DTO In**:
   - **Name**: `ExportInventoryDto`
   - Field: `output_path` (String)

4. **DTO Out**:
   - **Name**: `ExportReturnDto`
   - Field: `exported_count` (Integer)

5. **Entities**: Check `Root`, `Car`

### 5.4 UI Options

You already chose your UI frontends (Teksilo, CLI, Slint, or several) during manifest creation. You can change these later in the **User Interface** tab.

**Teksilo** is the recommended Rust desktop target. Qleany generates a complete,
compiling application: a reactive *single* per entity marked `single_model`, a
reactive *list model* per relationship marked `list_model`, a mock twin of each
behind the crate's `mocks` feature, the bridge that delivers backend events on
the UI thread, and a demo window wiring them together.

**Slint** gets a basic UI, event-system integration and command files binding the
UI to the generated controllers. **CLI** uses clap.

Teksilo and Slint can both be enabled; each becomes its own crate over the same
backend. The Teksilo binary takes the plain application name, the Slint one is
suffixed `-slint`.

### 5.5 Save the Manifest

Click **Save Manifest** in the header (or Ctrl+S).

### 5.6 Take a break, drink a coffee, sleep a bit

I mean it. A fresher mind sees things more clearly. You already saved a lot of time by using Qleany instead of writing all the boilerplate yourself. Don't rush the design phase, it's where you get the most value from Qleany.

Designing your domain and use cases is the most important part.  The generated code is a complete architecture, not mere scaffolding. If the model is wrong, the code won't help much. Take your time to get it right before generating. 

Yes, you can change the manifest and regenerate later. But it's better to get a solid design upfront. The more you change the model after generating, the more work you create for yourself. It's not a problem to evolve your design, but try to avoid major changes that require rewriting large parts of the generated code.

---

## Step 6: Save and Generate

### Commit to Git

Before generating, commit your current state to Git. This isn't optional advice — it's how Qleany is meant to be used. If you accidentally overwrite files you've modified, you can restore them.

```bash
git add .
git commit -m "Before Qleany generation"
```

### Generate Code

1. Click **Generate** in the sidebar
2. Review the groups and files. Use the **status** filters (Modified, New, Unchanged) and **nature** filters (Infra, Aggregate, Scaffold) to narrow the list
3. (Optional) Check **in temp/** to generate to a temporary folder first
4. Click a file to preview the generated code
5. Click **Generate (N)** where N is the number of selected files

The progress modal shows generation status. Files are written to your project.

The files are formatted with cargo fmt.

---

## Step 7: What You Get

After a generation, your project contains:

```
Cargo.toml
crates/
├── cli/
│   ├── src/
│   │   ├── main.rs                 # ← write your UI here
│   └── Cargo.toml
├── common/
│   ├── src/
│   │   ├── entities.rs             # Car, Customer, Sale structs
│   │   ├── database.rs
│   │   ├── database/
│   │   │   ├── db_context.rs
│   │   │   ├── db_helpers.rs
│   │   │   └── transactions.rs
│   │   ├── direct_access.rs
│   │   ├── direct_access/         # Holds the repository and table implementations for each entity
│   │   │   ├── use_cases/         # Generics for direct access use cases
│   │   │   ├── car.rs
│   │   │   ├── car/
│   │   │   │   ├── car_repository.rs
│   │   │   │   └── car_table.rs
│   │   │   ├── customer.rs
│   │   │   ├── customer/
│   │   │   │   ├── customer_repository.rs
│   │   │   │   └── customer_table.rs
│   │   │   ├── sale.rs
│   │   │   ├── sale/
│   │   │   │   ├── sale_repository.rs
│   │   │   │   └── sale_table.rs
│   │   │   ├── root.rs
│   │   │   ├── root/
│   │   │   │   ├── root_repository.rs
│   │   │   │   └── root_table.rs
│   │   │   ├── repository_factory.rs
│   │   │   └── setup.rs
│   │   ├── event.rs             # event system for reactive updates
│   │   ├── lib.rs
│   │   ├── long_operation.rs    # infrastructure for long operations
│   │   ├── types.rs         
│   │   └── undo_redo.rs        # undo/redo infrastructure
│   └── Cargo.toml
├── direct_access/                   # a direct access point for UI or CLI to interact with entities
│   ├── src/
│   │   ├── car.rs
│   │   ├── car/
│   │   │   ├── car_controller.rs   # Exposes CRUD operations to UI or CLI
│   │   │   ├── dtos.rs
│   │   │   └── units_of_work.rs
│   │   ├── customer.rs
│   │   ├── customer/
│   │   │   └── ...
│   │   ├── sale.rs
│   │   ├── sale/
│   │   │   └── ...
│   │   ├── root.rs
│   │   ├── root/
│   │   │   └── ...
│   │   └── lib.rs
│   └── Cargo.toml
├── inventory_management/
│   ├── src/
│   │   ├── inventory_management_controller.rs     # Exposes operations to UI or CLI
│   │   ├── dtos.rs
│   │   ├── units_of_work.rs
│   │   ├── units_of_work/          # ← adapt the macros here 
│   │   │   └── ...
│   │   ├── use_cases.rs
│   │   ├── use_cases/              # ← You implement the logic here
│   │   │   └── ...
│   │   └── lib.rs
│   └── Cargo.toml
├── frontend/                    # entry point for UI or CLI to interact with entities and features
│   ├── src/
│   │   ├── lib.rs
│   │   ├── event_hub_client.rs         # event hub client
│   │   ├── app_context.rs              # holds the instances needed by the backend
│   │   ├── commands.rs
│   │   └── commands/                   # convenient wrappers for controller APIs
│   │       ├── undo_redo_commands.rs
│   │       ├── car_commands.rs
│   │       ├── customer_commands.rs
│   │       ├── sale_commands.rs
│   │       └── root_commands.rs
│   └── Cargo.toml
└── teksilo_ui
    ├── Cargo.toml
    └── src
        ├── main.rs
        ├── lib.rs                      # builds and runs the app
        ├── event_source.rs             # backend events → the UI thread
        ├── session.rs                  # every single and model, wired
        ├── undo_redo.rs
        ├── app.rs                      # ← the demo window; replace with your UI
        ├── singles
        │   └── single_car.rs           # one per single_model entity
        └── models
            ├── coalesced_reload.rs
            └── root_cars_list_model.rs # one per list_model relationship



```

**What's generated:**
- Complete CRUD for all entities (create, get, update, remove, ...)
- Controllers exposing operations
- DTOs for data transfer
- Repository pattern for database access
- Undo/redo infrastructure for undoable operations
- Tests suites for the database and undo redo infrastructure
- Event system for reactive updates
- Basic CLI (if selected during project setup)
- A complete Teksilo desktop UI — singles, list models, mocks, demo window (if selected)
- Basic empty Slint UI (if selected during project setup)

**What you implement:**
- Your custom use case logic (import_inventory, export_inventory)
- Your UI or CLI on top of the controllers or their adapters.

---

## Step 8: Run the Generated Code

Let's assume that you have Rust installed.

In a terminal,
```bash
cargo run
```

`cargo run` with no arguments builds the workspace's `default-members`, which the
generator sets to one frontend in the order **Teksilo > Slint > CLI**. To run a
different one, name its binary:

```bash
cargo run --bin my_app          # Teksilo
cargo run --bin my_app-slint    # Slint
cargo run --bin my-app-cli      # CLI
```

The Slint binary carries the `-slint` suffix so it cannot collide with the
Teksilo one when both targets are enabled.

---

## Next Steps

1. Run the generated code — it compiles and provides working CRUD
2. Implement your custom use cases (`import_inventory`, `export_inventory`)
3. Build your UI on top of the controllers
4. Add more features as your application grows

The generated code is yours. Modify it, extend it, or regenerate when you add new entities. Qleany gets out of your way.

## Tips

### Understanding the Internal Store

Entities are stored in an in-memory HashMap store. This store is **internal**, users and UI devs don't interact with it directly.

**Typical pattern:**

1. User opens a file (e.g., `.carlot` project file)
2. Your `load_project` use case parses the file and populates entities
3. User works — all changes go to the internal database
4. User saves — your `save_project` use case serializes entities back to file

The internal database is ephemeral. It enables fast operations, undo/redo. The user's file is the permanent storage.

### Undo/Redo

Every generated CRUD operation supports undo/redo automatically. You don't have to display undo/redo controls in your UI if you don't want to, but the infrastructure is there when you need it.

If you mark a use case as **Undoable**, Qleany generates the command pattern scaffolding. You fill in what "undo" means for your specific operation.

For more information, see [Undo-Redo Architecture](undo-redo-architecture.md).

### Relationships

| Relationship | Use When |
|--------------|----------|
| one_to_one | Exclusive 1:1 (User → Profile) |
| many_to_one | Child references parent (Sale → Car) |
| one_to_many | Parent owns unordered children |
| ordered_one_to_many | Parent owns ordered children (chapters in a book) |
| many_to_many | Shared references (Items ↔ Tags) |

**Strong** means cascade delete — deleting the parent deletes children.

For more details, see [Manifest Reference](manifest-reference.md#relationship-fields).

### Regenerating

Made a mistake? The manifest is just YAML. You can:
- Edit it directly in a text editor or from the GUI tool
- Delete entities/features in the UI and recreate them
- Generate to a temp folder, review, then regenerate to the real location

For more details, see [Regeneration Workflow](regeneration-workflow.md).

---

The generated code is yours. Modify it, extend it, or regenerate when you add new entities. Qleany gets out of your way.

---

## Further Reading

- [README](../README.md) — Overview, building and running, reference implementation
- [Manifest Reference](manifest-reference.md) — Entity options, field types, relationships, features
- [Design Philosophy](design-philosophy.md) — Clean Architecture background, package by feature
- [Regeneration Workflow](regeneration-workflow.md) — How file generation works, what gets overwritten
- [Undo-Redo Architecture](undo-redo-architecture.md) — Entity tree structure, undoable vs non-undoable
- [QML Integration](qml-integration.md) — Reactive models and mocks for C++/Qt
- [Generated Infrastructure - C++/Qt](generated-code-cpp-qt.md) — Database layer, event system, file organization
- [Generated Infrastructure - Rust](generated-code-rust.md) — Database layer, event system, file organization
- [Troubleshooting](troubleshooting.md) — Common issues and how to fix them
