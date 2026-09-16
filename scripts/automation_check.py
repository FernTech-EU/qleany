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
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = ("US-CHK-01", "US-CHK-02", "US-CHK-03", "US-CHK-04", "US-CHK-05")
PROBE = "check"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle": {"settle_timeout_ms": 3000}}

OK = "The manifest validates"
CRITICAL = "The manifest has errors and will not generate"


def badge(session):
    for n in session.nodes():
        if n.get("role") == "Button" and n.get("label") in (
            OK,
            CRITICAL,
            "The manifest has warnings",
        ):
            return n
    return None


def texts(session):
    out = []
    for n in session.nodes():
        for key in ("label", "value", "description"):
            if n.get(key):
                out.append(n[key])
    return out


def nav_row(session, label):
    for n in session.nodes():
        if n.get("role") == "Button" and n.get("label") == label and n.get("bounds", {}).get("x", 99) < 172:
            return n
    return None


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
        session.click(mark)
        session.call("settle", SETTLE)
        shown = texts(session)
        checks.check("Manifest validation" in shown, "US-CHK-02 the panel opens")
        checks.check(
            "This manifest validates." in shown,
            "US-CHK-02 and reports a clean manifest",
        )
        session.shot(os.path.join(SHOT_DIR, "qleany-check.png"))

        # US-CHK-04: it closes again.
        close = next(
            (n for n in session.nodes()
             if n.get("role") == "Button" and n.get("label") == "Close"
             and n.get("bounds", {}).get("x", 0) < 1200),
            None,
        )
        if close is not None:
            session.click(close)
            session.call("settle", SETTLE)
        checks.check(
            "Manifest validation" not in texts(session),
            "US-CHK-04 the panel closes",
        )

        # US-CHK-03 and 05: break a rule for real, and watch every consumer hear it.
        session.click(session.find("Project", timeout=5))
        session.call("settle", SETTLE)
        name = next(
            n for n in session.nodes()
            if n.get("role") == "TextInput" and n.get("label") == "Application name"
        )
        session.call("focus_node", {"node": name["id"]})
        session.call("set_value", {"node": name["id"], "value": ""})
        session.call("inject_key", {"key": "Enter"})
        session.call("settle", {"settle": {"settle_timeout_ms": 4000}})

        mark = badge(session)
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
        session.click(mark)
        session.call("settle", SETTLE)
        shown = texts(session)
        checks.check(
            any("application_name is empty" in t for t in shown),
            "US-CHK-02 the panel carries the backend's own sentence",
        )
        session.shot(os.path.join(SHOT_DIR, "qleany-check-critical.png"))

        # US-CHK-03: and it recovers when the rule is satisfied again.
        name = next(
            n for n in session.nodes()
            if n.get("role") == "TextInput" and n.get("label") == "Application name"
        )
        session.call("focus_node", {"node": name["id"]})
        session.call("set_value", {"node": name["id"], "value": "Qleany"})
        session.call("inject_key", {"key": "Enter"})
        session.call("settle", {"settle": {"settle_timeout_ms": 4000}})
        mark = badge(session)
        checks.check(
            mark is not None and mark.get("label") == OK,
            f"US-CHK-03 fixing the rule clears the badge, got {mark and mark.get('label')!r}",
        )

        # US-CHK-04: closing the manifest clears the badge with everything else.
        session.click(session.find("Home", timeout=5))
        session.call("settle", SETTLE)
        session.click(session.find("Close current manifest", timeout=5))
        session.call("settle", {"settle": {"settle_timeout_ms": 4000}})
        checks.check(
            "Manifest validation" not in texts(session),
            "US-CHK-04 closing the manifest closes the panel",
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
