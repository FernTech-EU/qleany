#!/usr/bin/env python3
"""Probe: the window chrome.

Covers US-SHELL-02 (the rail moves between screens), US-SHELL-03 (screens that
need a manifest are out of reach without one), US-SHELL-04 (a Save button sits
beside the hamburger), US-MENU-01 (the hamburger opens the four menus) and
US-THEME-01 (an icon button toggles the theme).
"""

import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = ("US-SHELL-02", "US-SHELL-03", "US-SHELL-04", "US-MENU-01", "US-THEME-01")
PROBE = "shell"

SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())


def main():
    checks = fixture.Checks()
    log = os.path.join(
        fixture.SCRATCH, f"qleany-{PROBE}-{os.getpid()}.log"
    )
    env = fixture.isolated_config(PROBE)
    app = subprocess.Popen(
        [fixture.app_binary()],
        stdout=open(log, "w"),
        stderr=subprocess.STDOUT,
        env=env,
    )
    session = None
    try:
        bridge = fixture.wait_for_bridge(log, app, timeout=60)
        session = fixture.Session(bridge)

        labels = session.labels()
        print(f"  tree has {len(labels)} labelled nodes")

        # US-SHELL-02: every rail row is present.
        for row in ("Home", "Project", "Entities", "Features", "User Interface",
                    "Generate"):
            checks.check(row in labels, f"US-SHELL-02 rail shows {row!r}")

        # US-SHELL-03: with no manifest open, only Home is reachable.
        for row in ("Project", "Entities", "Features", "User Interface", "Generate"):
            node = session.find(row, timeout=2)
            disabled = node is not None and node.get("disabled", False)
            checks.check(
                disabled,
                f"US-SHELL-03 {row!r} is disabled with no manifest open",
            )
        home = session.find("Home", timeout=2)
        checks.check(
            home is not None and not home.get("disabled", False),
            "US-SHELL-03 Home stays reachable",
        )

        # US-SHELL-04 and US-THEME-01: the two title-bar icon buttons.
        checks.check(
            session.find("No unsaved changes", timeout=2) is not None,
            "US-SHELL-04 a Save button is in the title bar, disabled when clean",
        )
        checks.check(
            session.find("Switch to dark theme", timeout=2) is not None,
            "US-THEME-01 a theme toggle is in the title bar",
        )

        # US-MENU-01: the hamburger opens a menu carrying the four top-level names.
        burger = session.find("Menu", timeout=3)
        if checks.check(burger is not None, "US-MENU-01 a hamburger is present"):
            session.click(burger)
            session.call("settle")
            after = session.labels()
            for menu in ("File", "Edit", "View", "Help"):
                checks.check(
                    any(m and m.replace("&", "") == menu for m in after),
                    f"US-MENU-01 the menu carries {menu!r}",
                )

        shot = session.shot(os.path.join(SHOT_DIR, "qleany-shell.png"))
        if shot:
            print(f"  screenshot: {shot}")
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
