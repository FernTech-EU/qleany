#!/usr/bin/env python3
"""Shared harness for the Qleany automation probes.

Every probe is a standalone script that launches the app, attaches to its
automation bridge over MCP, drives it and prints PASS or FAIL. This module is the
part they all share: finding the binaries, sandboxing the config, copying a
manifest somewhere safe to mutate, and the JSON-RPC session itself.

Two rules earned the hard way elsewhere and encoded here rather than left to each
probe:

- A probe works on `working_copy()`, never on a checked-in manifest. A probe that
  presses Save for real would rewrite its own fixture and then keep passing.
- Node ids are stable only for a widget instance's lifetime. Any structural
  rebuild allocates new ones, so `Session.nodes()` is re-read on every assertion
  rather than cached.
"""

import json
import os
import re
import select
import shutil
import subprocess
import sys
import tempfile
import time

SCRATCH = os.environ.get("QLEANY_AUTOMATION_SCRATCH") or tempfile.gettempdir()


def repo_root():
    """The workspace root, derived from this file rather than from the cwd.

    A probe that hard-codes an absolute path is a demonstration of a bug, not a
    test of one, and deriving from `__file__` keeps probes working from a git
    worktree.
    """
    return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def repo_path(*parts):
    return os.path.join(repo_root(), *parts)


def app_binary():
    """The debug binary. Debug on purpose: release has no automation bridge."""
    env = os.environ.get("QLEANY_BIN")
    if env:
        return env
    target = os.environ.get("CARGO_TARGET_DIR") or repo_path("target")
    for name in ("qleany-teksilo", "qleany"):
        candidate = os.path.join(target, "debug", name)
        if os.path.exists(candidate):
            return candidate
    raise SystemExit(
        "no debug binary found; run `cargo build -p qleany-teksilo-ui` first"
    )


def mcp_binary():
    env = os.environ.get("TEKSILO_MCP_BIN")
    if env:
        return env
    found = shutil.which("teksilo-automation-mcp")
    if found:
        return found
    raise SystemExit(
        "teksilo-automation-mcp not on PATH; install it with\n"
        "  cargo install teksilo-automation-mcp --locked"
    )


def isolated_config(label, dark=False):
    """A private XDG_CONFIG_HOME, so a probe never reads or writes the real one.

    Returns an environment dict to hand to `subprocess.Popen`.
    """
    home = os.path.join(SCRATCH, f"qleany-automation-{label}-{os.getpid()}")
    os.makedirs(os.path.join(home, "config"), exist_ok=True)
    env = dict(os.environ)
    env["XDG_CONFIG_HOME"] = os.path.join(home, "config")
    env["QLEANY_AUTOMATION_HOME"] = home
    return env


def working_copy(src, label):
    """A throwaway copy of a manifest, outside the repo.

    Asserts the copy really did land outside, because a probe that mutates a
    checked-in fixture corrupts it and then still passes.
    """
    dest_dir = os.path.join(SCRATCH, f"qleany-fixture-{label}-{os.getpid()}")
    os.makedirs(dest_dir, exist_ok=True)
    dest = os.path.join(dest_dir, os.path.basename(src))
    shutil.copy2(src, dest)
    os.chmod(dest, 0o644)
    assert not os.path.abspath(dest).startswith(os.path.abspath(repo_root())), (
        f"working copy {dest} is inside the repo; a probe must never mutate a "
        "checked-in fixture"
    )
    return dest


BRIDGE_ENDPOINT = re.compile(r"bridge (?:endpoint|socket) = (\S+)")
BRIDGE_TOKEN = re.compile(r"TEKSILO_AUTOMATION_TOKEN=(\S+)")


class Bridge:
    def __init__(self, endpoint, token, pid):
        self.endpoint = endpoint
        self.token = token
        self.pid = pid


def wait_for_bridge(log_path, proc, timeout=90):
    """Poll the app's log for the three lines the bridge prints on start.

    The socket is bound before the line is printed, so there is no retry loop
    after this returns. A process that exited is reported as such rather than
    waited on until the timeout.
    """
    deadline = time.time() + timeout
    while time.time() < deadline:
        if proc.poll() is not None:
            tail = _tail(log_path)
            raise SystemExit(
                f"the app exited with {proc.returncode} before the bridge came up.\n"
                f"Usual causes: a release build (no bridge) or the `automation` "
                f"feature switched off.\n{tail}"
            )
        try:
            with open(log_path, "r", encoding="utf-8", errors="replace") as fh:
                text = fh.read()
        except FileNotFoundError:
            text = ""
        ep = BRIDGE_ENDPOINT.search(text)
        tok = BRIDGE_TOKEN.search(text)
        if ep and tok:
            endpoint = ep.group(1)
            assert os.path.exists(endpoint), (
                f"the bridge announced {endpoint} before binding it"
            )
            return Bridge(endpoint, tok.group(1), proc.pid)
        time.sleep(0.2)
    raise SystemExit(f"the bridge did not come up in {timeout}s\n{_tail(log_path)}")


def _tail(path, lines=25):
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            return "--- app log tail ---\n" + "".join(fh.readlines()[-lines:])
    except OSError:
        return ""


