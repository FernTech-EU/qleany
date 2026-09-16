# Qleany: UI strings (source locale).
#
# A single `&` marks the keyboard mnemonic of a menu label; `&&` is a literal `&`.
# Every user-visible string in the app resolves through this file via `tr!`, which
# validates keys at compile time. Data that is not translated (entity names, file
# paths, a manifest path) goes through `lit!` instead and never appears here.

## Application

app-name = Qleany
window-title-no-manifest = Qleany
window-title-with-path = Qleany - { $path }
window-title-unsaved = Qleany - new manifest without path

## Navigation rail

nav-home = Home
nav-project = Project
nav-entities = Entities
nav-features = Features
nav-user-interface = User Interface
nav-generate = Generate

## Menu bar: File

menu-file = &File
menu-new-manifest = &New manifest
menu-open-manifest = &Open manifest…
menu-save-manifest = &Save
menu-save-manifest-as = Save &as…
menu-close-manifest = &Close
menu-run-demo = Run &demo…
menu-quit = &Quit

## Menu bar: Edit

menu-edit = &Edit

## Menu bar: View

menu-view = &View
menu-theme-light = &Light theme
menu-theme-dark = &Dark theme

## Menu bar: Help

menu-help = &Help
menu-about = &About Qleany

## Native application menu (macOS)

native-menu-about = About { $app }
native-menu-quit = Quit { $app }

## Title bar controls

titlebar-save = Save manifest
titlebar-save-disabled = No unsaved changes
titlebar-theme-to-dark = Switch to dark theme
titlebar-theme-to-light = Switch to light theme
titlebar-undo = Undo
titlebar-redo = Redo

## Status strip

status-manifest-saved = Manifest saved successfully
status-mermaid-copied = Entities exported to mermaid markdown and copied to clipboard
status-generated-files = Generated { $count } files
status-error = Error: { $message }

## Home screen

home-title = Qleany
home-subtitle = Welcome to Qleany, a scaffolding generator for C++/Qt6 and Rust.
home-new-manifest = New manifest
home-open-manifest = Open manifest
home-save-manifest = Save manifest
home-save-manifest-as = Save manifest as…
home-close-manifest = Close current manifest
home-run-demo = Run demo
home-documentation = Documentation
home-for-testing = For testing
home-open-qleany-manifest = Open Qleany manifest

## Home screen: documentation links

doc-introduction = Introduction
doc-introduction-blurb = What is Qleany and why use it?
doc-quick-start-cpp-qt = Quick start, C++/Qt
doc-quick-start-cpp-qt-blurb = Build a complete app step by step
doc-quick-start-rust = Quick start, Rust
doc-quick-start-rust-blurb = Build a complete app step by step
doc-design-philosophy = Design philosophy
doc-design-philosophy-blurb = Why Qleany generates code the way it does
doc-undo-redo = Undo redo architecture
doc-undo-redo-blurb = How to add undo/redo support to your app
doc-how-operations-flow = How operations flow
doc-how-operations-flow-blurb = Button to DB and back, error handling, and more
doc-manifest-reference = Manifest reference
doc-manifest-reference-blurb = All YAML options explained
doc-qml-integration = QML integration
doc-qml-integration-blurb = Using the generated QML components, and UI mocking
doc-troubleshooting = Troubleshooting
doc-troubleshooting-blurb = Common issues and fixes

## About box

about-title = About Qleany
about-version = Version { $version }
about-made-by = Made by FernTech
about-licence = Mozilla Public License 2.0
about-docs-link = Documentation
about-repository-link = Repository

## Forms

form-required-marker = *

## Project settings screen

project-title = Project settings
project-language = Language
project-language-rust = Rust
project-language-cpp-qt = C++ / Qt
project-application-name = Application name
project-application-name-placeholder = Enter application name
project-application-name-required = Application name is required
project-organisation-name = Organisation name
project-organisation-name-placeholder = Enter organisation name
project-organisation-name-required = Organisation name is required
project-organisation-domain = Organisation domain
project-organisation-domain-placeholder = Enter organisation domain, for example com.example
project-organisation-domain-required = Organisation domain is required
project-prefix-path = Prefix path
project-prefix-path-placeholder = default: { $path }

