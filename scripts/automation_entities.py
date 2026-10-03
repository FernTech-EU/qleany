#!/usr/bin/env python3
"""Probe: the Entities screen.

Covers US-ENT-01 (row subtitles), US-ENT-02 (add), US-ENT-03 (delete through the
row menu), US-ENT-04 (name validation), US-ENT-05 (heritage-only hides what it
rules out), US-ENT-06 (the parent combo), US-FLD-01 (add a field and its
subtitle), US-FLD-04/05/06/09 (the field form adapts to the type and the
relationship) and US-ENT-09 (the Mermaid export).

The reorder and undo stories are not here: a drag is driven through the data
view's own protocol rather than through an accessibility action, and undo has no
UI yet. Both are pinned by unit tests in the meantime, and this file is where
they belong once they have one.
"""

import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture
from teksilo_probe import Report, launch_and_attach, navigate, shot, tree

STORIES = (
    "US-ENT-01", "US-ENT-02", "US-ENT-03", "US-ENT-04", "US-ENT-05", "US-ENT-06",
    "US-ENT-09", "US-FLD-01", "US-FLD-04", "US-FLD-05", "US-FLD-06", "US-FLD-09",
)
PROBE = "entities"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())

SETTLE = {"settle_timeout_ms": 3000}


#: Where each column starts. Three lists on one screen all publish
#: `ListBoxOption`, and a combo's dropdown publishes them too, so a query that
#: asks the whole tree for options gets all of them at once. Every helper below
#: is scoped by geometry instead, which is also how a user tells them apart.
ENTITY_COLUMN = (0, 430)
FIELD_COLUMN = (430, 900)


def in_column(node, column):
    x = node.get("bounds", {}).get("x")
    return x is not None and column[0] <= x < column[1]


def rows(session, column=ENTITY_COLUMN):
    """The rows of one list, in order, excluding any open dropdown."""
    listed = [
        n for n in tree.nodes(session)
        if n.get("role") == "ListBoxOption" and in_column(n, column)
    ]
    return sorted(listed, key=lambda n: n.get("bounds", {}).get("y", 0))


def dropdown_options(session, node):
    """Open a combo and read what it offers, then close it again.

    Scoped by what *appeared*: this screen has three lists whose rows all publish
    `ListBoxOption`, so the dropdown is identified as the options that were not
    there a moment ago rather than by role or by position.
    """
    before = {n["id"] for n in tree.nodes(session) if n.get("role") == "ListBoxOption"}
    navigate.click(session, node, settle=False)
    session.settle(**SETTLE)
    fresh = [
        n for n in tree.nodes(session)
        if n.get("role") == "ListBoxOption" and n["id"] not in before and n.get("label")
    ]
    offered = [
        n.get("label")
        for n in sorted(fresh, key=lambda n: n.get("bounds", {}).get("y", 0))
    ]
    session.tools.inject_key(key="Escape")
    session.settle(**SETTLE)
    return offered


def statuses(session):
    return [n.get("label") for n in tree.nodes(session) if n.get("role") == "Status"]


def form_labels(session):
    """Every label the two forms show, which is how the visibility rules are read."""
    return [n.get("value") for n in tree.nodes(session) if n.get("role") == "Label"]


def texts(session):
    """Every string on screen, labels and rendered runs alike.

    A list row's subtitle is a text run rather than a node value: it is part of
    the row's rendering, not a separate semantic node.
    """
    out = []
    for n in tree.nodes(session):
        for key in ("label", "value", "description"):
            if n.get(key):
                out.append(n[key])
    return out


def combo(session, label):
    for n in tree.nodes(session):
        if n.get("role") == "ComboBox" and n.get("label") == label:
            return n
    return None


def text_input(session, label, value=None):
    for n in tree.nodes(session):
        if n.get("role") == "TextInput" and n.get("label") == label:
            if value is None or n.get("value") == value:
                return n
    return None


