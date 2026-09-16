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
menu-undo = &Undo
menu-undo-labelled = &Undo { $action }
menu-redo = &Redo
menu-redo-labelled = &Redo { $action }

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
status-manifest-created = Manifest created and loaded successfully
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
about-close = Close

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
fields-none-selected = No field selected
fields-none-selected-hint = Pick a field on the left to edit it.
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

## Shared

common-cancel = Cancel
common-close = Close
common-delete = Delete
common-ok = OK
common-save = Save
common-discard = Discard
common-none = None
common-more-actions = More actions
