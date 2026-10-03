# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Drive a running teksilo app from Python, and assert on what it does.

A *probe* is a script that launches the app, attaches to the automation bridge
its debug build publishes, drives it through the accessibility tree, and exits
0 / 1 / 2. Nothing here needs a display server beyond what the app itself needs,
and nothing here is installed by the app: the bridge is compiled into the debug
build, and `teksilo-automation-mcp` — the client half — is the one thing to
`cargo install`.

The whole shape of a probe::

    #!/usr/bin/env python3
    from teksilo_probe import Report, launch_and_attach, tree

    app, session = launch_and_attach()
    report = Report("save button")
    try:
        tree.wait_for(session, "Untitled", timeout=30)
        save = tree.find(session, role="Button", label="Save")
        report.check(save is not None, "a Save button is on screen")
        if save:
            report.check(not save.get("disabled"), "…and it is enabled")
    except Exception as exc:
        report.error(str(exc))
    finally:
        session.close()
        app.terminate()
    raise SystemExit(report.finish())

Exit codes carry meaning: **0** pass, **1** the probe could not run, **2** it ran
cleanly and the behaviour is absent. See :mod:`teksilo_probe.report`.

Modules:

``session``   the MCP conversation — spawn, handshake, call, unwrap.
``tools``     generated typed wrapper per tool (do not edit; see `generate_tools.py`).
``bridge``    launching the app, and finding the endpoint descriptor it publishes.
``resolve``   locating the app and client binaries, and the version check.
``report``    checks, notes, errors, and the three-way exit code.
``tree``      snapshot / find / geometry over the accessibility tree.
``navigate``  reaching a widget a virtualized view has not realised yet.
``fixtures``  scratch space, and never opening a checked-in fixture.
``shot``      screenshots onto disk.
"""

from __future__ import annotations

from typing import Mapping, Sequence

from . import bridge, fixtures, navigate, report, resolve, shot, tools, tree
from .bridge import Bridge, LaunchedApp, attach, launch, mcp_argv, wait_for_bridge
from .fixtures import make_writable, scratch_dir, scratch_path, working_copy
from .navigate import (
    click,
    first_available,
    reveal_and_click,
    scroll_until,
    scroll_until_found,
    select_via_keyboard,
    visible_signature,
)
from .report import EXIT_ABSENT, EXIT_ERROR, EXIT_PASS, Report
from .resolve import ResolveError, app_binary, mcp_binary, teksilo_version
from .session import ProbeError, Session, ToolError, ToolResult, connect, unwrap
from .tree import bounds, center, find, find_all, in_region, labels, nodes, wait_for

__all__ = [
    # modules
    "bridge", "fixtures", "navigate", "report", "resolve", "session", "shot",
    "tools", "tree",
    # session
    "Session", "ToolResult", "ToolError", "ProbeError", "connect", "unwrap",
    # bridge
    "Bridge", "LaunchedApp", "launch", "wait_for_bridge", "attach", "mcp_argv",
    # resolve
    "ResolveError", "app_binary", "mcp_binary", "teksilo_version",
    # report
    "Report", "EXIT_PASS", "EXIT_ERROR", "EXIT_ABSENT",
    # tree
    "nodes", "find", "find_all", "labels", "bounds", "center", "in_region",
    "wait_for",
    # navigate
    "click", "reveal_and_click", "scroll_until", "scroll_until_found",
    "select_via_keyboard", "first_available", "visible_signature",
    # fixtures
    "working_copy", "make_writable", "scratch_dir", "scratch_path",
    # the one-call driver
    "launch_and_attach",
]

__version__ = "1"


def launch_and_attach(argv: Sequence[str] | None = None, *,
                      app: str | None = None,
                      args: Sequence[str] = (),
                      env: Mapping[str, str] | None = None,
                      token: str | None = None,
                      cwd: str | None = None,
                      label: str = "probe",
                      timeout: float = 60.0,
                      client_name: str = "teksilo-probe") -> tuple[LaunchedApp, Session]:
    """Launch the app, wait for its bridge, and attach an MCP session to it.

    The four steps every probe opens with, in the order that makes the later
    ones unnecessary:

    1. **Pin the token** before the process exists, so it is known without
       reading anything the app prints.
    2. **Launch** with output to a log, which is what a failure tail comes from.
    3. **Wait on the endpoint descriptor**, keyed by the pid we just spawned —
       not on a line of stderr whose wording has already changed once.
    4. **Attach** `teksilo-automation-mcp --attach-pid`, so the token never
       reaches a world-readable command line.

    Returns the launched app and a connected session. The caller closes both;
    `session` is also a context manager.
    """
    if argv is None:
        argv = [resolve.app_binary(app)] + [str(a) for a in args]
    launched = launch(argv, token=token, env=env, cwd=cwd, label=label)
    try:
        found = wait_for_bridge(
            launched.proc.pid, timeout=timeout, log=launched.log,
            proc=launched.proc, token=launched.token,
        )
        session = connect(found, client_name=client_name)
    except BaseException:
        launched.terminate()
        raise
    return launched, session