def set_combo(session, label, wanted, limit=24):
    """Walk a combo to the value asked for, with the keyboard.

    One step at a time, re-finding the combo each time. A list-box option
    publishes no click action, so the keyboard is the only way in; and each change
    rebuilds the form around it, which destroys the node that had focus, so a
    burst of arrow keys sent to one node only moves once. Enter commits the
    highlighted option before the next iteration re-finds the rebuilt combo.

    The direction flips when a key stops changing anything: a combo does not wrap
    at either end, so walking down from a value below the target never arrives.
    """
    key = "ArrowDown"
    last = None
    for _ in range(limit):
        node = combo(session, label)
        if node is None:
            return False
        value = node.get("value")
        if value == wanted:
            return True
        if value == last:
            # That key moved nothing: this end of the list. Turn around.
            key = "ArrowUp" if key == "ArrowDown" else "ArrowDown"
        last = value
        session.tools.focus_node(node=node["id"])
        session.tools.inject_key(key=key)
        session.tools.inject_key(key="Enter")
        session.settle(**SETTLE)
    node = combo(session, label)
    return node is not None and node.get("value") == wanted


def main():
    checks = Report(PROBE)
    app = session = None
    try:
        manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), PROBE)
        workdir = os.path.dirname(manifest)

        env = fixture.isolated_config(PROBE)
        env["QLEANY_DEV"] = "1"

        app, session = launch_and_attach(
            argv=[fixture.app_binary()],
            cwd=workdir,
            env=env, label=f"qleany-{PROBE}",
        )
        if tree.wait_for_node(session, label="Home", timeout=30) is None:
            raise RuntimeError("the app never rendered its navigation rail")
        session.settle()

        navigate.click(session, tree.wait_for_node(session, label="Open Qleany manifest", timeout=20), settle=False)
        session.settle(**SETTLE)
        fixture.go_to(session, "Entities", settle=False)
        session.settle(**SETTLE)

        # US-ENT-01: every entity, in manifest order.
        names = [n.get("label") for n in rows(session)]
        checks.check(
            names[:3] == ["EntityBase", "Root", "Workspace"],
            f"US-ENT-01 the list is in manifest order, got {names[:3]}",
        )
        checks.check(
            len(names) == 14,
            f"US-ENT-01 every entity is listed, got {len(names)}",
        )

        # The subtitle is the one thing the generated row cannot carry, because
        # `inherits_from` is a reference rather than a scalar.
        subtitles = texts(session)
        checks.check(
            "abstract" in subtitles,
            "US-ENT-01 a heritage-only entity says so",
        )
        checks.check(
            "extends EntityBase" in subtitles,
            "US-ENT-01 an entity that inherits names its parent",
        )

        shot.save(session, os.path.join(SHOT_DIR, "qleany-entities.png"))

        # US-ENT-02: add appends and selects.
        navigate.click(session, tree.wait_for_node(session, label="Add entity", timeout=5), settle=False)
        session.settle(**SETTLE)
        names = [n.get("label") for n in rows(session)]
        checks.check(
            names[-1] == "NewEntity",
            f"US-ENT-02 the new entity is appended, got {names[-1:]!r}",
        )
        checks.check(
            text_input(session, "Name", "NewEntity") is not None,
            "US-ENT-02 the new entity is selected and its form is showing",
        )

        # US-ENT-07: a concrete entity offers both flags.
        checks.check(
            "Single model" in form_labels(session) and "Undoable" in form_labels(session),
            "US-ENT-07 a concrete entity offers Single model and Undoable",
        )

        # US-ENT-06: the parent combo is None plus the heritage-only entities.
        parent = combo(session, "Inherits from")
        checks.check(parent is not None, "US-ENT-06 there is a parent combo")
        if parent is not None:
            offered = dropdown_options(session, parent)
            checks.check(
                offered[:2] == ["None", "EntityBase"],
                f"US-ENT-06 the combo offers None then the abstract entities, got {offered[:3]}",
            )
            checks.check(
                "Root" not in offered,
                "US-ENT-06 a concrete entity is not offered as a parent",
            )

        # US-ENT-04: the name is validated as it is typed.
        name = text_input(session, "Name", "NewEntity")
        session.tools.set_value(node=name["id"], value="")
        session.settle(**SETTLE)
        checks.check(
            "Entity name is required" in statuses(session),
            "US-ENT-04 an empty entity name says so",
        )
        name = text_input(session, "Name")
        session.tools.set_value(node=name["id"], value="not pascal")
        session.settle(**SETTLE)
        checks.check(
            "Entity name must be in PascalCase" in statuses(session),
            "US-ENT-04 a badly cased entity name says so, and differently",
        )
        name = text_input(session, "Name")
        session.tools.focus_node(node=name["id"])
        session.tools.set_value(node=name["id"], value="ProbedEntity")
        session.tools.inject_key(key="Enter")
        session.settle(**SETTLE)
        checks.check(
            "ProbedEntity" in [n.get("label") for n in rows(session)],
            "US-ENT-04 the committed name reaches the list",
        )

        # US-ENT-05: heritage-only takes three rows away in one step.
        heritage = next(
            (n for n in tree.nodes(session)
             if n.get("role") == "CheckBox" and n.get("label") == "Only for heritage"),
            None,
        )
        checks.check(heritage is not None, "US-ENT-05 there is a heritage checkbox")
        if heritage is not None:
            navigate.click(session, heritage, settle=False)
            session.settle(**SETTLE)
            after = form_labels(session)
            for gone in ("Inherits from", "Single model", "Undoable"):
                checks.check(
                    gone not in after,
                    f"US-ENT-05 {gone!r} is gone for a heritage-only entity",
                )
            checks.check(
                "abstract" in texts(session),
                "US-ENT-05 the row subtitle reads abstract",
            )
            # And back again, so the rest of the probe has a concrete entity.
            heritage = next(
                (n for n in tree.nodes(session)
                 if n.get("role") == "CheckBox" and n.get("label") == "Only for heritage"),
                None,
            )
            navigate.click(session, heritage, settle=False)
            session.settle(**SETTLE)
            checks.check(
                "Undoable" in form_labels(session),
                "US-ENT-05 the rows come back when it is concrete again",
            )

        # US-FLD-01: a field, with the subtitle its type asks for.
        navigate.click(session, tree.wait_for_node(session, label="Add field", timeout=5), settle=False)
        session.settle(**SETTLE)
        checks.check(
            "new_field" in [n.get("label") for n in rows(session, FIELD_COLUMN)],
            "US-FLD-01 the new field is appended",
        )
        checks.check(
            "String" in form_labels(session) or combo(session, "Type") is not None,
            "US-FLD-01 the field form opens on the new field",
        )

        # US-FLD-04/05/09: switching to Entity reveals what an Entity field has, and
        # takes away what it does not.
        type_combo = combo(session, "Type")
        checks.check(type_combo is not None, "US-FLD-04 there is a type combo")
        if type_combo is not None:
            offered = dropdown_options(session, type_combo)
            expected = ["Boolean", "Integer", "UInteger", "Float", "String",
                        "Uuid", "DateTime", "Entity", "Enum"]
            checks.check(
                offered == expected,
                f"US-FLD-04 the nine types are offered in order, got {offered}",
            )
            checks.check(
                set_combo(session, "Type", "Entity"),
                "US-FLD-04 the type changes",
            )

        after = form_labels(session)
        checks.check(
            "Referenced entity" in after and "Relationship type" in after,
            "US-FLD-05 an Entity field asks which entity and which relationship",
        )
        checks.check(
            "List" not in after,
            "US-FLD-09 an Entity field has no list flag: plurality is the relationship",
        )
        checks.check(
            "Strong (cascade delete)" in after,
            "US-FLD-06 a one-to-one Entity field can cascade",
        )
        checks.check(
            "Optional" in after,
            "US-FLD-09 a to-one Entity field may be optional",
        )

        # US-FLD-06/09: many_to_many is the one that rules out both.
        if combo(session, "Relationship type") is not None:
            checks.check(
                set_combo(session, "Relationship type", "many_to_many"),
                "US-FLD-05 the relationship changes",
            )
            after = form_labels(session)
            checks.check(
                "Strong (cascade delete)" not in after,
                "US-FLD-06 a many-to-many cannot cascade",
            )
            checks.check(
                "Optional" not in after,
                "US-FLD-09 a to-many is never optional",
            )
            checks.check(
                "List model" in after,
                "US-FLD-07 a to-many can be a list model",
            )

        shot.save(session, os.path.join(SHOT_DIR, "qleany-entities-field.png"))

        # US-FLD-09: leaving Entity clears what only an Entity field has.
        checks.check(
            set_combo(session, "Type", "Boolean"),
            "US-FLD-04 the type can be walked back to a plain one",
        )
        after = form_labels(session)
        checks.check(
            "Referenced entity" not in after and "Relationship type" not in after,
            "US-FLD-09 leaving the Entity type takes its rows away",
        )
        checks.check(
            "List" in after,
            "US-FLD-09 and gives back the list flag",
        )

        # US-ENT-03: delete through the row menu.
        before = len(rows(session))
        menus = [n for n in tree.nodes(session)
                 if n.get("role") == "Button" and n.get("label") == "More actions"]
        checks.check(bool(menus), "US-ENT-03 every row carries an overflow button")
        if menus:
            session.tools.invoke_action(node=menus[0]["id"], action="show_context_menu")
            session.settle(**SETTLE)
            entry = tree.wait_for_node(session, label="Delete entity", timeout=3)
            checks.check(entry is not None, "US-ENT-03 the row menu offers Delete")
            if entry is not None:
                navigate.click(session, entry, settle=False)
                session.settle(**SETTLE)
                # US-SAFE-01: an entity takes its fields and relationships with it,
                # so it asks first. The question is the safety probe's subject; here
                # it is answered so the delete can be checked.
                confirm = tree.wait_for_node(session, label="Yes", timeout=3)
                checks.check(confirm is not None, "US-SAFE-01 the delete asks first")
                if confirm is not None:
                    navigate.click(session, confirm, settle=False)
                    session.settle(**SETTLE)
                after_count = len(rows(session))
                checks.check(
                    after_count < before,
                    f"US-ENT-03 the row is gone, {before} to {after_count}",
                )

        # US-ENT-09: the Mermaid export.
        export = tree.wait_for_node(session, label="Export to Mermaid", timeout=3)
        checks.check(export is not None, "US-ENT-09 there is an export button")
        if export is not None:
            navigate.click(session, export, settle=False)
            session.settle(**SETTLE)
            spoken = session.tools.pull_announcements(since_seq=0)
            entries = spoken if isinstance(spoken, list) else (spoken or {}).get(
                "announcements", []
            )
            said = " ".join(
                (e.get("message") or e.get("text") or "") if isinstance(e, dict) else str(e)
                for e in entries
            )
            checks.check(
                "mermaid" in said.lower(),
                f"US-ENT-09 the export is announced, heard {said!r}",
            )

        # The edits reach the file. Everything above could pass against a form that
        # never wrote anything.
        save = tree.wait_for_node(session, label="Save manifest", timeout=5, role="Button")
        checks.check(save is not None, "there is something to save")
        if save is not None:
            navigate.click(session, save, settle=False)
            session.settle(**SETTLE)
        with open(manifest, "r", encoding="utf-8") as fh:
            written = fh.read()
        checks.check(
            "name: ProbedEntity" in written,
            "US-ENT-04 the renamed entity is in the file",
        )
        checks.check(
            "name: new_field" in written,
            "US-FLD-01 the new field is in the file",
        )
    except Exception as exc:
        checks.error(f"{type(exc).__name__}: {exc}")
    finally:
        if session:
            session.close()
        if app:
            if checks.exit_code:
                checks.note(app.log_tail())
            app.terminate()

    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
