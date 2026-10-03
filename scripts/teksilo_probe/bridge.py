# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Launching the app, and finding the automation bridge it publishes.

A debug build with the `automation` feature and
`.install_automation_bridge_in_debug()` binds an endpoint, writes an
**endpoint descriptor**, spawns its accept thread, and only then announces
itself on stderr::

    teksilo-automation: bridge endpoint = /run/user/1000/teksilo-automation/1234.sock
    teksilo-automation: descriptor = /run/user/1000/teksilo-automation/1234.json
    TEKSILO_AUTOMATION_TOKEN=<uuid>

**Discovery reads the descriptor, not the announce.** The prototype harness this
generalises scraped stderr, and 0.9.3 renamed `bridge socket = ` to
`bridge endpoint = ` — every probe's regex missed it, waited the full 60-second
timeout, and reported "no bridge socket" for an app whose bridge was up the
whole time. The descriptor's *path* is a published contract
(`<runtime dir>/teksilo-automation/<pid>.json`) and its schema is versioned, so
polling it by the pid we spawned depends on nothing that can be reworded.

The announce is still parsed, as a fallback for an older app and for a bridge
found from a log rather than a process we started. Both spellings are accepted,
because the rename is exactly the kind of thing that happens again.

Pin the token before launching (`launch()` does) and the fallback is never
needed for a process we own: we know the pid and we know the token, so there is
nothing left to scrape.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path
from typing import Mapping, NamedTuple, Sequence

#: The environment variable the app honours to pin its handshake token.
#: `teksilo_app::automation_bridge::install` reads it and falls back to a fresh
#: uuid, so setting it *before* launch makes the token known up front.
TOKEN_ENV = "TEKSILO_AUTOMATION_TOKEN"

#: Schema version of the descriptor this module understands
#: (`wire::ENDPOINT_FILE_VERSION`). A newer one is read anyway — every field
#: used here has been present since v1 — but the mismatch is worth reporting.
ENDPOINT_FILE_VERSION = 1

#: The announce lines, for the fallback path. `endpoint` is current; `socket`
#: is what 0.9.2 and earlier printed. Matching both is the whole point.
ANNOUNCE_ENDPOINT_RE = re.compile(r"bridge (?:endpoint|socket) = (\S+)")
ANNOUNCE_TOKEN_RE = re.compile(r"TEKSILO_AUTOMATION_TOKEN=(\S+)")
ANNOUNCE_DESCRIPTOR_RE = re.compile(r"teksilo-automation: descriptor = (\S+)")


class Bridge(NamedTuple):
    """Everything needed to attach a client to one running app."""

    pid: int | None
    endpoint: str
    token: str
    transport: str = "unix"
    descriptor: str | None = None
    app: str | None = None


class LaunchedApp(NamedTuple):
    """A process this harness started, and where its output went."""

    proc: subprocess.Popen
    log: str
    token: str

    def terminate(self) -> None:
        if self.proc.poll() is None:
            self.proc.terminate()

    def log_tail(self, lines: int = 25) -> str:
        return log_tail(self.log, lines)


# ---------------------------------------------------------------------------
# Where the descriptor lives
# ---------------------------------------------------------------------------


def runtime_dir() -> Path:
    """The per-user runtime directory, matching `wire::runtime_dir`.

    - **Windows** — `%LOCALAPPDATA%\\Teksilo`, per-user by ACL inheritance.
    - **Every other platform** — `$XDG_RUNTIME_DIR` when set (per-user `0700`
      by spec), otherwise the temp directory, which on Darwin is the per-user
      per-boot `/var/folders/…/T/`.

    Note that the XDG lookup is **not** conditional on Linux, and must not
    become so. `wire::runtime_dir` has no darwin branch either, and the one
    property that matters here is that this mirror looks where the app writes:
    a Mac whose environment happens to set `$XDG_RUNTIME_DIR` — some
    toolchains and dotfiles do — must have both sides agree on it. In practice
    Darwin sets no such variable, so the fallback is what runs there; that is
    an observation about the platform, not a branch in the code.
    """
    if sys.platform == "win32":
        local = os.environ.get("LOCALAPPDATA")
        return Path(local) / "Teksilo" if local else Path(tempfile.gettempdir())
    xdg = os.environ.get("XDG_RUNTIME_DIR")
    if xdg:
        return Path(xdg)
    return Path(tempfile.gettempdir())


def descriptor_dir() -> Path:
    return runtime_dir() / "teksilo-automation"


def descriptor_path(pid: int) -> Path:
    return descriptor_dir() / f"{pid}.json"