## Entities screen

entities-title = Entities
entities-list-heading = Entities
entities-add = Add entity
entities-delete = Delete entity
entities-export-mermaid = Export to Mermaid
entities-empty = No entities yet
entities-empty-hint = Add one to describe the data your application works with.
entities-none-selected = No entity selected
entities-none-selected-hint = Pick an entity on the left to edit it.
entities-name = Name
entities-name-placeholder = Enter entity name
entities-name-required = Entity name is required
entities-name-pascal-case = Entity name must be in PascalCase
entities-only-for-heritage = Only for heritage
entities-only-for-heritage-hint = An abstract entity: never stored, only inherited from.
entities-inherits-from = Inherits from
entities-single-model = Single model
entities-undoable = Undoable
entities-subtitle-abstract = abstract
entities-subtitle-extends = extends { $parent }
entities-subtitle-both = { $left } · { $right }

## Entity fields

fields-list-heading = Fields
fields-add = Add field
fields-delete = Delete field
fields-empty = No fields yet
fields-empty-hint = Add one to describe what this entity stores.
fields-name = Name
fields-name-placeholder = Enter field name
fields-name-required = Field name is required
fields-name-snake-case = Field name must be in snake_case
fields-type = Type
fields-referenced-entity = Referenced entity
fields-relationship = Relationship type
fields-optional = Optional
fields-is-list = List
fields-strong = Strong (cascade delete)
fields-list-model = List model
fields-displayed-field = List model displayed field
fields-displayed-field-placeholder = Enter displayed field name
fields-enum-name = Enum name
fields-enum-name-placeholder = Enter enum name
fields-enum-name-required = Enum name is required
fields-enum-name-pascal-case = Enum name must be in PascalCase
fields-enum-values = Enum values, one per line in PascalCase

## User interface targets

ui-title = User interface
ui-section-rust = Rust user interfaces
ui-section-cpp-qt = C++ / Qt user interfaces
ui-target-rust-cli = CLI
ui-target-rust-teksilo = Teksilo (recommended)
ui-target-rust-slint = Slint
ui-target-rust-ios = iOS (UniFFI)
ui-target-rust-android = Android (UniFFI)
ui-target-cpp-qt-widgets = Qt Widgets
ui-target-cpp-qt-quick = Qt Quick

## Features screen

features-title = Features
features-list-heading = Features
features-add = Add feature
features-delete = Delete feature
features-empty = No features yet
features-empty-hint = A feature groups the use cases that belong together.
features-none-selected = No feature selected
features-name = Name
features-name-placeholder = Enter feature name
features-name-required = Feature name is required
features-name-snake-case = Feature name must be in snake_case

## Use cases

use-cases-list-heading = Use cases
use-cases-add = Add use case
use-cases-delete = Delete use case
use-cases-empty = No use cases yet
use-cases-empty-hint = A use case is one operation this feature performs.
use-cases-none-selected = No use case selected
use-cases-none-selected-hint = Pick a use case to edit it.
use-cases-name = Name
use-cases-name-placeholder = Enter use case name
use-cases-name-required = Use case name is required
use-cases-name-snake-case = Use case name must be in snake_case
use-cases-read-only = Read only
use-cases-read-only-hint = Reads and never writes, so it has nothing to undo.
use-cases-undoable = Undoable
use-cases-long-operation = Long operation
use-cases-entities = Entities this use case touches
use-cases-entities-empty = No entities yet
use-cases-entities-empty-hint = Add an entity on the Entities screen first.

## DTOs

