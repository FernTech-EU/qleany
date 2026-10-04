# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Typed wrappers — one per automation tool. **Generated; do not edit.**

Regenerate with `python3 generate_tools.py` from a teksilo checkout. The names,
descriptions and `mutating` flags come from `TOOL_CATALOG`
(`crates/teksilo-automation/src/mcp_schema.rs`); the argument names and types
come from the `#[tool]` handlers' parameter structs
(`crates/teksilo-automation-mcp/src/server.rs`).

Every parameter struct on the Rust side carries `#[serde(deny_unknown_fields)]`,
so a misspelt argument is refused rather than ignored — but a *valid name used
for the wrong thing* is not. `inject_pointer` has both a `kind` (mouse / touch /
pen) and an `action` (click / double_click / down / up / move), and a harness
that passed `kind="click"` was quietly asking for an unknown pointer kind and
working only because `action` defaults to `click`. Calling these wrappers makes
that mistake unrepresentable.

`None` arguments are dropped before the call, so an omitted argument is genuinely
omitted and the Rust side's own default applies.

Each function takes the `Session` first and returns the tool's payload:

    from teksilo_probe import tools
    tree = tools.snapshot_tree(session)
    tools.inject_key(session, key="Enter", command=True)
"""

from __future__ import annotations

from typing import Any, Mapping, Sequence

__all__ = [
    "snapshot_tree",
    "read_node",
    "layout_tree",
    "inspect_node",
    "find_node",
    "assert_node",
    "list_windows",
    "invoke_action",
    "focus_node",
    "set_value",
    "expand",
    "collapse",
    "scroll",
    "inject_pointer",
    "right_click",
    "inject_key",
    "type_text",
    "type_ime",
    "drag_node",
    "inject_touch_sequence",
    "pinch",
    "fling",
    "long_press",
    "cancel_pointer",
    "query_pointers",
    "set_density",
    "get_overlays",
    "get_shortcuts",
    "list_live_regions",
    "pull_announcements",
    "advance_clock",
    "settle",
    "wait_for_condition",
    "screenshot",
    "TOOL_NAMES",
    "MUTATING",
]

def _present(args: Mapping[str, Any]) -> dict:
    """Drop the arguments the caller left out.

    Not a cosmetic filter: the Rust parameter structs use `deny_unknown_fields`
    and every optional field is an `Option`, so sending an explicit `null` for
    one is different from omitting it — and only omission lets the Rust side's
    own default apply.
    """
    return {k: v for k, v in args.items() if v is not None}


def snapshot_tree(
    session,
    *,
    window_id: int | None = None,
    max_depth: int | None = None,
) -> Any:
    """Snapshot the accessibility tree (roles, labels, values, bounds, actions).

    Read-only tool.
    """
    args = {
        "window_id": window_id,
        "max_depth": max_depth,
    }
    return session.call("snapshot_tree", **_present(args))


def read_node(session, node: int, *, window_id: int | None = None) -> Any:
    """Read a single semantic node by its id.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
    }
    return session.call("read_node", **_present(args))


def layout_tree(
    session,
    *,
    window_id: int | None = None,
    max_depth: int | None = None,
    include_debug: bool | None = None,
) -> Any:
    """Walk the full widget/layout tree (incl. widgets the AT tree prunes) with
    bounds + types.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
        "max_depth": max_depth,
        "include_debug": include_debug,
    }
    return session.call("layout_tree", **_present(args))


def inspect_node(session, node: int, *, window_id: int | None = None) -> Any:
    """One widget's full layout record: type, bounds, flags, tree position, Debug
    repr.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
    }
    return session.call("inspect_node", **_present(args))


def find_node(
    session,
    *,
    window_id: int | None = None,
    role: str | None = None,
    label: str | None = None,
) -> Any:
    """Find the first node matching a role and/or label.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
        "role": role,
        "label": label,
    }
    return session.call("find_node", **_present(args))


def assert_node(
    session,
    node: int,
    kind: str,
    *,
    window_id: int | None = None,
    value: str | None = None,
    flag: bool | None = None,
) -> Any:
    """Assert a property of a node (role/label/value/toggled/expanded/…).

    Read-only tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "kind": kind,
        "value": value,
        "flag": flag,
    }
    return session.call("assert_node", **_present(args))