def parse_descriptor(text: str | bytes) -> Bridge:
    """A descriptor's JSON into a :class:`Bridge`.

    The `endpoint` is `#[serde(flatten)]`-ed on the Rust side, so `transport`
    and `address` sit at the top level rather than nested — a detail worth
    keeping in one place, since guessing it wrong produces a `Bridge` whose
    endpoint is empty and a failure that names the socket rather than the
    parser.
    """
    if isinstance(text, bytes):
        text = text.decode("utf-8")
    data = json.loads(text)
    if not isinstance(data, Mapping):
        raise ValueError("descriptor is not a JSON object")
    address = data.get("address")
    token = data.get("token")
    if not address or not token:
        raise ValueError("descriptor has no address/token — not an endpoint descriptor")
    return Bridge(
        pid=int(data["pid"]) if data.get("pid") is not None else None,
        endpoint=str(address),
        token=str(token),
        transport=str(data.get("transport") or "unix"),
        app=data.get("app"),
    )


def read_descriptor(path: str | os.PathLike) -> Bridge | None:
    """Read one descriptor, or `None` if it is absent / half-written / broken.

    A half-written file is a real state, not a paranoid one: this polls a path
    another process is creating, so a read can land between `open` and `write`.
    Treating that as "not there yet" and polling again is right; raising would
    turn a 3 ms race into a probe failure.
    """
    path = Path(path)
    try:
        raw = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None
    try:
        bridge = parse_descriptor(raw)
    except (ValueError, json.JSONDecodeError, KeyError):
        return None
    if bridge.pid is not None and int(bridge.pid) != _pid_of(path):
        # A descriptor is named after its pid. A mismatch means the file was
        # replaced under us by a pid-recycled process, and acting on it would
        # drive a different app entirely.
        return None
    return bridge._replace(descriptor=str(path))


def _pid_of(path: Path) -> int:
    try:
        return int(path.stem)
    except ValueError:
        return -1


def list_bridges() -> list[Bridge]:
    """Every descriptor currently on disk, newest first where it can be told.

    A descriptor outlives a process that exits without unwinding, so entries
    here are candidates, not guarantees — `connect` is what proves one is live.
    """
    found: list[tuple[float, Bridge]] = []
    try:
        entries = sorted(descriptor_dir().glob("*.json"))
    except OSError:
        return []
    for entry in entries:
        bridge = read_descriptor(entry)
        if bridge is None:
            continue
        try:
            stamp = entry.stat().st_mtime
        except OSError:
            stamp = 0.0
        found.append((stamp, bridge))
    found.sort(key=lambda pair: pair[0], reverse=True)
    return [bridge for _, bridge in found]


# ---------------------------------------------------------------------------
# The announce fallback
# ---------------------------------------------------------------------------


def parse_announce(text: str) -> Bridge | None:
    """A bridge out of an app's stderr, or `None` if it has not announced yet.

    Only used when the descriptor cannot be read — an app older than the
    descriptor, or a log from a process this harness did not start (so it has
    no pid to look one up by).
    """
    endpoint = ANNOUNCE_ENDPOINT_RE.search(text)
    token = ANNOUNCE_TOKEN_RE.search(text)
    if not endpoint or not token:
        return None
    descriptor = ANNOUNCE_DESCRIPTOR_RE.search(text)
    address = endpoint.group(1)
    transport = "named_pipe" if address.startswith("\\\\") else "unix"
    return Bridge(
        pid=None,
        endpoint=address,
        token=token.group(1),
        transport=transport,
        descriptor=descriptor.group(1) if descriptor else None,
    )


# ---------------------------------------------------------------------------
# Launch + wait
# ---------------------------------------------------------------------------


def launch(argv: Sequence[str], *, token: str | None = None,
           env: Mapping[str, str] | None = None, log: str | None = None,
           cwd: str | None = None, label: str = "app") -> LaunchedApp:
    """Start the app with a **known** handshake token, output going to a log.

    The token is pinned in the child's environment before the process exists, so
    it is known without reading anything the app prints. That is what lets
    discovery be descriptor-only: we already have the pid (we spawned it) and
    the token (we chose it), and the descriptor supplies the endpoint.

    Debug build, not release: the bridge is `#[cfg(debug_assertions)]`, so a
    release binary has nothing to attach to and a probe would wait out its whole
    timeout for a line that can never be printed.
    """
    child = dict(os.environ if env is None else env)
    token = token or child.get(TOKEN_ENV) or str(uuid.uuid4())
    child[TOKEN_ENV] = token
    if log is None:
        handle = tempfile.NamedTemporaryFile(
            prefix=f"teksilo-probe-{label}-", suffix=".log", delete=False)
        log = handle.name
        handle.close()
    stream = open(log, "w", encoding="utf-8")
    try:
        proc = subprocess.Popen(
            [str(a) for a in argv],
            stdout=stream,
            stderr=subprocess.STDOUT,
            env=child,
            cwd=cwd,
        )
    finally:
        # The child dup'd the descriptor; this handle is ours and holding it
        # only leaks one per probe and buffers nothing useful.
        stream.close()
    return LaunchedApp(proc, log, token)