def descendants(nodes, root_id):
    """Every node at or under `root_id`, as a list.

    A modal renders in an overlay that is a sibling of the application body, so
    the whole tree holds two of everything a modal repeats: two "Generate"
    buttons, two "Close" buttons. Matching by label alone finds whichever comes
    first, which is the one behind the scrim, and clicking it answers
    UNHANDLED_ACTION because it is disabled. Scope the search instead.

    Returned in document order, which a probe asserting "the two languages are
    offered, in this order" depends on: a stack that pushed children as it found
    them would hand back every row of every list reversed.
    """
    by_id = {n["id"]: n for n in nodes}
    out, stack = [], [root_id]
    seen = set()
    while stack:
        node_id = stack.pop()
        if node_id in seen:
            continue
        seen.add(node_id)
        node = by_id.get(node_id)
        if node is None:
            continue
        out.append(node)
        stack.extend(reversed(node.get("children") or []))
    return out


def overlay_nodes(session, role="Dialog"):
    """Every node of the topmost overlay with `role` at its root, or []."""
    nodes = session.nodes()
    roots = [n for n in nodes if n.get("role") == role]
    if not roots:
        return []
    return descendants(nodes, roots[-1]["id"])


def mcp_argv(bridge, mcp=None):
    """Attach by pid rather than by token.

    `--connect <sock> --token <uuid>` puts the token on a command line anyone can
    read out of /proc.
    """
    return [mcp or mcp_binary(), "--attach-pid", str(bridge.pid)]


class Session:
    """One MCP conversation with the running app.

    Lives here rather than in each probe: Skribisto open-coded this in all 73 of
    its probes and calls that its own tech debt.
    """

    def __init__(self, bridge, mcp=None, stderr_path=None):
        self._err = open(stderr_path or os.devnull, "w")
        self._proc = subprocess.Popen(
            mcp_argv(bridge, mcp),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self._err,
            text=True,
            bufsize=1,
        )
        self._next_id = 0
        self._handshake()

    def _send(self, obj):
        self._proc.stdin.write(json.dumps(obj) + "\n")
        self._proc.stdin.flush()

    def _recv(self, timeout=30):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self._proc.poll() is not None:
                raise SystemExit(
                    f"the MCP server exited with {self._proc.returncode}"
                )
            ready, _, _ = select.select([self._proc.stdout], [], [], 0.2)
            if ready:
                line = self._proc.stdout.readline()
                if line.strip():
                    return json.loads(line)
        raise SystemExit("timed out waiting for the MCP server")

    def _handshake(self):
        self._next_id += 1
        self._send({
            "jsonrpc": "2.0",
            "id": self._next_id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "qleany-automation", "version": "1"},
            },
        })
        self._recv()
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def call(self, name, args=None):
        """One tool call. Returns (raw result, parsed payload)."""
        self._next_id += 1
        self._send({
            "jsonrpc": "2.0",
            "id": self._next_id,
            "method": "tools/call",
            "params": {"name": name, "arguments": args or {}},
        })
        msg = self._recv()
        result = msg.get("result", msg)
        payload = result.get("structuredContent")
        if payload is None:
            text = "".join(
                c.get("text", "")
                for c in result.get("content", [])
                if c.get("type") == "text"
            )
            if text.strip()[:1] in "{[":
                payload = json.loads(text)
        return result, payload

    def nodes(self, **kwargs):
        """The accessibility tree, re-read every time it is asked for."""
        _, payload = self.call("snapshot_tree", kwargs)
        return (payload or {}).get("nodes", [])

    def find(self, label, timeout=10, role=None):
        """The first node whose label matches, polling until it appears."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            for n in self.nodes():
                if n.get("label") == label and (role is None or n.get("role") == role):
                    return n
            self.call("settle")
            time.sleep(0.25)
        return None

    def labels(self):
        return [n.get("label") for n in self.nodes() if n.get("label")]

    def click(self, node):
        return self.call("invoke_action", {"node": node["id"], "action": "click"})

    def shot(self, path, node=None):
        args = {"node": node["id"]} if node else {}
        result, _ = self.call("screenshot", args)
        for c in result.get("content", []):
            if c.get("type") == "image":
                import base64

                with open(path, "wb") as fh:
                    fh.write(base64.b64decode(c["data"]))
                return path
        return None

    def close(self):
        try:
            self._proc.terminate()
            self._proc.wait(timeout=5)
        except Exception:
            self._proc.kill()
        self._err.close()


class Checks:
    """Collect every violation in one run rather than stopping at the first."""

    def __init__(self):
        self.failures = []

    def check(self, ok, message):
        if ok:
            print(f"  ok   {message}")
        else:
            print(f"  FAIL {message}")
            self.failures.append(message)
        return ok

    def finish(self, probe, log_path=None):
        if self.failures:
            print(f"FAIL: {probe} ({len(self.failures)} failed)")
            for f in self.failures:
                print(f"  - {f}")
            if log_path:
                print(_tail(log_path))
            return 1
        print(f"PASS: {probe}")
        return 0