def list_windows(session, *, window_id: int | None = None) -> Any:
    """List the app's managed windows with ids, labels, and titles.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
    }
    return session.call("list_windows", **_present(args))


def invoke_action(
    session,
    node: int,
    action: str,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Invoke an AccessKit action on a node (click/focus/expand/…).

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "action": action,
        "settle": settle,
    }
    return session.call("invoke_action", **_present(args))


def focus_node(
    session,
    node: int,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Move focus to a node.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "settle": settle,
    }
    return session.call("focus_node", **_present(args))


def set_value(
    session,
    node: int,
    value: str,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Set a node's value via the SetValue AT action.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "value": value,
        "settle": settle,
    }
    return session.call("set_value", **_present(args))


def expand(
    session,
    node: int,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Expand a disclosure / tree node.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "settle": settle,
    }
    return session.call("expand", **_present(args))


def collapse(
    session,
    node: int,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Collapse a disclosure / tree node.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "settle": settle,
    }
    return session.call("collapse", **_present(args))


def scroll(
    session,
    node: int,
    *,
    window_id: int | None = None,
    dx: float | None = None,
    dy: float | None = None,
    ctrl: bool | None = None,
    shift: bool | None = None,
    alt: bool | None = None,
    meta: bool | None = None,
    command: bool | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Scroll the widget under a node by a pixel delta, with optional modifiers
    (ctrl/shift/alt/meta/command). A modifier-held wheel is its own gesture,
    e.g. Ctrl+wheel to zoom. Use `command` for the platform accelerator
    (Control on Windows/Linux, Command on macOS); `ctrl` is literal Control.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "dx": dx,
        "dy": dy,
        "ctrl": ctrl,
        "shift": shift,
        "alt": alt,
        "meta": meta,
        "command": command,
        "settle": settle,
    }
    return session.call("scroll", **_present(args))


def inject_pointer(
    session,
    x: float,
    y: float,
    *,
    window_id: int | None = None,
    action: str | None = None,
    ctrl: bool | None = None,
    shift: bool | None = None,
    alt: bool | None = None,
    meta: bool | None = None,
    command: bool | None = None,
    button: str | None = None,
    kind: str | None = None,
    pointer_id: int | None = None,
    pressure: float | None = None,
    tilt: Sequence[float] | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Inject a pointer event at a point: action = click (default), double_click,
    down, up or move; button = primary (default), secondary, middle, back,
    forward; kind = mouse (default), touch or pen; with optional
    ctrl/shift/alt/meta/command held for the press and release. A touch or pen
    enters through the tree's pointer door, so the kind reaches the hit test,
    the slop, the hover rules and the arbitration; pen carries `pressure`
    (0..1) and `tilt` ([x, y] degrees). Continue a contact a previous call
    left down with `pointer_id` from query_pointers (on a move or an up; a
    down and a click mint their own), or drive a whole gesture with
    inject_touch_sequence. Use `command` for the platform accelerator (Control
    on Windows/Linux, Command on macOS) — accelerator-click to extend a
    selection is `command`, not `ctrl`. Unknown names and unknown fields are
    refused rather than defaulted, and so are pressure/tilt/pointer_id on a
    mouse.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "x": x,
        "y": y,
        "action": action,
        "ctrl": ctrl,
        "shift": shift,
        "alt": alt,
        "meta": meta,
        "command": command,
        "button": button,
        "kind": kind,
        "pointer_id": pointer_id,
        "pressure": pressure,
        "tilt": tilt,
        "settle": settle,
    }
    return session.call("inject_pointer", **_present(args))


def right_click(
    session,
    node: int,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Right-click a node (secondary button at its point) to open its context
    menu.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "settle": settle,
    }
    return session.call("right_click", **_present(args))


def inject_key(
    session,
    key: str,
    *,
    window_id: int | None = None,
    ctrl: bool | None = None,
    shift: bool | None = None,
    alt: bool | None = None,
    meta: bool | None = None,
    command: bool | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Inject a key press (with optional modifiers) to the focused widget. Use
    `command` for any accelerator chord (Control on Windows/Linux, Command on
    macOS) — a shortcut declared Ctrl+S resolves to the Command chord on
    macOS, so `ctrl` there injects a key that matches no binding and still
    reports success. `ctrl` stays literal Control, for chords that really are
    Control everywhere (Ctrl+Tab).

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "key": key,
        "ctrl": ctrl,
        "shift": shift,
        "alt": alt,
        "meta": meta,
        "command": command,
        "settle": settle,
    }
    return session.call("inject_key", **_present(args))


def type_text(
    session,
    node: int,
    text: str,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Focus a node and type text into it.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "text": text,
        "settle": settle,
    }
    return session.call("type_text", **_present(args))


def type_ime(
    session,
    node: int,
    *,
    window_id: int | None = None,
    preedit: str | None = None,
    commit: str | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Drive IME composition / commit on a node.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "preedit": preedit,
        "commit": commit,
        "settle": settle,
    }
    return session.call("type_ime", **_present(args))


def drag_node(
    session,
    node: int,
    *,
    window_id: int | None = None,
    to_node: int | None = None,
    to_x: float | None = None,
    to_y: float | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Drag from a node to another node or a point.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "to_node": to_node,
        "to_x": to_x,
        "to_y": to_y,
        "settle": settle,
    }
    return session.call("drag_node", **_present(args))


def inject_touch_sequence(
    session,
    steps: Sequence[Mapping[str, Any]],
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Drive a whole multi-touch gesture in one call — a list of steps, each
    naming a finger slot, a phase (down/move/up/cancel), a point, and how many
    simulated milliseconds to advance first — and report the arbitration after
    every step: the frozen touch_action, every competitor with its role and
    state, and the winner. Fingers are named by slot, not by id: identities
    are minted by the framework and the reply says which one each slot got. A
    sequence that stops short of its `up` leaves the finger down, which is how
    a live arbitration stays observable.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "steps": steps,
        "settle": settle,
    }
    return session.call("inject_touch_sequence", **_present(args))


def pinch(
    session,
    ax0: float,
    ay0: float,
    bx0: float,
    by0: float,
    ax1: float,
    ay1: float,
    bx1: float,
    by1: float,
    *,
    window_id: int | None = None,
    steps: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Two fingers moving from one span to another — the pinch a zoomable surface
    reads. Both land before either moves, because the recogniser's reference
    span is the distance between the landings.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "ax0": ax0,
        "ay0": ay0,
        "bx0": bx0,
        "by0": by0,
        "ax1": ax1,
        "ay1": ay1,
        "bx1": bx1,
        "by1": by1,
        "steps": steps,
        "settle": settle,
    }
    return session.call("pinch", **_present(args))


def fling(
    session,
    from_x: float,
    from_y: float,
    to_x: float,
    to_y: float,
    over_ms: int,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """One finger travelling from one point to another over N simulated
    milliseconds and released while still moving — the shape a kinetic coast
    is handed off from. A drag latches on distance; a fling hands a velocity
    to the scroller, so the duration is the whole difference.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "from_x": from_x,
        "from_y": from_y,
        "to_x": to_x,
        "to_y": to_y,
        "over_ms": over_ms,
        "settle": settle,
    }
    return session.call("fling", **_present(args))


