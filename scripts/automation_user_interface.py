#!/usr/bin/env python3
"""Probe: the User Interface screen.

Covers US-UIT-01: the frontends on offer are the ones the project's language has,
each persists, and changing the language swaps the section rather than leaving
the previous language's targets on screen.

The language lives on the Project screen, so this probe crosses between the two.
That is the point of it: the two screens share one signal, and the way to find
out whether they really do is to change it on one and look at the other.
"""

import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = ("US-UIT-01",)
PROBE = "user_interface"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle": {"settle_timeout_ms": 3000}}

RUST_TARGETS = ["CLI", "Teksilo (recommended)", "Slint", "iOS (UniFFI)", "Android (UniFFI)"]
CPP_TARGETS = ["Qt Widgets", "Qt Quick"]


def checkboxes(session):
    return {
        n.get("label"): n
        for n in session.nodes()
        if n.get("role") == "CheckBox"
    }


def ui_block(path):
    """The manifest's `ui:` block, and nothing else.

    The same names appear elsewhere in the file as the fields of the
    `UserInterface` entity, so a search of the whole document finds them whether
    or not any target is switched on.
    """
    lines = []
    inside = False
    with open(path, "r", encoding="utf-8") as fh:
        for line in fh:
            if line.startswith("ui:"):
                inside = True
                continue
            if inside:
                if line.strip() and not line.startswith((" ", "\t")):
                    break
                lines.append(line.strip())
    return "\n".join(l for l in lines if l)


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
        go_to(session, "User Interface")

        boxes = checkboxes(session)
        checks.check(
            list(boxes) == RUST_TARGETS,
            f"US-UIT-01 a Rust manifest offers the Rust targets, got {list(boxes)}",
        )

        # Qleany's own manifest: CLI, Teksilo and Slint on, the two mobile targets
        # off. Reading them is what proves the flags persist rather than defaulting.
        for target, expected in (
            ("CLI", "true"),
            ("Teksilo (recommended)", "true"),
            ("Slint", "true"),
            ("iOS (UniFFI)", "false"),
            ("Android (UniFFI)", "false"),
        ):
            node = boxes.get(target)
            checks.check(
                node is not None and node.get("toggled") == expected,
                f"US-UIT-01 {target!r} reads {expected} from the manifest",
            )

        session.shot(os.path.join(SHOT_DIR, "qleany-user-interface.png"))

        # A flag can be turned off, and it stays off across a visit to another
        # screen: this is the whole write path, through a single that has to be
        # re-pointed when the screen is rebuilt.
        session.click(boxes["Slint"])
        session.call("settle", SETTLE)
        go_to(session, "Home")
        go_to(session, "User Interface")
        checks.check(
            checkboxes(session)["Slint"].get("toggled") == "false",
            "US-UIT-01 a target that was turned off stays off",
        )

        # Changing the language swaps the section clean.
        go_to(session, "Project")
        language = next(
            n for n in session.nodes()
            if n.get("role") == "ComboBox" and n.get("label") == "Language"
        )
        session.call("focus_node", {"node": language["id"]})
        session.call("inject_key", {"key": "ArrowDown"})
        session.call("inject_key", {"key": "Enter"})
        session.call("settle", SETTLE)
        go_to(session, "User Interface")

        boxes = checkboxes(session)
        checks.check(
            list(boxes) == CPP_TARGETS,
            f"US-UIT-01 a C++/Qt manifest offers the Qt targets, got {list(boxes)}",
        )
        checks.check(
            not any(t in boxes for t in RUST_TARGETS),
            "US-UIT-01 no Rust target survives the swap",
        )
        checks.check(
            "C++ / Qt user interfaces" in
            [n.get("value") for n in session.nodes() if n.get("role") == "Label"],
            "US-UIT-01 the section is renamed with its targets",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-user-interface-cpp.png"))

        # Turn a Qt target on, save, and read the file: the form could show all of
        # this and write none of it.
        session.click(boxes["Qt Quick"])
        session.call("settle", SETTLE)
        save = session.find("Save manifest", timeout=5, role="Button")
        checks.check(save is not None, "there is something to save")
        if save is not None:
            session.click(save)
            session.call("settle", SETTLE)

        block = ui_block(manifest)
        checks.check(
            "cpp_qt_qtquick: true" in block,
            f"US-UIT-01 a target turned on reaches the file, block is {block!r}",
        )
        # A flag that is off is left out of the block rather than written as false,
        # which is why this looks for an absence rather than for `rust_slint: false`.
        # Searching the whole file would find the entity field of that name.
        checks.check(
            "rust_slint" not in block,
            f"US-UIT-01 and one turned off reaches it too, block is {block!r}",
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
