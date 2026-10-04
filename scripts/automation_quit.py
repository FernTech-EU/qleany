#!/usr/bin/env python3
"""Probe: quitting from every screen exits cleanly.

Covers US-SAFE-04. The other probes cannot test this: answering the quit guard
closes the window, which ends the session they are driving. So this one starts
the application once per screen, visits it, closes the window, and asserts the
process exited 0.

It exists because of a crash that only this shape finds. A view-model registered
an effect on a signal belonging to one of its own generated handles, and that
handle keeps an `ObserverHandle` on the same signal for its dirty tracking. The
closure therefore owned the handle. Tearing the window down dropped the closure
from inside the signal's own borrow, the handle's removal re-entered that borrow,
and a panic in a destructor aborts rather than unwinds. Nothing in a unit test or
in any other probe touches the teardown path, and the failure is invisible until
a user closes the window from the wrong screen.
"""

import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture
from teksilo_probe import Report, launch_and_attach, navigate, tree, ProbeError

STORIES = ("US-SAFE-04",)
PROBE = "quit"
SETTLE = {"settle_timeout_ms": 3000}

# Every screen the rail offers. Generate is included even though it is gated on a
# clean manifest, because Qleany's own validates.
SCREENS = ["Home", "Project", "Entities", "Features", "User Interface", "Generate"]


def window_close(session):
    for node in tree.nodes(session):
        if node.get("role") == "Button" and node.get("label") == "Close":
            return node
    return None


def quit_from(screen, checks):
    """Open the manifest, go to `screen`, close the window, report the exit."""
    app = session = None
    try:
        manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), f"{PROBE}-{screen}")
        env = fixture.isolated_config(f"{PROBE}-{screen}")
        env["QLEANY_DEV"] = "1"
        env["RUST_BACKTRACE"] = "1"

        app, session = launch_and_attach(
            argv=[fixture.app_binary()], cwd=os.path.dirname(manifest),
            env=env, label=f"qleany-{PROBE}-{screen}",
        )
        navigate.click(session, tree.wait_for_node(session, label="Open Qleany manifest", timeout=30), settle=False)
        session.settle(**SETTLE)
        target = tree.wait_for_node(session, label=screen, timeout=10)
        if not checks.check(target is not None, f"{screen}: the rail row is reachable"):
            return
        navigate.click(session, target)
        close = window_close(session)
        if not checks.check(close is not None, f"{screen}: the window has a close button"):
            return
        # Drive the title-bar pointer route, as in the dialog example.
        # Closing the window can disconnect MCP before it replies. Only that
        # shutdown call may fail; the actual process exit is the assertion.
        try:
            x, y = tree.center(close)
            session.tools.inject_pointer(x=x, y=y, action="click")
        except ProbeError as exc:
            checks.note(f"{screen}: close disconnected MCP: {exc}")
        try:
            code = app.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            checks.check(False, f"US-SAFE-04 {screen}: the process exits after closing")
            checks.note(f"{screen}: remaining UI: {tree.labels(session)}")
        else:
            checks.check(
                code == 0,
                f"US-SAFE-04 {screen}: quitting exits cleanly, got {code}"
                + (" (SIGABRT, look for 'panic in a destructor')" if code == -6 else ""),
            )
    except Exception as exc:
        checks.error(f"{screen}: {type(exc).__name__}: {exc}")
    finally:
        if session:
            session.close()
        if app:
            if checks.exit_code:
                checks.note(app.log_tail())
            app.terminate()


def main():
    checks = Report(PROBE)
    for screen in SCREENS:
        quit_from(screen, checks)
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
