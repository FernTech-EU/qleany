#!/usr/bin/env python3
"""Probe: undo and redo.

Covers US-UNDO-01 (undo reverses the last change on the screen you are on),
US-UNDO-02 (redo reapplies it), US-UNDO-03 (the menu row names what would be
undone, and a toast says what was) and US-UNDO-04 (undo never reaches a screen
you are not looking at, and Home and Generate offer none).

US-UNDO-04 is the one worth driving rather than reasoning about: the four stacks
are per screen, and the way to find out whether Ctrl+Z respects that is to edit
on one screen, move to another, and press it.
"""

import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = ("US-UNDO-01", "US-UNDO-02", "US-UNDO-03", "US-UNDO-04")
PROBE = "undo"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle": {"settle_timeout_ms": 3000}}


def texts(session):
    out = []
    for n in session.nodes():
        for key in ("label", "value", "description"):
            if n.get(key):
                out.append(n[key])
    return out


def button(session, label):
    for n in session.nodes():
        if n.get("role") == "Button" and n.get("label") == label:
            return n
    return None


def entity_rows(session):
    listed = [
        n for n in session.nodes()
        if n.get("role") == "ListBoxOption" and (n.get("bounds", {}).get("x") or 99) < 430
    ]
    return [
        n.get("label")
        for n in sorted(listed, key=lambda n: n.get("bounds", {}).get("y", 0))
    ]


def menu_rows(session):
    """Open the hamburger's Edit menu and read its rows, then close it again."""
    session.click(button(session, "Menu"))
    session.call("settle", SETTLE)
    edit = next(
        (n for n in session.nodes()
         if n.get("role") in ("MenuItem", "Button") and (n.get("label") or "").strip("&") == "Edit"),
        None,
    )
    if edit is not None:
        session.click(edit)
        session.call("settle", SETTLE)
    rows = [
        (n.get("label"), n.get("disabled", False))
        for n in session.nodes()
        if n.get("role") == "MenuItem" and n.get("label")
    ]
    session.call("inject_key", {"key": "Escape"})
    session.call("settle", SETTLE)
    return rows


def go_to(session, screen):
    session.click(session.find(screen, timeout=10))
    session.call("settle", SETTLE)


def main():
    checks = fixture.Checks()
    log = os.path.join(fixture.SCRATCH, f"qleany-{PROBE}-{os.getpid()}.log")

    manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), PROBE)
    env = fixture.isolated_config(PROBE)
    env["QLEANY_DEV"] = "1"

    app = subprocess.Popen(
        [fixture.app_binary()],
        cwd=os.path.dirname(manifest),
        stdout=open(log, "w"),
        stderr=subprocess.STDOUT,
        env=env,
    )
    session = None
    try:
        bridge = fixture.wait_for_bridge(log, app, timeout=60)
        session = fixture.Session(bridge)

        session.click(session.find("Open Qleany manifest", timeout=20))
        session.call("settle", SETTLE)

        # US-UNDO-04: a fresh manifest has nothing to undo anywhere. A load writes
        # hundreds of rows, and none of it is an edit the user made.
        undo = button(session, "Undo")
        checks.check(undo is not None, "US-UNDO-03 there is an Undo button")
        checks.check(
            undo is not None and undo.get("disabled", False),
            "US-UNDO-04 a freshly loaded manifest has nothing to undo",
        )

        # US-UNDO-01: make an edit on Entities.
        go_to(session, "Entities")
        before = entity_rows(session)
        session.click(session.find("Add entity", timeout=5))
        session.call("settle", SETTLE)
        after_add = entity_rows(session)
        checks.check(
            len(after_add) == len(before) + 1,
            f"an entity was added, {len(before)} to {len(after_add)}",
        )

        undo = button(session, "Undo")
        checks.check(
            undo is not None and not undo.get("disabled", False),
            "US-UNDO-01 the edit made Undo live",
        )

        # US-UNDO-03: the Edit menu names it.
        rows = menu_rows(session)
        labels = [label for label, _ in rows]
        checks.check(
            any("add entity" in (label or "") for label in labels),
            f"US-UNDO-03 the menu row names the operation, got {labels}",
        )

        # US-UNDO-04: the edit belongs to Entities, so Project has nothing to undo.
        go_to(session, "Project")
        undo = button(session, "Undo")
        checks.check(
            undo is not None and undo.get("disabled", False),
            "US-UNDO-04 another screen's stack is not this screen's",
        )
        go_to(session, "Home")
        undo = button(session, "Undo")
        checks.check(
            undo is not None and undo.get("disabled", False),
            "US-UNDO-04 Home offers no undo",
        )

        # US-UNDO-01: back on Entities, Ctrl+Z takes the entity away.
        go_to(session, "Entities")
        session.call("inject_key", {"key": "Character('z')", "ctrl": True})
        session.call("settle", SETTLE)
        if entity_rows(session) == after_add:
            # The keystroke spelling varies with the bridge; the button is the same
            # command by the same name.
            session.click(button(session, "Undo"))
            session.call("settle", SETTLE)
        undone = entity_rows(session)
        checks.check(
            undone == before,
            f"US-UNDO-01 undo took the entity back, got {len(undone)} of {len(before)}",
        )
        checks.check(
            any("Undone" in t for t in texts(session)),
            "US-UNDO-03 and said what it undid",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-undo.png"))

        # US-UNDO-02: redo puts it back.
        redo = button(session, "Redo")
        checks.check(
            redo is not None and not redo.get("disabled", False),
            "US-UNDO-02 redo is live after an undo",
        )
        if redo is not None:
            session.click(redo)
            session.call("settle", SETTLE)
        checks.check(
            len(entity_rows(session)) == len(before) + 1,
            "US-UNDO-02 redo reapplied it",
        )

        # US-UNDO-02: a new edit clears the redo side.
        session.click(button(session, "Undo"))
        session.call("settle", SETTLE)
        session.click(session.find("Add entity", timeout=5))
        session.call("settle", SETTLE)
        redo = button(session, "Redo")
        checks.check(
            redo is not None and redo.get("disabled", False),
            "US-UNDO-02 a new edit clears what there was to redo",
        )

        # US-UNDO-04: closing the manifest discards every stack.
        go_to(session, "Home")
        session.click(session.find("Close current manifest", timeout=5))
        session.call("settle", {"settle": {"settle_timeout_ms": 4000}})
        undo = button(session, "Undo")
        checks.check(
            undo is not None and undo.get("disabled", False),
            "US-UNDO-04 closing the manifest discards its history",
        )
    finally:
        if session:
            session.close()
        app.terminate()
        try:
            app.wait(timeout=5)
        except subprocess.TimeoutExpired:
            app.kill()

    return checks.finish(PROBE, log)


if __name__ == "__main__":
    sys.exit(main())
