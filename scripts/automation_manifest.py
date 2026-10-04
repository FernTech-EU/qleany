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
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture
from teksilo_probe import Report, launch_and_attach, navigate, shot, tree

STORIES = ("US-MAN-04", "US-SHELL-01", "US-SHELL-03", "US-HOME-01", "US-HOME-03")
PROBE = "manifest"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())

LOCKED = ("Project", "Entities", "Features", "User Interface", "Generate")


def main():
    checks = Report(PROBE)
    app = session = None
    try:

        # A throwaway copy, in its own directory, so loading it and any later save
        # can never touch the checked-in manifest.
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

        # US-HOME-01: every manifest action is on Home.
        for label in ("New manifest", "Open manifest", "Save manifest",
                      "Save manifest as…", "Close current manifest", "Run demo"):
            checks.check(
                tree.wait_for_node(session, label=label, timeout=3) is not None,
                f"US-HOME-01 Home offers {label!r}",
            )

        # US-HOME-03: developer mode is on, so the test block shows.
        checks.check(
            tree.wait_for_node(session, label="Open Qleany manifest", timeout=3) is not None,
            "US-HOME-03 the developer block shows under QLEANY_DEV=1",
        )

        shot.save(session, os.path.join(SHOT_DIR, "qleany-home.png"))

        # Load it.
        dev = tree.wait_for_node(session, label="Open Qleany manifest", timeout=3)
        navigate.click(session, dev, settle=False)
        session.settle(settle_timeout_ms=2000)

        # US-SHELL-03: the rail unlocks.
        for row in LOCKED:
            node = tree.wait_for_node(session, label=row, timeout=5)
            checks.check(
                node is not None and not node.get("disabled", False),
                f"US-SHELL-03 {row!r} is reachable once a manifest is open",
            )

        # US-SHELL-01: the title names it.
        # The title is a bound text prop, so it reads as the node's `value`; the
        # `label` of a TextWidget is its constructor argument.
        values = [n.get("value") for n in tree.nodes(session)]
        checks.check(
            any(v and v.startswith("Qleany - ") and "qleany.yaml" in v for v in values),
            "US-SHELL-01 the title names the open manifest",
        )

        # US-MAN-06: a fresh load is not dirty. The load publishes a Created event
        # per entity it reads, each of which marks the manifest dirty, so this is
        # the assertion that the Load event gets the last word.
        save = tree.wait_for_node(session, label="No unsaved changes", timeout=3)
        checks.check(
            save is not None,
            "US-MAN-06 a freshly loaded manifest is not dirty",
        )

        shot.save(session, os.path.join(SHOT_DIR, "qleany-loaded.png"))

        # US-MAN-04: close locks it all again.
        close = tree.wait_for_node(session, label="Close current manifest", timeout=3)
        navigate.click(session, close, settle=False)
        session.settle(settle_timeout_ms=2000)
        for row in LOCKED:
            node = tree.wait_for_node(session, label=row, timeout=3)
            checks.check(
                node is not None and node.get("disabled", False),
                f"US-MAN-04 {row!r} locks again after close",
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
