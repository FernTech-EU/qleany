# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Reaching a widget that is not on screen yet.

Everything in this module is knowledge that was probe-local in the harness this
library generalises, rediscovered independently at least three times, and
misdiagnosed as "the feature is broken" on each of the first two. It is written
down here so the fourth probe does not have to learn it.

**1. A virtualized view only realises the rows in (and slightly past) its
viewport.** `ListView`, `TreeView`, `TableView`, `TreeTableView` and `GridView`
all do this. A row below the fold has no widget, so it has no accessibility node
at all. `tree.find()` returning `None` means *"not on screen"*, and a probe that
reads it as *"absent"* reports a working feature as missing.

**2. A row scrolled back into view is rebuilt, and a rebuilt widget gets a fresh
node id.** Ids are stable for a widget's *lifetime*, and a virtualizer ends that
lifetime every time a row leaves its buffer. So: never hold an id across a
scroll. Re-find between the scroll and the click — every helper here does, and
:func:`reveal_and_click` exists because forgetting it is a click on a dead id
that reports `NOT_FOUND` for a row sitting right there.

**3. Pointer-clicking a node's reported bounds does not work for a row laid out
below the viewport.** The bounds are perfectly real; nothing is painted at them;
the click lands on empty chrome and the pane never changes. The failure looks
exactly like a wrong selector. Teksilo scrolls the *focused* row into view, so
keyboard navigation sidesteps the whole problem — which is what
:func:`select_via_keyboard` is for.

**4. Scroll direction must be discovered, not assumed.** A fixed "wheel down
until found" can never reach a row *above* the starting position, and the result
is indistinguishable from the row not existing. :func:`scroll_until` flips
direction the first time a scroll changes nothing, and recognises "both ends
reached" by comparing a signature of the visible labels before and after.

