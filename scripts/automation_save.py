#!/usr/bin/env python3
"""Save must include text still being edited, across every manifest editor.

Each edit starts clean; earlier mutations must not mask a missing dirty binding.
Ctrl+S deliberately does not blur the field before saving.
"""
import os
from pathlib import Path
import sys
import time

import automation_fixture as fixture
import automation_entities as entities
import automation_features as features
from ui_probe_helpers import read_manifest
from teksilo_probe import Report, launch_and_attach, navigate, tree

PROBE = "save"


def clean(session):
    return tree.wait_for_node(session, role="Button", label="No unsaved changes", timeout=5)


def save(session):
    session.tools.inject_key(key="s", command=True)
    session.settle()
    time.sleep(0.3)  # backend events from the flushed edits must drain as well
    session.settle()


def edit_and_save(session, report, manifest, label, before, after, *, field_path, role="TextInput"):
    """Exercise an edit without submit/blur, then check the persisted YAML."""
    if not report.check(clean(session) is not None, f"{label}/{before}: starts clean"):
        return
    field = tree.wait_for_node(session, role=role, label=label, timeout=5,
                               pred=lambda n: n.get("value", "") == before)
    if field is None:
        raise RuntimeError(f"missing {label!r} input containing {before!r}")
    session.tools.focus_node(node=field["id"])
    if role == "MultilineTextInput":
        session.tools.type_text(node=field["id"], text=after)
    else:
        session.tools.set_value(node=field["id"], value=after)
    session.settle()
    enabled = tree.wait_for_node(session, role="Button", label="Save manifest", timeout=3,
                                 pred=lambda n: not n.get("disabled"))
    report.check(enabled is not None, f"{label}/{before}: typing enables Save without Enter or blur")
    save(session)
    written = read_manifest(manifest)
    for key in field_path:
        written = written[key]
    expected = [after] if field_path[-1] == "enum_values" else after
    report.check(written == expected, f"{label}/{before}: Ctrl+S writes the exact YAML field")
    report.check(clean(session) is not None, f"{label}/{before}: saved buffer becomes clean")


