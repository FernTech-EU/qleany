#!/usr/bin/env python3
"""Probe: the manifest lifecycle.

Covers US-MAN-04 (close clears the workspace and locks the rail), US-SHELL-01 (the
title names the open manifest), US-SHELL-03 (the rail unlocks once a manifest is
open) and US-HOME-01/03 (Home offers every manifest action, and the developer
block appears only in developer mode).

Loads through the developer affordance rather than the native file picker: a
probe cannot drive an OS dialog, and the affordance loads `qleany.yaml` from the
working directory, which is what this runs in.
"""

import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = ("US-MAN-04", "US-SHELL-01", "US-SHELL-03", "US-HOME-01", "US-HOME-03")
PROBE = "manifest"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())

LOCKED = ("Project", "Entities", "Features", "User Interface", "Generate")


def main():
    checks = fixture.Checks()
    log = os.path.join(fixture.SCRATCH, f"qleany-{PROBE}-{os.getpid()}.log")

    # A throwaway copy, in its own directory, so loading it and any later save
    # can never touch the checked-in manifest.
    manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), PROBE)
    workdir = os.path.dirname(manifest)

    env = fixture.isolated_config(PROBE)
    env["QLEANY_DEV"] = "1"

    app = subprocess.Popen(
        [fixture.app_binary()],
        cwd=workdir,
        stdout=open(log, "w"),
        stderr=subprocess.STDOUT,
        env=env,
    )
    session = None
    try:
        bridge = fixture.wait_for_bridge(log, app, timeout=60)
        session = fixture.Session(bridge)

        # US-HOME-01: every manifest action is on Home.
        for label in ("New manifest", "Open manifest", "Save manifest",
                      "Save manifest as…", "Close current manifest", "Run demo"):
            checks.check(
                session.find(label, timeout=3) is not None,
                f"US-HOME-01 Home offers {label!r}",
            )

        # US-HOME-03: developer mode is on, so the test block shows.
        checks.check(
            session.find("Open Qleany manifest", timeout=3) is not None,
            "US-HOME-03 the developer block shows under QLEANY_DEV=1",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-home.png"))

        # Load it.
        dev = session.find("Open Qleany manifest", timeout=3)
        session.click(dev)
        session.call("settle", {"settle": {"settle_timeout_ms": 2000}})

        # US-SHELL-03: the rail unlocks.
        for row in LOCKED:
            node = session.find(row, timeout=5)
            checks.check(
                node is not None and not node.get("disabled", False),
                f"US-SHELL-03 {row!r} is reachable once a manifest is open",
            )

        # US-SHELL-01: the title names it.
        # The title is a bound text prop, so it reads as the node's `value`; the
        # `label` of a TextWidget is its constructor argument.
        values = [n.get("value") for n in session.nodes()]
        checks.check(
            any(v and v.startswith("Qleany - ") and "qleany.yaml" in v for v in values),
            "US-SHELL-01 the title names the open manifest",
        )

        # US-MAN-06: a fresh load is not dirty. The load publishes a Created event
        # per entity it reads, each of which marks the manifest dirty, so this is
        # the assertion that the Load event gets the last word.
        save = session.find("No unsaved changes", timeout=3)
        checks.check(
            save is not None,
            "US-MAN-06 a freshly loaded manifest is not dirty",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-loaded.png"))

        # US-MAN-04: close locks it all again.
        close = session.find("Close current manifest", timeout=3)
        session.click(close)
        session.call("settle", {"settle": {"settle_timeout_ms": 2000}})
        for row in LOCKED:
            node = session.find(row, timeout=3)
            checks.check(
                node is not None and node.get("disabled", False),
                f"US-MAN-04 {row!r} locks again after close",
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
