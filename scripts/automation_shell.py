#!/usr/bin/env python3
"""Probe: the window chrome.

Covers US-SHELL-02 (the rail moves between screens), US-SHELL-03 (screens that
need a manifest are out of reach without one), US-SHELL-04 (a Save button sits
beside the hamburger), US-MENU-01 (the hamburger opens the four menus) and
US-THEME-01 (an icon button toggles the theme).
"""

import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture
from teksilo_probe import Report, launch_and_attach, navigate, shot, tree

STORIES = ("US-SHELL-02", "US-SHELL-03", "US-SHELL-04", "US-MENU-01", "US-THEME-01")
PROBE = "shell"

SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())


def main():
    checks = Report(PROBE)
    app = session = None
    try:
        env = fixture.isolated_config(PROBE)
        app, session = launch_and_attach(
            argv=[fixture.app_binary()],
            env=env, label=f"qleany-{PROBE}",
        )
        if tree.wait_for_node(session, label="Home", timeout=30) is None:
            raise RuntimeError("the app never rendered its navigation rail")
        session.settle()

        labels = tree.labels(session)
        print(f"  tree has {len(labels)} labelled nodes")

        # US-SHELL-02: every rail row is present.
        for row in ("Home", "Project", "Entities", "Features", "User Interface",
                    "Generate"):
            checks.check(row in labels, f"US-SHELL-02 rail shows {row!r}")

        # US-SHELL-03: with no manifest open, only Home is reachable.
        for row in ("Project", "Entities", "Features", "User Interface", "Generate"):
            node = tree.wait_for_node(session, label=row, timeout=2)
            disabled = node is not None and node.get("disabled", False)
            checks.check(
                disabled,
                f"US-SHELL-03 {row!r} is disabled with no manifest open",
            )
        home = tree.wait_for_node(session, label="Home", timeout=2)
        checks.check(
            home is not None and not home.get("disabled", False),
            "US-SHELL-03 Home stays reachable",
        )

        # US-SHELL-04 and US-THEME-01: the two title-bar icon buttons.
        checks.check(
            tree.wait_for_node(session, label="No unsaved changes", timeout=2) is not None,
            "US-SHELL-04 a Save button is in the title bar, disabled when clean",
        )
        checks.check(
            tree.wait_for_node(session, label="Switch to dark theme", timeout=2) is not None,
            "US-THEME-01 a theme toggle is in the title bar",
        )

        toggle = tree.wait_for_node(session, label="Switch to dark theme", timeout=2)
        if toggle is not None:
            navigate.click(session, toggle)
            checks.check(tree.wait_for_node(session, label="Switch to light theme", timeout=3) is not None,
                         "US-THEME-01 activating the toggle selects dark theme")
            toggle = tree.wait_for_node(session, label="Switch to light theme", timeout=2)
            if toggle is not None:
                navigate.click(session, toggle)
                checks.check(tree.wait_for_node(session, label="Switch to dark theme", timeout=3) is not None,
                             "US-THEME-01 activating again returns to light theme")

        # US-MENU-01: the hamburger opens a menu carrying the four top-level names.
        burger = tree.wait_for_node(session, label="Menu", timeout=3)
        if checks.check(burger is not None, "US-MENU-01 a hamburger is present"):
            navigate.click(session, burger, settle=False)
            session.settle()
            after = tree.labels(session)
            for menu in ("File", "Edit", "View", "Help"):
                checks.check(
                    any(m and m.replace("&", "") == menu for m in after),
                    f"US-MENU-01 the menu carries {menu!r}",
                )

        screenshot = os.path.join(SHOT_DIR, "qleany-shell.png")
        shot.save(session, screenshot)
        checks.note(f"screenshot: {screenshot}")
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