def wait_for_bridge(pid: int | None = None, *, timeout: float = 60.0,
                    interval: float = 0.1, log: str | None = None,
                    proc: subprocess.Popen | None = None,
                    token: str | None = None) -> Bridge:
    """Poll until the app's bridge is attachable. Descriptor first, announce second.

    Fails fast when the process is already gone: waiting out a 60-second timeout
    for a process that exited 200 ms in only delays the log tail that says why.

    One invariant is asserted rather than assumed — a Unix endpoint named in the
    descriptor must already exist on disk. The bridge binds, then publishes,
    then spawns the accept thread, then announces, so by the time anything is
    readable the socket is connectable. If that stops holding, every client of
    this bridge is racing and not just this one, and the failure should name the
    ordering rather than surface as a flaky connect.
    """
    deadline = time.monotonic() + timeout
    while True:
        if pid is not None:
            bridge = read_descriptor(descriptor_path(pid))
            if bridge is not None:
                return _validated(bridge, token)
        if log is not None:
            bridge = parse_announce(_read(log))
            if bridge is not None:
                return _validated(bridge._replace(pid=pid), token)

        if proc is not None and proc.poll() is not None:
            raise RuntimeError(
                f"the app exited (code {proc.returncode}) before publishing an "
                f"automation bridge.\n{_log_section(log)}"
            )
        if time.monotonic() >= deadline:
            raise RuntimeError(
                f"no automation bridge within {timeout:g}s"
                + (f" (looked for {descriptor_path(pid)})" if pid is not None else "")
                + ".\nA release build has no bridge at all (it is "
                "`#[cfg(debug_assertions)]`), a build without teksilo's "
                "`automation` feature has none either, and an app that never "
                "called `install_automation_bridge_in_debug()` has none either."
                f"\n{_log_section(log)}"
            )
        time.sleep(interval)


def _validated(bridge: Bridge, token: str | None) -> Bridge:
    if bridge.transport == "unix" and not os.path.exists(bridge.endpoint):
        raise RuntimeError(
            f"the bridge published {bridge.endpoint} before binding it — the "
            "announce-before-bind race is back; see teksilo "
            "`automation_bridge::spawn_bridge_thread`, which binds, publishes, "
            "spawns the accept thread and only then announces."
        )
    if token and bridge.token != token:
        # We pinned a token and got a different one back: the descriptor belongs
        # to another process (a recycled pid, or a second app in the same
        # runtime dir). Driving it would report someone else's UI as ours.
        raise RuntimeError(
            f"the bridge at {bridge.endpoint} has a different token than the one "
            "this harness pinned — it belongs to another process."
        )
    return bridge


def attach(pid: int, *, timeout: float = 5.0) -> Bridge:
    """The bridge of an app already running under `pid`."""
    return wait_for_bridge(pid, timeout=timeout)


def mcp_argv(bridge: Bridge, mcp: str | None = None) -> list[str]:
    """The command line that attaches the MCP client to `bridge`.

    `--attach-pid` whenever the pid is known: the client then reads the endpoint
    and the token out of the `0600` descriptor. The alternative,
    `--connect <endpoint> --token <uuid>`, puts the token on a command line —
    and a command line is readable by every user on the machine through
    `/proc/<pid>/cmdline`, which is the exposure 0.9.3 tightened the descriptor's
    mode to close. Handing it straight back would undo that.

    The `--connect` form is the fallback for a bridge discovered from a log
    rather than a process we started; the token still travels in the
    environment (see `session._child_env`), not on the line, wherever the client
    will accept it there.
    """
    from .resolve import mcp_binary

    exe = mcp or mcp_binary()
    if bridge.pid is not None:
        return [exe, "--attach-pid", str(bridge.pid)]
    return [exe, "--connect", bridge.endpoint, "--token", bridge.token]


def _read(path: str) -> str:
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            return handle.read()
    except OSError:
        return ""


def log_tail(path: str | None, lines: int = 25) -> str:
    if not path:
        return ""
    return "\n".join(_read(path).splitlines()[-lines:])


def _log_section(path: str | None) -> str:
    tail = log_tail(path)
    return f"--- app log tail ---\n{tail}" if tail else "(no app log captured)"
