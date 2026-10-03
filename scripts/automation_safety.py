#!/usr/bin/env python3
"""Probe: the About box, and the confirmations before something is lost.

Covers US-ABOUT-01 (Help, About shows what this build is and closes on Escape),
US-SAFE-01 (a delete that cascades asks first, one that does not deletes at once)
US-SAFE-02 (a confirmation names the object by its own name and says what goes
with it) and US-SAFE-03 (every path that replaces or closes the open manifest
asks before discarding unsaved work).

The quit guard is not driven here: answering it closes the window, which ends the
session the probe is driving. Its two halves are covered instead by the shell
probe, which reads the menu row, and by the dirty flag the guard consults.
"""

import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture
from teksilo_probe import Report, launch_and_attach, navigate, shot, tree

STORIES = ("US-ABOUT-01", "US-SAFE-01", "US-SAFE-02", "US-SAFE-03")
PROBE = "safety"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle_timeout_ms": 3000}


def unsaved(session):
    """Whether the title bar says there is work that is not on disk."""
    return "No unsaved changes" not in [n.get("label") for n in tree.nodes(session)]


def go_home(session):
    fixture.go_to(session, "Home", settle=False)
    session.settle(**SETTLE)


def close_discarding(session):
    """Close whatever is open, answering the guard if it asks."""
    go_home(session)
    close = button(session, "Close current manifest")
    if close is None or close.get("disabled", False):
        return
    navigate.click(session, close, settle=False)
    session.settle(**SETTLE)
    discard = tree.wait_for_node(session, label="Discard", timeout=3)
    if discard is not None:
        navigate.click(session, discard, settle=False)
        session.settle(**SETTLE)


def dirty_the_manifest(session):
    """Open Qleany's own manifest and change one field, committed with Enter.

    The Project form commits on Enter or blur rather than per keystroke, so a
    `set_value` alone leaves the store untouched and the manifest clean.
    """
    close_discarding(session)
    navigate.click(session, tree.wait_for_node(session, label="Open Qleany manifest", timeout=10), settle=False)
    session.settle(**SETTLE)
    fixture.go_to(session, "Project", settle=False)
    session.settle(**SETTLE)
    field = tree.wait_for_node(session, label="Application name", timeout=10, role="TextInput")
    session.tools.set_value(node=field["id"], value="DirtiedByProbe")
    session.tools.focus_node(node=field["id"])
    session.tools.inject_key(key="Enter")
    session.settle(**SETTLE)
    go_home(session)


def texts(session):
    out = []
    for n in tree.nodes(session):
        for key in ("label", "value", "description"):
            if n.get(key):
                out.append(n[key])
    return out


def centre_y(node):
    bounds = node.get("bounds", {})
    return (bounds.get("y") or 0) + (bounds.get("height") or 0) / 2


def menu_item(session, label):
    """A menu row, by its label without the keyboard mnemonic."""
    for n in tree.nodes(session):
        if n.get("role") == "MenuItem" and (n.get("label") or "").replace("&", "") == label:
            return n
    return None


def button(session, label):
    for n in tree.nodes(session):
        if n.get("role") == "Button" and n.get("label") == label:
            return n
    return None


def entity_rows(session):
    listed = [
        n for n in tree.nodes(session)
        if n.get("role") == "ListBoxOption" and (n.get("bounds", {}).get("x") or 99) < 430
    ]
    return [
        n.get("label")
        for n in sorted(listed, key=lambda n: n.get("bounds", {}).get("y", 0))
    ]


def field_rows(session):
    listed = [
        n for n in tree.nodes(session)
        if n.get("role") == "ListBoxOption" and 430 <= (n.get("bounds", {}).get("x") or -1) < 900
    ]
    return [
        n.get("label")
        for n in sorted(listed, key=lambda n: n.get("bounds", {}).get("y", 0))
    ]