dto-in-heading = Input DTO
dto-out-heading = Output DTO
dto-in-enable = Enable DTO In
dto-out-enable = Enable DTO Out
dto-in-disabled-hint = Enable DTO In to configure it
dto-out-disabled-hint = Enable DTO Out to configure it
dto-name = Name
dto-name-placeholder = Enter DTO name
dto-name-required = DTO name is required
dto-name-pascal-case = DTO name must be in PascalCase
dto-fields-heading = Fields
dto-field-add = Add field
dto-field-delete = Delete field
dto-fields-empty = No fields yet
dto-fields-empty-hint = Add one to describe what this DTO carries.
dto-field-none-selected = No field selected
dto-field-none-selected-hint = Pick a field to edit it.
dto-field-name = Name
dto-field-name-placeholder = Enter field name
dto-field-name-required = Field name is required
dto-field-name-snake-case = DTO field name must be in snake_case
dto-field-type = Type
dto-field-optional = Optional
dto-field-is-list = List
dto-field-enum-name = Enum name
dto-field-enum-name-placeholder = Enter enum name
dto-field-enum-name-required = Enum name is required
dto-field-enum-name-pascal-case = Enum name must be in PascalCase
dto-field-enum-values = Enum values, one per line in PascalCase
dto-disable-title = Delete this DTO?
dto-disable-message = { $name } and its { $count } fields will be deleted.

## Manifest validation

check-panel-title = Manifest validation
check-recheck = Check again
check-all-clear = This manifest validates.
check-tooltip-ok = The manifest validates
check-tooltip-warning = The manifest has warnings
check-tooltip-critical = The manifest has errors and will not generate

## Generate screen

generate-title = Generate
generate-groups-heading = Groups
generate-filter-placeholder = Filter by path or name
generate-status-modified = Modified
generate-status-new = New
generate-status-unchanged = Unchanged
generate-nature-infrastructure = Infra
generate-nature-aggregate = Aggregate
generate-nature-scaffold = Scaffold
generate-select-all = Select all
generate-unselect-all = Unselect all
generate-in-temp = In temp/
generate-view-diff = View diff
generate-run = Generate ({ $count })
generate-refresh = Recompute
generate-cancel = Cancel
generate-empty = No files match
generate-empty-hint = Widen the filters, or pick another group.
generate-no-selection = No file selected
generate-no-selection-hint = Pick a file to see what would be written.
generate-no-differences = No differences
generate-step-rendering = Rendering the files this manifest implies
generate-step-writing = Writing files
generate-computing-title = Computing file status
generate-generating-title = Generating files

## Undo and redo

undo-menu-plain = &Undo
undo-menu-labelled = &Undo { $action }
redo-menu-plain = &Redo
redo-menu-labelled = &Redo { $action }
undo-toast = Undone: { $action }
redo-toast = Redone: { $action }
undo-edit-project = project settings
undo-add-entity = add entity
undo-remove-entity = remove entity
undo-reorder-entities = reorder entities
undo-edit-entity = edit entity
undo-add-field = add field
undo-remove-field = remove field
undo-reorder-fields = reorder fields
undo-edit-field = edit field
undo-add-feature = add feature
undo-remove-feature = remove feature
undo-reorder-features = reorder features
undo-edit-feature = edit feature
undo-add-use-case = add use case
undo-remove-use-case = remove use case
undo-reorder-use-cases = reorder use cases
undo-edit-use-case = edit use case
undo-enable-dto = enable DTO
undo-disable-dto = delete DTO
undo-edit-dto = edit DTO
undo-add-dto-field = add DTO field
undo-remove-dto-field = remove DTO field
undo-reorder-dto-fields = reorder DTO fields
undo-edit-dto-field = edit DTO field
undo-edit-user-interface = edit user interface targets

## New manifest wizard

wizard-title = New manifest
wizard-step-language = Language
wizard-step-names = Names
wizard-step-template = Template
wizard-step-targets = User interfaces
wizard-back = Back
wizard-next = Next
wizard-create = Create
wizard-cancel = Cancel
wizard-required = Required
wizard-must-be-pascal-case = Must be PascalCase
wizard-language-rust = Rust
wizard-language-rust-blurb = 2024 edition
wizard-language-cpp-qt = C++ / Qt
wizard-language-cpp-qt-blurb = C++ 20 and Qt 6
wizard-application-name = Application name
wizard-application-name-placeholder = MyApplication
wizard-organisation-name = Organisation name
wizard-organisation-name-placeholder = FernTech
wizard-template-blank = Blank
wizard-template-blank-blurb = Nothing but the project scaffolding.
wizard-template-minimal = Minimal
wizard-template-minimal-blurb = One entity and one feature, to build on.
wizard-template-document-editor = Document editor
wizard-template-document-editor-blurb = Documents, open and save, undo and redo.
wizard-template-data-management = Data management
wizard-template-data-management-blurb = Records, lists and forms over a store.
wizard-targets-footnote = You can change these later on the User Interface screen.
wizard-targets-cpp-note = The Qt targets need a Qt 6 installation and a git tag to build against.