**5. A control can be absent from the AT tree entirely.** An overflowed
`SegmentedControl` cell, a collapsed `Toolbar` command and a parked `Switcher`
branch are *dormant* — no widget, no node, and no amount of scrolling will
produce one. :func:`first_available` is the fallback chain for that: try the
direct route, then the overflow menu, then the keyboard.
"""

from __future__ import annotations

import time
from typing import Any, Callable, Iterable, Mapping, Sequence

from . import tree as _tree

#: One wheel notch, in logical pixels. Roughly a third of a viewport at default
#: density — big enough to make progress, small enough not to jump a row past
#: the realization buffer between two snapshots.
SCROLL_STEP = 1200.0

#: How long to let a scrolled view repaint before re-snapshotting. A `settle`
#: covers animations and layout; this covers the frame after it.
SETTLE_PAUSE = 0.6


def visible_signature(rows: Iterable[Mapping]) -> tuple:
    """A comparable fingerprint of what is on screen.

    Labels, sorted — not ids, and not positions. Ids churn on every rebuild the
    scroll itself causes, so an id-based signature reports "something changed"
    on a scroll that moved nothing; positions move under a smooth-scroll
    animation for the same reason. Labels are the one thing that is stable while
    the view is stable and different when it is not.
    """
    return tuple(sorted(str(row.get("label") or "") for row in rows))


# ---------------------------------------------------------------------------
# Scrolling
# ---------------------------------------------------------------------------


def scroll_until(session: Any, container: Mapping | int,
                 predicate: Callable[[Sequence[Mapping]], Any], *,
                 rows: Callable[[], Sequence[Mapping]] | None = None,
                 role: str | None = None,
                 step: float = SCROLL_STEP, limit: int = 24,
                 pause: float = SETTLE_PAUSE,
                 window_id: int | None = None) -> Any:
    """Wheel `container` until `predicate` finds something, discovering direction.

    `predicate` receives the currently visible rows and returns the thing the
    caller wants (a node, usually) or a falsy value. `rows` supplies them; by
    default, every node whose role is `role`, or every node when no role is
    given.

    Direction is **discovered**: it starts with `step` as given — positive `dy`
    moves *down* a teksilo list — and the first scroll that leaves the visible
    signature unchanged flips it. A second unchanged scroll after the flip means
    both ends have been reached, and *that* is the only honest "the row really
    is not there", which is why this returns `None` rather than raising: a
    caller usually wants to try another route. Pass a negative `step` to start
    upwards.

    Every iteration re-snapshots. The rows from before a scroll are stale by
    construction (rule 2), so reusing them would search a view that no longer
    exists.

    ## Three measured facts this works around

    **A teksilo list scrolls down on POSITIVE `dy`.** An earlier version of this
    function forced `-abs(step)` on the belief that negative moved down. It does
    not, and the sign was not overridable, so a search that started at the top of
    a list scrolled *further up*, saw nothing change, flipped once, and reported
    "both ends reached" with the view never having moved.

    **The first `scroll` of a session moves nothing.** The tool's own
    `pointer_move` establishes hover, and the wheel routes by hover, so the
    effect only begins with the next event. The priming scroll below is
    therefore not counted as a direction signal — treating it as one is what
    made a stationary first frame look like the end of the list.

    **A notch smaller than the realization buffer produces no observable
    change**, because the rows it would reveal were already realised. That is
    indistinguishable from the end of the list to a signature comparison, which
    is why `SCROLL_STEP` is a whole viewport rather than a few rows.
    """
    container_id = container if isinstance(container, int) else container.get("id")

    def visible() -> Sequence[Mapping]:
        if rows is not None:
            return rows()
        snapshot = _tree.nodes(session, window_id=window_id)
        if role is None:
            return snapshot
        return [n for n in snapshot if _tree.matches(n, role=role)]

    current = visible()
    hit = predicate(current)
    if hit:
        return hit

    delta = step
    flipped = False

    # Prime the hover. This scroll's result is deliberately ignored: the wheel
    # routes by hover and the tool's own pointer_move only establishes it as a
    # side effect, so the first notch of a session lands nowhere. Counting it
    # would flip the direction before a single row had moved.
    session.call("scroll", node=container_id, dx=0.0, dy=delta)
    time.sleep(pause)
    current = visible()
    hit = predicate(current)
    if hit:
        return hit

    for _ in range(limit):
        before = visible_signature(current)
        session.call("scroll", node=container_id, dx=0.0, dy=delta)
        time.sleep(pause)
        current = visible()
        hit = predicate(current)
        if hit:
            return hit
        if visible_signature(current) == before:
            if flipped:
                return None  # both ends reached; it genuinely is not in this view
            delta, flipped = -delta, True
    return None


def scroll_until_found(session: Any, container: Mapping | int, *,
                       role: str | None = None, label: str | None = None,
                       label_contains: str | None = None,
                       pred: Callable[[Mapping], bool] | None = None,
                       row_role: str | None = None,
                       step: float = SCROLL_STEP, limit: int = 24,
                       pause: float = SETTLE_PAUSE,
                       window_id: int | None = None) -> dict | None:
    """Scroll `container` until a node matching the criteria is realised.

    The common case of :func:`scroll_until`. `row_role` narrows what counts as a
    row for the change-detection signature (e.g. `"TreeItem"`); `role` and the
    label arguments select the node being looked for.
    """
    criteria = dict(role=role, label=label, label_contains=label_contains, pred=pred)

    def match(candidates: Sequence[Mapping]) -> dict | None:
        for node in candidates:
            if _tree.matches(node, **criteria):
                return node
        return None

    return scroll_until(session, container, match, role=row_role, step=step,
                        limit=limit, pause=pause, window_id=window_id)


# ---------------------------------------------------------------------------
# Clicking
# ---------------------------------------------------------------------------


def click(session: Any, node: Mapping, *, settle: bool = True) -> None:
    """Activate a node — its AT `click` action when it has one, a pointer otherwise.

    Prefer the action: it needs no coordinates, so it cannot miss, and it is
    what an assistive technology would do. The pointer is the fallback for a
    node that advertises no action (a decorative surface, a custom hit region),
    and it carries rule 3's caveat — aim only at something actually painted.
    """
    if "click" in (node.get("actions") or []):
        session.call("invoke_action", node=node["id"], action="click")
    else:
        x, y = _tree.center(node)
        session.call("inject_pointer", x=x, y=y, action="click")
    if settle:
        session.settle()


def reveal_and_click(session: Any, refind: Callable[[], Mapping | None], *,
                     pause: float = 0.4, settle: bool = True) -> dict | None:
    """Scroll a node into view, **re-find it**, then click it.

    `refind` is a callable, not a node, and that is the whole point.
    `scroll_into_view` rebuilds the row it scrolls to, so the node handed in
    before the scroll is dead after it — clicking its id reports `NOT_FOUND` for
    a row that is sitting right there on screen. Re-finding between the two
    calls is the fix, and making the caller pass a finder is how this helper
    guarantees it.

    Returns the node it actually clicked, or `None` if the refind came up empty.
    """
    node = refind()
    if node is None:
        return None
    if "scroll_into_view" in (node.get("actions") or []):
        session.call("invoke_action", node=node["id"], action="scroll_into_view")
        time.sleep(pause)
        node = refind() or node
    click(session, node, settle=settle)
    return node


# ---------------------------------------------------------------------------
# Keyboard
# ---------------------------------------------------------------------------


def select_via_keyboard(session: Any, reached: Callable[[], Any], *,
                        anchor: Mapping | None = None,
                        keys: Sequence[str] = ("Down", "Up"),
                        steps: int = 14, pause: float = 0.35) -> Any:
    """Walk to a row with the arrow keys, letting the view do the scrolling.

    The route that works when clicking bounds does not (rule 3): teksilo scrolls
    the *focused* row into view, so stepping focus through a list reveals rows a
    pointer could never reach. Click a row that **is** visible to put focus in
    the view, then step.

    Three ways this is easy to get wrong, each of which reported a reachable
    target as unreachable:

    * **Walking one direction only.** An anchor is a guess about list order, so
      a target above it can never be reached — indistinguishable from its
      absence. Both directions are tried, re-anchoring between them.
    * **Clicking the anchor unconditionally.** A view that remembers its last
      selection can *start* on the target, and the first click leaves it. So
      `reached()` is consulted before anything is pressed.
    * **Naming the anchor by its English label.** The anchor lookup used to be
      a hard-coded page name, so on a French desktop it found nothing and
      returned before pressing a single key — reported as "could not select the
      page" for a page sitting right there. Pass a node you have *already*
      located; that is why `anchor` is a node and not a string.
    """
    hit = reached()
    if hit:
        return hit
    if anchor is None:
        return None

    for key in keys:
        try:
            x, y = _tree.center(anchor)
        except ValueError:
            return None
        session.call("inject_pointer", x=x, y=y, action="click")
        session.settle()
        time.sleep(pause)
        for _ in range(steps):
            hit = reached()
            if hit:
                return hit
            session.call("inject_key", key=key)
            session.settle()
            time.sleep(pause)
    return reached()


# ---------------------------------------------------------------------------
# Fallbacks
# ---------------------------------------------------------------------------


def first_available(*strategies: Callable[[], Any]) -> Any:
    """Try each strategy in order; return the first truthy result.

    For rule 5 — a control that is not in the accessibility tree at all. An
    overflowed `SegmentedControl` cell, a `Toolbar` command collapsed into the
    chevron menu and a parked `Switcher` branch are all *dormant*: no widget, no
    node, and no amount of scrolling produces one, because there is nothing to
    realise. The control is reachable by another route — the overflow menu, a
    shortcut, a keyboard walk — and this is how a probe states the routes in
    preference order instead of failing on the first::

        cell = first_available(
            lambda: find(s, role="Tab", label="Details"),
            lambda: open_overflow_then(s, "Details"),
            lambda: select_via_keyboard(s, reached, anchor=first_tab),
        )

    An exception from one strategy is *not* swallowed: a broken strategy and an
    inapplicable one are different things, and hiding the first behind the
    second is how a probe ends up silently testing its own fallback.
    """
    for strategy in strategies:
        result = strategy()
        if result:
            return result
    return None


def expand_all(session: Any, *, role: str = "TreeItem", passes: int = 8,
               window_id: int | None = None) -> int:
    """Expand every collapsed row, repeatedly, until nothing more opens.

    Repeated because expanding a row realises its children, which may themselves
    be collapsed — one pass reaches only the depth that was already on screen.
    Returns how many rows were expanded in total.
    """
    total = 0
    for _ in range(passes):
        opened = 0
        for node in _tree.find_all(session, role=role, window_id=window_id):
            if node.get("expanded") is False and "expand" in (node.get("actions") or []):
                session.call("expand", node=node["id"])
                opened += 1
        if not opened:
            break
        session.settle()
        total += opened
    return total
