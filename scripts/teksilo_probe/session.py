# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""The MCP conversation: spawn the client, handshake, call tools.

**The wire protocol is not reimplemented here, on purpose.** The app's bridge
speaks a framed protocol that `teksilo_automation::client` states plainly is not
frozen — 0.9.3 replaced the socket announce with an endpoint descriptor and put
a deadline on the token handshake. A Python reimplementation of that framing
would be a second, unversioned copy of a moving target. So a probe spawns
`teksilo-automation-mcp`, the versioned Rust client, as a subprocess and talks
MCP JSON-RPC over its stdin/stdout — the one interface that *is* a published
contract.

Two shapes the replies come in, both handled:

* `structuredContent` — what every current tool returns. Read it directly.
* `content[].text` — the same JSON as a text block, which is all an older
  server sent. Parsed as the fallback.

and the payload itself may be an object *or* an array (`snapshot_tree` returns
an object with `nodes`; some tools return a bare list), so nothing here assumes
a dict.
"""

from __future__ import annotations

import json
import os
import queue
import subprocess
import threading
import time
from collections import deque
from typing import Any, Iterable, Mapping

#: MCP protocol revision announced at `initialize`.
PROTOCOL_VERSION = "2024-11-05"


class ProbeError(RuntimeError):
    """Anything that stops a probe from driving the app at all."""


class ToolError(ProbeError):
    """A tool came back with `isError`.

    `code` is the stable machine-readable one the server promises — `NOT_FOUND`,
    `BAD_ARGUMENT`, `UNKNOWN_NAME`, `ASSERTION_FAILED`, `GPU_UNAVAILABLE`,
    `SETTLE_TIMEOUT`. Branch on it rather than on the message: the last two are
    environmental (no GPU, a poll hit its budget) and the first four are real
    mistakes, and only the code tells them apart.
    """

    def __init__(self, tool: str, code: str, message: str) -> None:
        super().__init__(f"{tool}: {code}: {message}")
        self.tool = tool
        self.code = code
        self.message = message


class ToolResult:
    """One tool reply, whether or not it succeeded."""

    __slots__ = ("tool", "ok", "payload", "code", "message", "content")

    def __init__(self, tool, ok, payload, code, message, content) -> None:
        self.tool = tool
        self.ok = ok
        self.payload = payload
        self.code = code
        self.message = message
        self.content = content

    def __bool__(self) -> bool:
        return self.ok

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        state = "ok" if self.ok else f"error {self.code}"
        return f"<ToolResult {self.tool} {state}>"


class _Pump(threading.Thread):
    """Read a pipe line-by-line onto a queue.

    A thread rather than `select`: `select` cannot watch a pipe on Windows, and
    a harness that only works on Unix is a harness that silently stops covering
    two of the three platforms the bridge supports.
    """

    def __init__(self, stream, sink) -> None:
        super().__init__(daemon=True)
        self.stream = stream
        self.sink = sink

    def run(self) -> None:
        try:
            for line in self.stream:
                self.sink(line)
        except (ValueError, OSError):
            pass  # the pipe closed under us; the reader notices via poll()


class Session:
    """A live MCP conversation with one app.

    Normally built by :func:`connect`, which owns the retry loop::

        with connect(bridge) as s:
            s.call("settle")
    """

    def __init__(self, proc: subprocess.Popen, *, timeout: float = 30.0) -> None:
        self.proc = proc
        self.timeout = timeout
        self._next_id = 0
        self._inbox: "queue.Queue[str]" = queue.Queue()
        #: The client's own stderr, kept for diagnostics. Bounded: a chatty
        #: client must not grow a probe's memory without limit.
        self.stderr_tail: "deque[str]" = deque(maxlen=200)
        _Pump(proc.stdout, self._inbox.put).start()
        if proc.stderr is not None:
            _Pump(proc.stderr, lambda line: self.stderr_tail.append(line.rstrip("\n"))).start()

    # -- JSON-RPC ---------------------------------------------------------

    def _send(self, method: str, params: Mapping[str, Any] | None = None,
              *, notify: bool = False) -> int | None:
        message: dict[str, Any] = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            message["params"] = params
        rpc_id = None
        if not notify:
            self._next_id += 1
            rpc_id = self._next_id
            message["id"] = rpc_id
        if self.proc.stdin is None or self.proc.poll() is not None:
            raise ProbeError(self._died("the MCP client is gone"))
        self.proc.stdin.write(json.dumps(message) + "\n")
        self.proc.stdin.flush()
        return rpc_id

    def _recv(self, rpc_id: int, timeout: float) -> dict:
        """The reply with this id, skipping notifications and other traffic."""
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise ProbeError(self._died(f"no MCP reply within {timeout:g}s"))
            try:
                line = self._inbox.get(timeout=min(remaining, 0.25))
            except queue.Empty:
                if self.proc.poll() is not None and self._inbox.empty():
                    raise ProbeError(self._died("the MCP client exited"))
                continue
            line = line.strip()
            if not line:
                continue
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                continue  # a stray log line on stdout is not our business
            if message.get("id") == rpc_id:
                return message

    def _died(self, what: str) -> str:
        tail = "\n  ".join(list(self.stderr_tail)[-12:])
        code = self.proc.poll()
        suffix = f" (exit {code})" if code is not None else ""
        return f"{what}{suffix}" + (f"\n--- MCP client stderr ---\n  {tail}" if tail else "")

    # -- Tools ------------------------------------------------------------

    def try_call(self, tool: str, *, _timeout: float | None = None, **args: Any) -> ToolResult:
        """Call a tool and report the outcome without raising on a tool error."""
        rpc_id = self._send("tools/call", {"name": tool, "arguments": dict(args)})
        assert rpc_id is not None
        message = self._recv(rpc_id, _timeout if _timeout is not None else self.timeout)
        if "error" in message:
            err = message["error"] or {}
            raise ProbeError(
                f"{tool}: JSON-RPC error {err.get('code')}: {err.get('message')}"
            )
        result = message.get("result") or {}
        payload = unwrap(result)
        is_error = bool(result.get("isError"))
        code = message_text = ""
        if is_error and isinstance(payload, Mapping):
            code = str(payload.get("code") or "ERROR")
            message_text = str(payload.get("message") or "")
        elif is_error:
            code, message_text = "ERROR", str(payload)
        return ToolResult(tool, not is_error, payload, code, message_text,
                          result.get("content") or [])

    def call(self, tool: str, *, _timeout: float | None = None, **args: Any) -> Any:
        """Call a tool and return its payload. Raises :class:`ToolError` on failure."""
        result = self.try_call(tool, _timeout=_timeout, **args)
        if not result.ok:
            raise ToolError(tool, result.code, result.message)
        return result.payload

    def call_result(self, tool: str, *, _timeout: float | None = None, **args: Any) -> ToolResult:
        """Like :meth:`call`, but hand back the whole result — content blocks included.

        `screenshot` is why this exists: its pixels ride in an image content
        block, not in the payload.
        """
        result = self.try_call(tool, _timeout=_timeout, **args)
        if not result.ok:
            raise ToolError(tool, result.code, result.message)
        return result

    @property
    def tools(self):
        """The generated typed wrappers, bound to this session.

        `session.tools.inject_key(key="Enter")` is `tools.inject_key(session,
        key="Enter")`. Prefer it over :meth:`call`: the wrapper knows the tool's
        real argument names, and a typo becomes a `TypeError` here instead of a
        `deny_unknown_fields` rejection three layers away.
        """
        return _ToolsProxy(self)

    def settle(self, **spec: Any) -> Any:
        """Run animations and layout to quiescence, then re-sync the tree."""
        return self.call("settle", **({"settle": spec} if spec else {}))

    # -- Lifecycle --------------------------------------------------------

    def initialize(self, client_name: str = "teksilo-probe") -> dict:
        """The MCP handshake: `initialize`, then `notifications/initialized`."""
        rpc_id = self._send("initialize", {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": client_name, "version": "1"},
        })
        assert rpc_id is not None
        reply = self._recv(rpc_id, min(self.timeout, 10.0))
        self._send("notifications/initialized", notify=True)
        return reply.get("result") or {}

    def close(self) -> None:
        """Terminate the client. Safe to call twice."""
        if self.proc.poll() is None:
            try:
                self.proc.terminate()
                self.proc.wait(timeout=5)
            except (OSError, subprocess.TimeoutExpired):
                try:
                    self.proc.kill()
                except OSError:
                    pass
        for pipe in (self.proc.stdin, self.proc.stdout, self.proc.stderr):
            try:
                if pipe is not None:
                    pipe.close()
            except (OSError, ValueError):
                pass

    def __enter__(self) -> "Session":
        return self

    def __exit__(self, *exc) -> None:
        self.close()


class _ToolsProxy:
    """`session.tools.<name>(...)` — the generated wrappers with the session bound."""

    __slots__ = ("_session",)

    def __init__(self, session: Session) -> None:
        self._session = session

    def __getattr__(self, name: str):
        from . import tools as _tools

        fn = getattr(_tools, name, None)
        if fn is None or name.startswith("_"):
            raise AttributeError(
                f"no automation tool named {name!r}. Known tools: "
                + ", ".join(_tools.TOOL_NAMES)
            )

        def bound(*args, **kwargs):
            return fn(self._session, *args, **kwargs)

        bound.__name__ = name
        bound.__doc__ = fn.__doc__
        return bound

    def __dir__(self) -> Iterable[str]:
        from . import tools as _tools

        return list(_tools.TOOL_NAMES)


def unwrap(result: Mapping[str, Any]) -> Any:
    """The payload out of an MCP `tools/call` result.

    `structuredContent` first — every current tool mirrors its JSON there
    precisely so a client can branch on the stable `code` field without parsing
    prose. The text blocks are the fallback for an older server, and they may
    hold an object *or* an array, so nothing here assumes a dict. Text that is
    not JSON at all comes back as `{"_text": ...}` rather than raising: a tool
    that answered in prose has still answered.
    """
    if "structuredContent" in result and result["structuredContent"] is not None:
        return result["structuredContent"]
    text = "".join(
        block.get("text", "")
        for block in (result.get("content") or [])
        if isinstance(block, Mapping) and block.get("type") == "text"
    )
    stripped = text.strip()
    if stripped[:1] in ("{", "["):
        try:
            return json.loads(stripped)
        except json.JSONDecodeError:
            pass
    return {"_text": text}


def connect(bridge, *, mcp: str | None = None, client_name: str = "teksilo-probe",
            timeout: float = 30.0, connect_timeout: float = 30.0,
            attempt_timeout: float = 5.0) -> Session:
    """Spawn the MCP client against `bridge` and complete the handshake.

    Retries the *whole* spawn, not just the read. The descriptor is published
    after the endpoint is bound but a moment before the accept thread exists, so
    a client that arrives in that window gets a connection nobody has picked up
    yet; and a client that fails to connect exits rather than waiting. Killing
    it and starting a new one is therefore the retry that works, which is why
    this loop respawns instead of re-reading.
    """
    from .bridge import mcp_argv

    argv = mcp_argv(bridge, mcp)
    deadline = time.monotonic() + connect_timeout
    last: Exception | None = None
    while True:
        try:
            proc = subprocess.Popen(
                argv,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
                env=_child_env(bridge),
            )
        except OSError as exc:
            # Not retryable, and worth saying plainly: an absent or
            # non-executable client is a setup problem, and retrying it for
            # 30 s would bury the one sentence that fixes it.
            raise ProbeError(
                f"cannot run the MCP client `{argv[0]}`: {exc}\n"
                "Install it with `cargo install teksilo-automation-mcp --locked`, "
                "or set $TEKSILO_MCP_BIN to an existing build."
            ) from exc
        session = Session(proc, timeout=timeout)
        try:
            session.initialize(client_name)
            return session
        except ProbeError as exc:
            last = exc
            session.close()
        if time.monotonic() >= deadline:
            raise ProbeError(
                f"could not attach `{argv[0]}` to the app within {connect_timeout:g}s.\n"
                f"Last attempt: {last}"
            )
        time.sleep(0.3)


def _child_env(bridge) -> dict:
    """The client's environment.

    The token goes in the environment, never on the command line: a command line
    is world-readable through `/proc/<pid>/cmdline`, and closing that exposure is
    why the descriptor is `0600` in the first place. `--attach-pid` normally
    means the client reads the token out of the descriptor itself and this never
    matters; it does matter on the `--connect` fallback path.
    """
    env = dict(os.environ)
    token = getattr(bridge, "token", None)
    if token:
        env["TEKSILO_AUTOMATION_TOKEN"] = token
    return env