def long_press(
    session,
    x: float,
    y: float,
    *,
    window_id: int | None = None,
    kind: str | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Press at a point, hold for exactly the device's long-press threshold,
    release. The hold is read off the active input profile for the pointer
    kind, so the call means 'hold long enough' without the script knowing the
    number.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "x": x,
        "y": y,
        "kind": kind,
        "settle": settle,
    }
    return session.call("long_press", **_present(args))


def cancel_pointer(
    session,
    pointer_id: int,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Revoke a live pointer the way the system does (a compositor grab, a lost
    capture). Not an up: no tap completes and every widget working on the
    pointer is told. Takes an id from query_pointers.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "pointer_id": pointer_id,
        "settle": settle,
    }
    return session.call("cancel_pointer", **_present(args))


def query_pointers(session, *, window_id: int | None = None) -> Any:
    """Every live pointer with its id, kind, position, pressure/tilt, capture,
    frozen touch_action, competitors and arbitration winner. How to learn the
    id of a contact left down by anything but inject_touch_sequence, whose
    reply names its own.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
    }
    return session.call("query_pointers", **_present(args))


def set_density(
    session,
    density: str,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Switch the app's target density (compact / comfortable / touch). WARNING:
    a density change rebuilds every widget, so every node id captured before
    it is dead — re-snapshot or re-find afterwards. Setting the density it
    already has is a no-op and keeps the ids.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "density": density,
        "settle": settle,
    }
    return session.call("set_density", **_present(args))


def get_overlays(session, *, window_id: int | None = None) -> Any:
    """List active overlays (popovers, menus, tooltips, dialogs).

    Read-only tool.
    """
    args = {
        "window_id": window_id,
    }
    return session.call("get_overlays", **_present(args))


def get_shortcuts(session, *, window_id: int | None = None) -> Any:
    """List effective keyboard shortcuts and their bindings.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
    }
    return session.call("get_shortcuts", **_present(args))