## Destructive deletes

confirm-delete-entity-title = Delete this entity?
confirm-delete-entity = { $name } goes, with its fields and its relationships.
confirm-delete-feature-title = Delete this feature?
confirm-delete-feature = { $name } goes, with its use cases and their DTOs.
confirm-delete-use-case-title = Delete this use case?
confirm-delete-use-case = { $name } goes, with its input and output DTOs.

## Quitting with unsaved work

quit-unsaved-title = Save before quitting?
quit-unsaved-message = This manifest has changes that are not on disk yet.

## Demo generator

demo-title = Run the demo
demo-blurb = Generate a complete sample project: a manifest, every crate it implies, and a build that runs.
demo-language = Language
demo-language-rust-blurb = A Cargo workspace, with a CLI and a Teksilo window.
demo-language-cpp-qt-blurb = A CMake project, with Qt Quick and Qt Widgets applications.
demo-destination = Destination
demo-browse = Browse...
demo-destination-hint = The project is written into this folder, which is created if it does not exist.
demo-generate = Generate
demo-open-folder = Open folder
demo-copy = Copy the command
demo-copied = Copied

demo-running-title = Generating the demo project

demo-step-preparing = Preparing...
demo-step-folder = Creating the output folder...
demo-step-manifest = Creating the manifest...
demo-step-loading = Loading the manifest...
demo-step-checks = Validating the manifest...
demo-step-listing = Working out which files to write...
demo-step-rendering = Generating code...
demo-step-comparing = Comparing with the files on disk...
demo-step-writing = Writing files to disk...
demo-step-git = Creating the git repository and its first tag...
demo-step-done = Done!

demo-success = Demo project generated successfully!
demo-summary-stats = Generated { $files } files from a manifest of { $lines } lines.
demo-includes-title = Your project includes:
demo-includes-crud = Complete CRUD infrastructure: controllers, DTOs, use cases and repositories
demo-includes-undo = Multi-stack undo and redo, with cascade snapshot and restore
demo-includes-events = A thread-safe event system with buffering, so events fire only on success
demo-includes-relationships = Relationship management with ordering and cascade deletion
demo-includes-tests = A generated test suite
demo-includes-ui = Scaffolds for two user interfaces: { $first } and { $second }
demo-ui-teksilo = Teksilo
demo-next-step = Next step:

demo-error-exists = A project already exists at { $path }. Remove it, or choose another folder.
demo-error-no-git = Git is not installed, or is not on PATH. The C++/Qt demo needs it to create the tag its build reads a version from.
demo-error-no-git-identity = Git has no user.name or no user.email, so it cannot make the demo's first commit. Set them with: git config --global user.name "Your Name" and git config --global user.email "you@example.com"
demo-error-failed = The demo could not be generated: { $message }

demo-unsaved-title = Save before running the demo?
demo-unsaved-message = The demo loads a manifest of its own, which closes this one. It has changes that are not on disk yet.

## Native file dialogs

# The desktop's own dialogs, which take a resolved string rather than a
# LocalizedString. They are still what the user reads, so they are still keys.

dialog-manifest-filter = Qleany manifest
dialog-open-manifest = Open a Qleany manifest
dialog-save-manifest-as = Save the manifest as
dialog-create-manifest = Create a Qleany manifest
dialog-demo-destination = Where should the demo project go?

## Shared

common-cancel = Cancel
common-close = Close
common-none = None
common-more-actions = More actions
