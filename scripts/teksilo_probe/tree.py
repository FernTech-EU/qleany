# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Reading the accessibility tree: snapshot, search, geometry.

Nodes are plain dicts, exactly as `SemanticNode` serialises them: `id`, `role`
(an AccessKit role's Debug name, e.g. `"Button"`), `label`, `value`,
`description`, `toggled`, `expanded`, `selected`, `level`, `disabled`,
`focused`, `live`, `numeric_value`, `bounds`, `actions`, `children`, `text`.

Two things about this tree that a caller has to know before searching it:

* **Node ids are stable for a widget's lifetime, not for a probe's.** They
  survive relayout, a theme change and a locale change; a structural rebuild
  that recreates the widget mints a new one. Scrolling a virtualized row out of
  and back into view *is* such a rebuild. Never cache an id across anything that
  can rebuild — re-find instead.
* **A node's absence is not the widget's absence.** A virtualized `ListView` /
  `TreeView` / `TableView` only realises the rows in (and slightly past) its
  viewport, so a row below the fold has no widget and therefore no AT node at
  all. `find()` returning `None` means "not on screen", not "not there".
  :mod:`teksilo_probe.navigate` is where that is handled.
"""

from __future__ import annotations

import time
from typing import Any, Callable, Mapping, Sequence


def nodes(source: Any, *, window_id: int | None = None) -> list[dict]:
    """Every node of a fresh snapshot, flattened.

    Accepts a `Session` (snapshots it) or an already-taken payload / list of
    nodes (returns it), so every helper here takes `source` and a caller can
    snapshot once and search many times.
    """
    if isinstance(source, Mapping):
        return list(source.get("nodes") or [])
    if isinstance(source, (list, tuple)):
        return list(source)
    payload = source.call("snapshot_tree", **({"window_id": window_id} if window_id else {}))
    if isinstance(payload, Mapping):
        return list(payload.get("nodes") or [])
    return list(payload or [])


def _norm(text: Any) -> str:
    return str(text or "").strip().lower()


def matches(node: Mapping, *, role: str | None = None, label: str | None = None,
            label_contains: str | None = None, value: str | None = None,
            pred: Callable[[Mapping], bool] | None = None) -> bool:
    """Does one node satisfy every stated criterion?

    Role and label comparisons are case-insensitive and whitespace-trimmed: a
    role arrives as a Debug name (`"TreeItem"`) and a label as whatever the app
    put there, and a probe that has to get the capitalisation right is a probe
    that breaks on a rename that changed nothing.
    """
    if role is not None and _norm(node.get("role")) != _norm(role):
        return False
    if label is not None and _norm(node.get("label")) != _norm(label):
        return False
    if label_contains is not None and _norm(label_contains) not in _norm(node.get("label")):
        return False
    if value is not None and _norm(node.get("value")) != _norm(value):
        return False
    if pred is not None and not pred(node):
        return False
    return True


def find(source: Any, *, role: str | None = None, label: str | None = None,
         label_contains: str | None = None, value: str | None = None,
         pred: Callable[[Mapping], bool] | None = None,
         window_id: int | None = None) -> dict | None:
    """The first matching node, or `None`.

    `None` does **not** mean the widget is absent — see the module docstring.
    """
    for node in nodes(source, window_id=window_id):
        if matches(node, role=role, label=label, label_contains=label_contains,
                   value=value, pred=pred):
            return node
    return None


def find_all(source: Any, *, role: str | None = None, label: str | None = None,
             label_contains: str | None = None, value: str | None = None,
             pred: Callable[[Mapping], bool] | None = None,
             window_id: int | None = None) -> list[dict]:
    """Every matching node, in AT order."""
    return [
        node for node in nodes(source, window_id=window_id)
        if matches(node, role=role, label=label, label_contains=label_contains,
                   value=value, pred=pred)
    ]


def labels(source: Any, *, role: str | None = None, window_id: int | None = None) -> list[str]:
    """Every non-empty label on screen, in AT order.

    The first thing to print when a `find` comes back empty: it answers "is the
    label spelled differently?" and "is the view even up yet?" in one line.
    """
    return [
        str(node.get("label"))
        for node in nodes(source, window_id=window_id)
        if node.get("label") and (role is None or _norm(node.get("role")) == _norm(role))
    ]


def children_of(source: Any, node: Mapping | int, *, window_id: int | None = None) -> list[dict]:
    """A node's direct children, resolved from a snapshot."""
    snapshot = nodes(source, window_id=window_id)
    by_id = {n.get("id"): n for n in snapshot}
    parent = by_id.get(node if isinstance(node, int) else node.get("id"))
    if parent is None:
        return []
    return [by_id[cid] for cid in (parent.get("children") or []) if cid in by_id]


def descendants(source: Any, node: Mapping | int, *,
                window_id: int | None = None) -> list[dict]:
    """A node's whole subtree, depth-first, excluding itself."""
    snapshot = nodes(source, window_id=window_id)
    by_id = {n.get("id"): n for n in snapshot}
    root = node if isinstance(node, int) else node.get("id")
    out: list[dict] = []
    seen: set = set()
    stack = list(reversed((by_id.get(root) or {}).get("children") or []))
    while stack:
        nid = stack.pop()
        if nid in seen or nid not in by_id:
            continue
        seen.add(nid)
        current = by_id[nid]
        out.append(current)
        stack.extend(reversed(current.get("children") or []))
    return out


# ---------------------------------------------------------------------------
# Geometry
# ---------------------------------------------------------------------------


def bounds(node: Mapping | None) -> dict:
    """A node's bounds, or an empty dict. Logical pixels, screen-projected."""
    return dict((node or {}).get("bounds") or {})


def center(node: Mapping) -> tuple[float, float]:
    """The middle of a node's bounds, for a synthetic pointer.

    ⚠ Correct only for a node that is actually **painted**. A virtualized row
    laid out below its scroll viewport has perfectly real bounds and nothing
    drawn at them, so a click there lands on empty chrome and the failure looks
    exactly like a wrong selector. See :mod:`teksilo_probe.navigate`.
    """
    box = bounds(node)
    if not box:
        raise ValueError(f"node {node.get('id')} has no bounds to aim at")
    return (box.get("x", 0.0) + box.get("width", 0.0) / 2.0,
            box.get("y", 0.0) + box.get("height", 0.0) / 2.0)


def in_region(node: Mapping, *, min_x: float | None = None, max_x: float | None = None,
              min_y: float | None = None, max_y: float | None = None) -> bool:
    """Is a node's **origin** inside the given band?

    The origin rather than the whole rectangle, because the question this
    answers is "which strip of chrome does this belong to" — a leading rail, a
    title bar, a status line. A node wider than its strip (a row whose bounds
    run past a narrow rail) is still *in* it; testing containment of the whole
    rectangle would say otherwise and drop the very nodes being looked for.

    A node with no bounds is not in any region — `False`, never a crash.
    """
    box = bounds(node)
    if not box:
        return False
    x, y = box.get("x"), box.get("y")
    if x is None or y is None:
        return False
    if min_x is not None and x < min_x:
        return False
    if max_x is not None and x > max_x:
        return False
    if min_y is not None and y < min_y:
        return False
    if max_y is not None and y > max_y:
        return False
    return True


def text_of(node: Mapping) -> str:
    """A node's value and label as one lowercase haystack, for a loose match."""
    return _norm(f"{node.get('value') or ''} {node.get('label') or ''}")


# ---------------------------------------------------------------------------
# Waiting
# ---------------------------------------------------------------------------


def wait_for(session: Any, markers: str | Sequence[str], *, timeout: float = 30.0,
             interval: float = 0.5, window_id: int | None = None) -> bool:
    """Poll until any of `markers` appears in some node's label or value.

    A single snapshot taken straight after the bridge connects is not enough.
    The bridge answers as soon as the event loop is up, which is before the app
    has opened a document, run a migration, or populated a panel — so checking
    once reports "it did not load" for something that loads perfectly 400 ms
    later, a false failure that costs a whole launch to diagnose. Wait; never
    sleep-and-hope.
    """
    if isinstance(markers, str):
        markers = (markers,)
    wanted = [_norm(m) for m in markers]
    deadline = time.monotonic() + timeout
    while True:
        for node in nodes(session, window_id=window_id):
            haystack = text_of(node)
            if any(marker in haystack for marker in wanted):
                return True
        if time.monotonic() >= deadline:
            return False
        time.sleep(interval)


def wait_for_node(session: Any, *, timeout: float = 30.0, interval: float = 0.5,
                  window_id: int | None = None, **criteria: Any) -> dict | None:
    """Poll until a node matching `criteria` appears. Returns it, or `None`."""
    deadline = time.monotonic() + timeout
    while True:
        hit = find(session, window_id=window_id, **criteria)
        if hit is not None:
            return hit
        if time.monotonic() >= deadline:
            return None
        time.sleep(interval)