def main():
    report = Report(PROBE)
    app = session = None
    try:
        manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), PROBE)
        env = fixture.isolated_config(PROBE)
        env["QLEANY_DEV"] = "1"
        app, session = launch_and_attach(argv=[fixture.app_binary()],
                                        cwd=os.path.dirname(manifest), env=env, label=PROBE)
        navigate.click(session, tree.wait_for_node(session, label="Open Qleany manifest", timeout=30))
        fixture.go_to(session, "Project")
        time.sleep(0.4)
        project_paths = {"Application name": ("global", "application_name"),
                         "Organisation name": ("global", "organisation", "name"),
                         "Organisation domain": ("global", "organisation", "domain"),
                         "Prefix path": ("global", "prefix_path")}
        for label, value in [("Application name", "PendingApp"),
                             ("Organisation name", "PendingOrg"),
                             ("Organisation domain", "eu.pending"),
                             ("Prefix path", "pending_crates")]:
            old = tree.find(session, role="TextInput", label=label)["value"]
            edit_and_save(session, report, manifest, label, old, value, field_path=project_paths[label])

        fixture.go_to(session, "Entities")
        navigate.click(session, entities.rows(session)[0])
        old = tree.find(session, role="TextInput", label="Name")["value"]
        edit_and_save(session, report, manifest, "Name", old, "PendingEntity", field_path=("entities", 0, "name"))
        navigate.click(session, entities.rows(session, entities.FIELD_COLUMN)[0])
        fields = tree.find_all(session, role="TextInput", label="Name")
        old = next(n["value"] for n in fields if n["value"] != "PendingEntity")
        edit_and_save(session, report, manifest, "Name", old, "pending_field", field_path=("entities", 0, "fields", 0, "name"))
        report.check(entities.set_combo(session, "Type", "Enum"), "field can be changed to Enum")
        save(session)
        old = tree.find(session, role="TextInput", label="Enum name").get("value", "")
        edit_and_save(session, report, manifest, "Enum name", old, "PendingEnum", field_path=("entities", 0, "fields", 0, "enum_name"))

        edit_and_save(session, report, manifest, None, "", "PendingVariant",
                      role="MultilineTextInput", field_path=("entities", 0, "fields", 0, "enum_values"))
        report.check(entities.set_combo(session, "Type", "Entity"), "field changes to Entity")
        report.check(entities.set_combo(session, "Referenced entity", "Global"), "reference selects Global")
        report.check(entities.set_combo(session, "Relationship type", "many_to_many"),
                     "field changes to a to-many relationship")
        navigate.click(session, tree.wait_for_node(session, label="List model"))
        save(session)
        field = tree.wait_for_node(session, role="TextInput", label="List model displayed field")
        edit_and_save(session, report, manifest, "List model displayed field", field.get("value", ""),
                      "pending_display", field_path=("entities", 0, "fields", 0, "list_model_displayed_field"))

        feature_index = next(i for i, f in enumerate(read_manifest(manifest)["features"]) if f["name"] == "handling_manifest")
        base = ("features", feature_index)
        use_case = base + ("use_cases", 0)
        fixture.go_to(session, "Features")
        features.click_row(session, "handling_manifest", features.FEATURE_COLUMN)
        edit_and_save(session, report, manifest, "Name", "handling_manifest", "pending_feature", field_path=base + ("name",))
        features.click_row(session, "load", features.USE_CASE_COLUMN)
        edit_and_save(session, report, manifest, "Name", "load", "pending_use_case", field_path=use_case + ("name",))
        edit_and_save(session, report, manifest, "Name", "LoadDto", "PendingInputDto", field_path=use_case + ("dto_in", "name"))
        # Input DTO fields are in the detail pane, underneath the DTO name.
        row = tree.find(session, role="ListBoxOption", label="manifest_path")
        if row is None:
            raise RuntimeError("missing input DTO manifest_path field")
        navigate.click(session, row)
        edit_and_save(session, report, manifest, "Name", "manifest_path", "pending_dto_field", field_path=use_case + ("dto_in", "fields", 0, "name"))
        report.check(entities.set_combo(session, "Type", "Enum"), "DTO field can be changed to Enum")
        save(session)
        old = tree.find(session, role="TextInput", label="Enum name").get("value", "")
        edit_and_save(session, report, manifest, "Enum name", old, "PendingDtoEnum", field_path=use_case + ("dto_in", "fields", 0, "enum_name"))
        edit_and_save(session, report, manifest, None, "", "PendingDtoVariant",
                      role="MultilineTextInput", field_path=use_case + ("dto_in", "fields", 0, "enum_values"))
        navigate.click(session, tree.wait_for_node(session, label="Output DTO"))
        old = next(n["value"] for n in tree.find_all(session, role="TextInput", label="Name") if n.get("value", "").endswith("Dto"))
        edit_and_save(session, report, manifest, "Name", old, "PendingOutputDto", field_path=use_case + ("dto_out", "name"))
        row = tree.wait_for_node(session, role="ListBoxOption", label="workspace_id", timeout=5)
        if row is None:
            raise AssertionError("saving an output DTO must preserve its tab and fields")
        navigate.click(session, row)
        edit_and_save(session, report, manifest, "Name", "workspace_id", "pending_output_field", field_path=use_case + ("dto_out", "fields", 0, "name"))
    except AssertionError as exc:
        report.check(False, str(exc))
    except Exception as exc:
        report.error(f"{type(exc).__name__}: {exc}")
    finally:
        if session:
            session.close()
        if app:
            if report.exit_code:
                report.note(app.log_tail())
            app.terminate()
    return report.finish()


if __name__ == "__main__":
    sys.exit(main())
