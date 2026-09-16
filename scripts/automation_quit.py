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
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = ("US-SAFE-04",)
PROBE = "quit"
SETTLE = {"settle": {"settle_timeout_ms": 3000}}

# Every screen the rail offers. Generate is included even though it is gated on a
# clean manifest, because Qleany's own validates.
SCREENS = ["Home", "Project", "Entities", "Features", "User Interface", "Generate"]


def window_close(session):
    for node in session.nodes():
        if node.get("role") == "Button" and node.get("label") == "Close":
            return node
    return None


def quit_from(screen, checks):
    """Open the manifest, go to `screen`, close the window, report the exit."""
    log = os.path.join(fixture.SCRATCH, f"qleany-{PROBE}-{screen}-{os.getpid()}.log")
    manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), f"{PROBE}-{screen}")
    env = fixture.isolated_config(f"{PROBE}-{screen}")
    env["QLEANY_DEV"] = "1"
    env["RUST_BACKTRACE"] = "1"

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
        session.click(session.find("Open Qleany manifest", timeout=10))
        session.call("settle", SETTLE)

        target = session.find(screen, timeout=10)
        if not checks.check(target is not None, f"{screen}: the rail row is reachable"):
            return
        session.click(target)
        session.call("settle", SETTLE)

        close = window_close(session)
        if not checks.check(close is not None, f"{screen}: the window has a close button"):
            return
        # The window is gone after this, so the reply never arrives.
        try:
            session.call("invoke_action", {"node": close["id"], "action": "click"})
        except Exception:
            pass
    finally:
        if session:
            try:
                session.close()
            except Exception:
                pass
        # `close()` above terminated the MCP client, not the app. Give the app a
        # moment to finish its own shutdown before deciding how it went.
        deadline = time.time() + 10
        while app.poll() is None and time.time() < deadline:
            time.sleep(0.2)
        code = app.poll()
        if code is None:
            app.kill()
            app.wait(timeout=5)
            checks.check(False, f"US-SAFE-04 {screen}: the window closed and the process exited")
            return

    # A panic in a destructor aborts: SIGABRT is -6, and a clean exit is 0.
    checks.check(
        code == 0,
        f"US-SAFE-04 {screen}: quitting exits cleanly, got {code}"
        + (" (SIGABRT, look for 'panic in a destructor')" if code == -6 else ""),
    )
    if code != 0:
        print(fixture._tail(log, 12))


def main():
    checks = fixture.Checks()
    for screen in SCREENS:
        quit_from(screen, checks)
    return checks.finish(PROBE)


if __name__ == "__main__":
    sys.exit(main())
