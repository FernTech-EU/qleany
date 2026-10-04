#!/usr/bin/env python3
"""Probe: manifest validation.

Covers US-CHK-01 (the badge and its counts), US-CHK-02 (the panel lists every
problem in the backend's own words), US-CHK-03 (it keeps up on its own and can be
forced), US-CHK-04 (the panel closes, and closing the manifest clears it) and
US-CHK-05 (a critical error puts Generate out of reach).

The critical error is made rather than mocked: clearing the application name on
the Project screen is a real rule failure, which is the only way to find out
whether the badge, the panel and the navigation rail all hear about it.
"""

import os
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture
from teksilo_probe import Report, launch_and_attach, navigate, shot, tree

STORIES = ("US-CHK-01", "US-CHK-02", "US-CHK-03", "US-CHK-04", "US-CHK-05")
PROBE = "check"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle_timeout_ms": 3000}

OK = "The manifest validates"
CRITICAL = "The manifest has errors and will not generate"


def badge(session):
    for n in tree.nodes(session):
        if n.get("role") == "Button" and n.get("label") in (
            OK,
            CRITICAL,
            "The manifest has warnings",
        ):
            return n
    return None


def texts(session):
    out = []
    for n in tree.nodes(session):
        for key in ("label", "value", "description"):
            if n.get(key):
                out.append(n[key])
    return out


def nav_row(session, label):
    for n in tree.nodes(session):
        if n.get("role") == "Button" and n.get("label") == label and n.get("bounds", {}).get("x", 99) < 172:
            return n
    return None



def panel_close(session):
    """Find Close under the validation heading's ancestor, not the title bar."""
    nodes = tree.nodes(session)
    heading = tree.find(nodes, pred=lambda n: "Manifest validation" in (n.get("label"), n.get("value")))
    parents = {child: n for n in nodes for child in n.get("children", [])}
    while heading is not None:
        close = tree.find(tree.descendants(nodes, heading), role="Button", label="Close")
        if close is not None:
            return close
        heading = parents.get(heading["id"])
    return None


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

        navigate.click(session, tree.wait_for_node(session, label="Open Qleany manifest", timeout=20), settle=False)
        session.settle(**SETTLE)
        # Validation consumes manifest events on subsequent frames. Its initial
        # unchecked badge has the same label as OK, but will be rebuilt.
        time.sleep(0.4)
        session.settle(**SETTLE)

        # US-CHK-01: Qleany's own manifest validates, so the badge says so.
        mark = badge(session)
        checks.check(mark is not None, "US-CHK-01 the badge is on screen")
        checks.check(
            mark is not None and mark.get("label") == OK,
            f"US-CHK-01 a valid manifest reads as valid, got {mark and mark.get('label')!r}",
        )

        # US-CHK-05: and Generate is reachable.
        generate = nav_row(session, "Generate")
        checks.check(
            generate is not None and not generate.get("disabled", False),
            "US-CHK-05 Generate is reachable while the manifest validates",
        )

        # US-CHK-02: the panel opens and says so in the backend's own words.
        x, y = tree.center(badge(session))
        session.tools.inject_pointer(x=x, y=y, action="click")
        session.settle(**SETTLE)
        opened = tree.wait_for(session, "Manifest validation", timeout=5)
        shown = texts(session)
        checks.check(opened, "US-CHK-02 the panel opens")
        checks.check(
            "This manifest validates." in shown,
            "US-CHK-02 and reports a clean manifest",
        )
        shot.save(session, os.path.join(SHOT_DIR, "qleany-check.png"))

        # US-CHK-04: it closes again.
        close = panel_close(session)
        checks.check(close is not None, "US-CHK-04 the panel has its own Close button")
        if close is not None:
            navigate.click(session, close, settle=False)
            session.settle(**SETTLE)
            # Like the dialog example, allow the dismiss animation to finish
            # before the next key event targets the form behind it.
            time.sleep(0.4)
            session.settle(**SETTLE)
        checks.check(
            "Manifest validation" not in texts(session),
            "US-CHK-04 the panel closes",
        )

        # US-CHK-03 and 05: break a rule for real, and watch every consumer hear it.
        fixture.go_to(session, "Project", settle=False)
        session.settle(**SETTLE)
        name = next(
            n for n in tree.nodes(session)
            if n.get("role") == "TextInput" and n.get("label") == "Application name"
        )
        session.tools.focus_node(node=name["id"])
        session.tools.set_value(node=name["id"], value="")
        session.tools.inject_key(key="Enter")
        session.settle(settle_timeout_ms=4000)

        mark = tree.wait_for_node(session, role="Button", label=CRITICAL, timeout=5)
        checks.check(
            mark is not None and mark.get("label") == CRITICAL,
            f"US-CHK-03 an edit re-runs the check, got {mark and mark.get('label')!r}",
        )

        generate = nav_row(session, "Generate")
        checks.check(
            generate is not None and generate.get("disabled", False),
            "US-CHK-05 a critical error puts Generate out of reach",
        )
        entities = nav_row(session, "Entities")
        checks.check(
            entities is not None and not entities.get("disabled", False),
            "US-CHK-05 and leaves reachable the screens that let it be fixed",
        )

        # US-CHK-02: the panel names the rule that failed.
        x, y = tree.center(badge(session))
        session.tools.inject_pointer(x=x, y=y, action="click")
        session.settle(**SETTLE)
        shown = texts(session)
        checks.check(
            any("application_name is empty" in t for t in shown),
            "US-CHK-02 the panel carries the backend's own sentence",
        )
        shot.save(session, os.path.join(SHOT_DIR, "qleany-check-critical.png"))

        # Dismiss the popover before directing keyboard input to the form.
        session.tools.inject_key(key="Escape")
        session.settle(**SETTLE)

        # US-CHK-03: and it recovers when the rule is satisfied again.
        name = next(
            n for n in tree.nodes(session)
            if n.get("role") == "TextInput" and n.get("label") == "Application name"
        )
        session.tools.focus_node(node=name["id"])
        session.tools.set_value(node=name["id"], value="Qleany")
        session.tools.inject_key(key="Enter")
        session.settle(settle_timeout_ms=4000)
        mark = tree.wait_for_node(session, role="Button", label=OK, timeout=5)
        checks.check(
            mark is not None and mark.get("label") == OK,
            f"US-CHK-03 fixing the rule clears the badge, got {mark and mark.get('label')!r}",
        )

        # US-CHK-04: closing the manifest clears the badge with everything else.
        fixture.go_to(session, "Home", settle=False)
        session.settle(**SETTLE)
        navigate.click(session, tree.wait_for_node(session, label="Close current manifest", timeout=5), settle=False)
        session.settle(settle_timeout_ms=4000)
        checks.check(
            "Manifest validation" not in texts(session),
            "US-CHK-04 closing the manifest closes the panel",
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