def open_row_menu(session, row_label, column_max_x):
    """Open one row's overflow menu, by the row's own position."""
    row = next(
        (n for n in tree.nodes(session)
         if n.get("role") == "ListBoxOption" and n.get("label") == row_label
         and (n.get("bounds", {}).get("x") or 999) < column_max_x),
        None,
    )
    if row is None:
        return False
    # The button's centre has to fall *inside* the row, not merely within a row's
    # height of its top: rows are stacked at exactly that pitch, so a tolerance of
    # one row height matches the row above as well, and `next` takes whichever comes
    # first in the tree. That is how a delete confirmation ended up naming the row
    # above the one whose menu was opened.
    top = row["bounds"]["y"]
    bottom = top + row["bounds"]["height"]
    # Constrained to the row's own column as well as its own vertical span: two
    # lists sit side by side, their rows line up, and an overflow button matched by
    # height alone is as likely to be the neighbouring list's.
    left = row["bounds"]["x"]
    right = left + row["bounds"]["width"]
    overflow = next(
        (n for n in tree.nodes(session)
         if n.get("role") == "Button" and n.get("label") == "More actions"
         and top <= centre_y(n) < bottom
         and left <= (n.get("bounds", {}).get("x") or -1) < right),
        None,
    )
    if overflow is None:
        return False
    session.tools.invoke_action(node=overflow["id"], action="show_context_menu")
    session.settle(**SETTLE)
    return True