def list_live_regions(session, *, window_id: int | None = None) -> Any:
    """List nodes that are live regions (polite/assertive).

    Read-only tool.
    """
    args = {
        "window_id": window_id,
    }
    return session.call("list_live_regions", **_present(args))


def pull_announcements(
    session,
    *,
    window_id: int | None = None,
    since_seq: int | None = None,
) -> Any:
    """Drain captured live-region announcements since a sequence number.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
        "since_seq": since_seq,
    }
    return session.call("pull_announcements", **_present(args))


def advance_clock(session, millis: int, *, window_id: int | None = None) -> Any:
    """Advance the simulation clock by N milliseconds.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "millis": millis,
    }
    return session.call("advance_clock", **_present(args))


def settle(
    session,
    *,
    window_id: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Run animations / layout to quiescence, then re-sync the tree.

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "settle": settle,
    }
    return session.call("settle", **_present(args))


def wait_for_condition(
    session,
    kind: str,
    *,
    window_id: int | None = None,
    node: int | None = None,
    role: str | None = None,
    label: str | None = None,
    expected: str | None = None,
    version: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Poll until a condition holds (node exists / value / gone / version).

    Mutating tool.
    """
    args = {
        "window_id": window_id,
        "kind": kind,
        "node": node,
        "role": role,
        "label": label,
        "expected": expected,
        "version": version,
        "settle": settle,
    }
    return session.call("wait_for_condition", **_present(args))


def screenshot(
    session,
    *,
    window_id: int | None = None,
    node: int | None = None,
    settle: Mapping[str, Any] | None = None,
) -> Any:
    """Render the window (or a node's bounds) to a PNG image block.

    Read-only tool.
    """
    args = {
        "window_id": window_id,
        "node": node,
        "settle": settle,
    }
    return session.call("screenshot", **_present(args))


#: Every tool name, in `TOOL_CATALOG` order. The Rust conformance test in
#: `crates/teksilo-automation/src/mcp_schema.rs` compares this against the
#: catalog, so a tool added there without regenerating this file fails the build.
TOOL_NAMES = (
    "snapshot_tree",
    "read_node",
    "layout_tree",
    "inspect_node",
    "find_node",
    "assert_node",
    "list_windows",
    "invoke_action",
    "focus_node",
    "set_value",
    "expand",
    "collapse",
    "scroll",
    "inject_pointer",
    "right_click",
    "inject_key",
    "type_text",
    "type_ime",
    "drag_node",
    "inject_touch_sequence",
    "pinch",
    "fling",
    "long_press",
    "cancel_pointer",
    "query_pointers",
    "set_density",
    "get_overlays",
    "get_shortcuts",
    "list_live_regions",
    "pull_announcements",
    "advance_clock",
    "settle",
    "wait_for_condition",
    "screenshot",
)

#: The tools that mutate UI state (and so accept a `settle` policy).
MUTATING = frozenset(
    name
    for name, mutating in (
        ("snapshot_tree", False),
        ("read_node", False),
        ("layout_tree", False),
        ("inspect_node", False),
        ("find_node", False),
        ("assert_node", False),
        ("list_windows", False),
        ("invoke_action", True),
        ("focus_node", True),
        ("set_value", True),
        ("expand", True),
        ("collapse", True),
        ("scroll", True),
        ("inject_pointer", True),
        ("right_click", True),
        ("inject_key", True),
        ("type_text", True),
        ("type_ime", True),
        ("drag_node", True),
        ("inject_touch_sequence", True),
        ("pinch", True),
        ("fling", True),
        ("long_press", True),
        ("cancel_pointer", True),
        ("query_pointers", False),
        ("set_density", True),
        ("get_overlays", False),
        ("get_shortcuts", False),
        ("list_live_regions", False),
        ("pull_announcements", False),
        ("advance_clock", True),
        ("settle", True),
        ("wait_for_condition", True),
        ("screenshot", False),
    )
    if mutating
)