def main():
    checks = Report(PROBE)
    app = session = None
    try:
        manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), PROBE)
        env = fixture.isolated_config(PROBE)
        env["QLEANY_DEV"] = "1"

        app, session = launch_and_attach(
            argv=[fixture.app_binary()],
            cwd=os.path.dirname(manifest),
            env=env, label=f"qleany-{PROBE}",
        )
        if tree.wait_for_node(session, label="Home", timeout=30) is None:
            raise RuntimeError("the app never rendered its navigation rail")
        session.settle()

        # US-ABOUT-01: through the menu, which is where it lives. The hamburger
        # opens the bar; Help is a submenu, and About is inside it.
        navigate.click(session, button(session, "Menu"), settle=False)
        session.settle(**SETTLE)
        help_menu = menu_item(session, "Help")
        checks.check(help_menu is not None, "US-ABOUT-01 the menu bar has Help")
        if help_menu is not None:
            navigate.click(session, help_menu, settle=False)
            session.settle(**SETTLE)
        about = menu_item(session, "About Qleany")
        checks.check(about is not None, "US-ABOUT-01 Help offers About")
        if about is not None:
            navigate.click(session, about, settle=False)
            session.settle(**SETTLE)
        shown = texts(session)
        checks.check(
            any(t.startswith("Version ") for t in shown),
            f"US-ABOUT-01 About names the version",
        )
        checks.check("Made by FernTech" in shown, "US-ABOUT-01 and who made it")
        checks.check(
            any("Mozilla Public License" in t for t in shown),
            "US-ABOUT-01 and the licence",
        )
        links = [n.get("label") for n in tree.nodes(session) if n.get("role") == "Link"]
        checks.check(
            "Documentation" in links and "Repository" in links,
            f"US-ABOUT-01 and both links, got {links}",
        )
        shot.save(session, os.path.join(SHOT_DIR, "qleany-about.png"))

        session.tools.inject_key(key="Escape")
        session.settle(**SETTLE)
        checks.check(
            not any(t.startswith("Version ") for t in texts(session)),
            "US-ABOUT-01 Escape closes it",
        )

        # US-SAFE-01: a cascading delete asks first.
        navigate.click(session, tree.wait_for_node(session, label="Open Qleany manifest", timeout=10), settle=False)
        session.settle(**SETTLE)
        fixture.go_to(session, "Entities", settle=False)
        session.settle(**SETTLE)

        before = entity_rows(session)
        checks.check(open_row_menu(session, "Workspace", 430), "the row menu opens")
        delete = tree.wait_for_node(session, label="Delete entity", timeout=3)
        checks.check(delete is not None, "US-SAFE-01 the row menu offers Delete")
        if delete is not None:
            navigate.click(session, delete, settle=False)
            session.settle(**SETTLE)

        asked = texts(session)
        checks.check(
            any("Delete this entity?" in t for t in asked),
            "US-SAFE-01 an entity delete asks first",
        )
        checks.check(
            any("Workspace" in t and "fields" in t for t in asked),
            f"US-SAFE-02 and names the entity and what goes with it",
        )
        checks.check(
            not any("this item" in t for t in asked),
            "US-SAFE-02 never 'this item'",
        )
        shot.save(session, os.path.join(SHOT_DIR, "qleany-confirm.png"))

        # Declining leaves it alone.
        no = tree.wait_for_node(session, label="No", timeout=3)
        checks.check(no is not None, "US-SAFE-01 the question can be declined")
        if no is not None:
            navigate.click(session, no, settle=False)
            session.settle(**SETTLE)
        checks.check(
            entity_rows(session) == before,
            "US-SAFE-01 declining keeps the entity",
        )

        # And accepting deletes it.
        checks.check(open_row_menu(session, "Workspace", 430), "the row menu reopens")
        navigate.click(session, tree.wait_for_node(session, label="Delete entity", timeout=3), settle=False)
        session.settle(**SETTLE)
        navigate.click(session, tree.wait_for_node(session, label="Yes", timeout=3), settle=False)
        session.settle(**SETTLE)
        checks.check(
            "Workspace" not in entity_rows(session),
            "US-SAFE-01 accepting deletes it",
        )

        # US-SAFE-01: a field is one row and Undo covers it, so it goes at once.
        navigate.click(session, next(
            n for n in tree.nodes(session)
            if n.get("role") == "ListBoxOption" and n.get("label") == "Entity"
            and (n.get("bounds", {}).get("x") or 999) < 430
        ))
        session.settle(**SETTLE)
        fields_before = field_rows(session)
        checks.check(bool(fields_before), "the entity has fields")
        checks.check(
            open_row_menu(session, fields_before[0], 900),
            "the field row menu opens",
        )
        navigate.click(session, tree.wait_for_node(session, label="Delete field", timeout=3), settle=False)
        session.settle(**SETTLE)
        checks.check(
            not any("Delete this" in t for t in texts(session)),
            "US-SAFE-01 a field deletes without a question",
        )
        checks.check(
            len(field_rows(session)) == len(fields_before) - 1,
            "US-SAFE-01 and it is gone",
        )
        checks.check(
            not button(session, "Undo").get("disabled", False),
            "US-SAFE-01 with Undo covering it",
        )

        # US-SAFE-03: New, Open and Close each replace or discard the open
        # manifest. The Slint UI asked before all three; this one asked before
        # none of them until these checks were written.
        for action, question in [
            ("New manifest", "Save before starting a new manifest?"),
            ("Open manifest", "Save before opening another manifest?"),
            ("Close current manifest", "Save before closing?"),
        ]:
            dirty_the_manifest(session)
            checks.check(unsaved(session), f"{action}: there is unsaved work first")

            navigate.click(session, button(session, action), settle=False)
            session.settle(**SETTLE)
            checks.check(
                any(question in t for t in texts(session)),
                f"US-SAFE-03 {action} asks {question!r}",
            )
            cancel = tree.wait_for_node(session, label="Cancel", timeout=3)
            checks.check(cancel is not None, f"US-SAFE-03 {action} can be cancelled")
            if cancel is not None:
                navigate.click(session, cancel, settle=False)
                session.settle(**SETTLE)
            checks.check(
                unsaved(session),
                f"US-SAFE-03 cancelling {action} keeps the unsaved work",
            )

            # Discard, so the next round starts from a clean manifest.
            close_discarding(session)
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
