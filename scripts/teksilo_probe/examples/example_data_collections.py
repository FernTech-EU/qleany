#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Virtualization: what a `ListView` / `TreeView` does and does not publish.

Proves five things about a virtualized data view, against a live
`cargo run -p data-collections` (200 `ListView` rows, a `TreeView` beside them):

1. **A row below the fold has no accessibility node at all.** The model holds
   200 items; the view realises ~17. `tree.find(label="Item 200")` comes back
   `None`, and that `None` means *"not on screen"*, not *"not there"*.
2. **`navigate.scroll_until_found` reaches one anyway** — it wheels the
   container until the row is realised.
3. **A row scrolled out of and back into view is rebuilt, and a rebuilt widget
   gets a fresh node id.** `Item 1`'s id before the scroll is not `Item 1`'s id
   after it.
4. **Holding an id across a scroll is the bug that follows from 3.** The stale
   id answers `NOT_FOUND` for a row sitting right there on screen.
5. **Keyboard navigation is the exact route to a distant row.** One `End` puts
   `Item 200` on screen *and selects it* — one call, no direction discovery
   and no animation to wait out, where the wheel takes eight notches to get
   there and selects nothing.

Why this needs a live app: none of it is observable from a unit test. The
realization window is decided by the viewport the compositor gives the widget,
the ids are minted by the arena as rows are built and destroyed, and the
scroll is an animation the frame loop drives. A headless `WidgetTree` test can
assert the layout maths; only a running app can tell you what an agent driving
it will actually see.

    python3 example_data_collections.py [--binary PATH] [--keep]

Exit codes: 0 every check passed, 1 the probe could not run, 2 it ran cleanly
and the behaviour is absent. See `teksilo_probe.report`.
"""

from __future__ import annotations

import sys
import time
from pathlib import Path


def _bootstrap() -> None:
    """Put the `teksilo_probe` package on `sys.path`.

    Two layouts, both real. In a consumer's repository this file sits at
    `scripts/teksilo_probe/examples/`, so the importable directory is two
    levels up (`scripts/`). In the framework's own tree it sits beside the
    package instead. Walking up and asking *which ancestor contains a
    `teksilo_probe/__init__.py`* answers both without either hard-coding the
    other's depth.
    """
    for ancestor in Path(__file__).resolve().parents:
        candidate = ancestor.parent if ancestor.name == "teksilo_probe" else ancestor
        if (candidate / "teksilo_probe" / "__init__.py").is_file():
            sys.path.insert(0, str(candidate))
            return


_bootstrap()

from teksilo_probe import Report, launch_and_attach, navigate, tree  # noqa: E402

#: The workspace binary this probe drives. `resolve.app_binary` turns it into a
#: path under whichever target directory cargo is actually using — never a
#: path spelled out here, which would pass on one machine and no other.
APP = "data-collections"

#: One wheel notch, in logical pixels. Deliberately large: this list's rows are
#: 40–64 dp and its realization buffer is ~14 rows deep, so a notch smaller
#: than the buffer moves nothing the snapshot can see and `scroll_until`'s
#: change detection reads it as "the end of the list".
STEP = 1200.0

#: How long to let the wheel's scroll animation finish before re-reading the
#: tree. The `scroll` tool settles the app, but the smooth-scroll tween outlives
#: that budget on a list this long.
SCROLL_PAUSE = 0.6


def rows(session) -> list[dict]:
    """Every realised list row, in AT order.

    A teksilo `ListView` publishes `Role::ListBox` with `Role::ListBoxOption`
    rows — *not* `ListItem`, which is what an ARIA-shaped guess would look for
    and find nothing.
    """
    return tree.find_all(session, role="ListBoxOption")


def visible_span(session) -> str:
    """First..last realised row, for a note a reader can act on."""
    realised = rows(session)
    return f"{realised[0]['label']}..{realised[-1]['label']}" if realised else "(none)"


def list_container(session) -> dict:
    """The `ListView`'s own node — the thing a wheel or a focus is aimed at.

    Raises rather than returning `None`, and names what was on screen instead:
    a `TypeError` three lines later on `container["id"]` says nothing about
    which view was missing, and finding that out costs another launch.
    """
    found = tree.find(session, role="ListBox")
    if found is None:
        raise RuntimeError(
            "no ListBox on screen; roles present: "
            + ", ".join(sorted({str(n.get("role")) for n in tree.nodes(session)}))
        )
    return found


def open_tab(session, report: Report, title: str) -> None:
    """Switch the demo's `TabWidget` to a named tab and let it settle.

    Through the tab's own AT `click` action rather than a synthetic pointer:
    an action needs no coordinates, so it cannot miss, and it is what a screen
    reader would do.
    """
    tab = tree.find(session, role="Tab", label=title)
    if tab is None:
        raise RuntimeError(f"no {title!r} tab; tabs on screen: {tree.labels(session, role='Tab')}")
    navigate.click(session, tab)
    session.settle()


# ---------------------------------------------------------------------------
# The legs
# ---------------------------------------------------------------------------


def virtualization_leg(session, report: Report) -> dict | None:
    """A row below the fold has no node. Returns `Item 1` as it is *now*."""
    realised = rows(session)
    report.note(f"realised rows: {len(realised)} of 200 — {visible_span(session)}")
    report.check(
        0 < len(realised) < 200,
        f"the view realises a window of rows, not all 200 (got {len(realised)})",
    )

    first = tree.find(session, role="ListBoxOption", label="Item 1")
    last = tree.find(session, role="ListBoxOption", label="Item 200")
    report.check(first is not None, "'Item 1' is realised and has an AT node")
    report.check(
        last is None,
        "'Item 200' has NO AT node — it is below the fold, so no widget exists "
        "for it and `find` returns None. That is 'not on screen', not 'absent'",
    )
    return first


def scroll_leg(session, report: Report) -> None:
    """`scroll_until_found` realises a row the first snapshot could not see.

    Three things about wheeling a teksilo view that each cost a real debugging
    session, and all three are why this leg is not a one-liner:

    * **Positive `dy` scrolls *down* a teksilo list** (towards higher indices).
      `navigate.scroll_until` opens with a *negative* step, so on a list parked
      at the top its first notch is a clamped no-op; it sees the view unchanged,
      flips direction, and from then on is going the right way.
    * **The first `scroll` of a session moves nothing.** The tool hovers the
      target before delivering the wheel, and a wheel routes to whatever was
      hovered when it arrived — so the first call only establishes the hover.
      Priming it here means `scroll_until`'s direction discovery is reading real
      movement rather than that artefact, which is the difference between it
      finding the row and giving up after two iterations.
    The target is a row in the middle rather than the last one. Not because the
    wheel cannot reach the end — measured, it does: nine `dy = +1200` notches
    put `Item 188..Item 200` on screen and further notches are clamped no-ops.
    It is that `scroll_until`'s "has anything changed?" signature cannot tell
    *arrived at the end* from *this direction is wrong*, so a target at the very
    last row is decided by whether the sweep happened to reach it before the
    flip. A mid-list target exercises the same machinery without depending on
    that; `keyboard_leg` covers the end exactly.
    """
    container = list_container(session)
    for _ in range(2):
        session.call("scroll", node=container["id"], dx=0.0, dy=STEP)
        time.sleep(SCROLL_PAUSE)
    report.note(f"wheel primed; visible {visible_span(session)}")

    target = "Item 120"
    found = navigate.scroll_until_found(
        session,
        container,
        role="ListBoxOption",
        label=target,
        # What counts as "a row" for the has-anything-changed signature. Without
        # it the signature is every label in the window — including the static
        # header text, which never changes and would mask a scroll that did.
        row_role="ListBoxOption",
        step=STEP,
        pause=SCROLL_PAUSE,
        limit=30,
    )
    report.check(
        found is not None,
        f"`scroll_until_found` realised {target!r}, which no snapshot had shown",
    )
    if found is not None:
        report.note(f"{target} id={found['id']}; visible {visible_span(session)}")


def fresh_id_leg(session, report: Report, before: dict) -> None:
    """A rebuilt row gets a fresh id, and the old one is dead.

    `before` is the `Item 1` node captured at the top of the list, from before
    any scrolling. Getting back to it uses the keyboard rather than the wheel
    (see `keyboard_leg`), because `Home` is exact and a wheel is not.
    """
    container = list_container(session)
    session.call("invoke_action", node=container["id"], action="focus")
    session.settle()
    session.call("inject_key", key="Home")
    session.settle()
    time.sleep(0.4)
    session.settle()

    after = tree.find(session, role="ListBoxOption", label="Item 1")
    report.check(after is not None, "'Item 1' is realised again after coming back to the top")
    if after is None:
        return

    report.check(
        after["id"] != before["id"],
        f"…and it is a NEW widget: id {before['id']} → {after['id']}. A "
        "virtualizer destroys a row that leaves its buffer, so an id is stable "
        "for a widget's lifetime, not for a probe's",
    )

    # The consequence, stated as an observation rather than a warning: the id
    # captured before the scroll now names nothing. This is what a probe that
    # cached a node sees — `NOT_FOUND` for a row it can see on screen — and the
    # symptom is indistinguishable from a wrong selector, which is exactly why
    # it is worth pinning.
    stale = session.try_call("read_node", node=before["id"])
    report.check(
        (not stale.ok) and stale.code == "NOT_FOUND",
        f"the id captured before the scroll is dead: read_node → {stale.code or 'ok'}. "
        "Re-find between a scroll and a click; never hold an id across one",
    )


def keyboard_leg(session, report: Report) -> None:
    """The route that reaches a row the wheel cannot.

    Clicking a node's reported bounds does not work for a row laid out below
    the viewport: the bounds are perfectly real, nothing is painted at them, and
    the click lands on empty chrome. Teksilo scrolls the *focused* row into
    view, so moving focus makes the view do the scrolling — one call, landing on
    a named row, where a wheel is a sweep whose stopping point you then have to
    read back.
    """
    container = list_container(session)
    session.call("invoke_action", node=container["id"], action="focus")
    session.settle()
    session.call("inject_key", key="End")
    session.settle()
    time.sleep(0.4)
    session.settle()

    last = tree.find(session, role="ListBoxOption", label="Item 200")
    report.check(
        last is not None,
        "one `End` realised 'Item 200' — exactly, and in one call",
    )
    if last is not None:
        report.check(
            bool(last.get("selected")),
            "…and selected it, so focus and selection followed the view",
        )
        report.note(f"visible {visible_span(session)}; Item 200 id={last['id']}")


def tree_leg(session, report: Report) -> None:
    """The same rule one level up: a collapsed subtree has no nodes either.

    A `TreeView` realises only the rows its flattened, expanded view contains.
    A child of a collapsed branch is absent for the same reason a row below the
    fold is — and, like a scrolled row, expanding its parent rebuilds the parent
    too, so even the node you just acted on has a new id afterwards.
    """
    open_tab(session, report, "TreeView")
    time.sleep(0.4)

    report.check(
        tree.find(session, role="TreeItem", label="Projects") is None,
        "'Projects' has no node while 'Documents' is collapsed",
    )

    branch = tree.find(session, role="TreeItem", label="Documents")
    if branch is None:
        raise RuntimeError(f"no 'Documents' row; rows: {tree.labels(session, role='TreeItem')}")
    report.check(
        branch.get("expanded") is False,
        "'Documents' advertises itself as collapsed (expanded=False)",
    )

    session.call("expand", node=branch["id"])
    session.settle()
    time.sleep(0.5)
    session.settle()

    report.check(
        tree.find(session, role="TreeItem", label="Projects") is not None,
        "expanding 'Documents' realised its children",
    )
    reopened = tree.find(session, role="TreeItem", label="Documents")
    report.check(
        reopened is not None and reopened["id"] != branch["id"],
        f"…and rebuilt the branch itself: id {branch['id']} → "
        f"{reopened and reopened['id']}. Even the node you acted on is stale afterwards",
    )


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def main(argv: list[str]) -> int:
    binary = None
    keep = False
    rest = list(argv)
    if "--keep" in rest:
        rest.remove("--keep")
        keep = True
    if "--binary" in rest:
        i = rest.index("--binary")
        if i + 1 >= len(rest):
            print("--binary needs a path", file=sys.stderr)
            return 1
        binary = rest[i + 1]
        del rest[i : i + 2]
    if rest:
        print(f"unexpected arguments: {' '.join(rest)}\n\n{__doc__}", file=sys.stderr)
        return 1

    report = Report("data-collections — virtualized ListView / TreeView")
    app = session = None
    try:
        # `argv=[binary]` skips binary resolution entirely; without it,
        # `launch_and_attach` reads cargo's own metadata to find the debug
        # build. Debug, not release: the automation bridge is
        # `#[cfg(debug_assertions)]`, so a release build has nothing to attach
        # to and the wait would time out on a bridge that cannot exist.
        app, session = launch_and_attach(
            argv=[binary] if binary else None, app=APP, label=APP,
        )

        # One snapshot straight after connecting is not enough: the bridge
        # answers as soon as the event loop is up, which is before the window
        # has laid anything out. Wait for a marker instead of sleeping.
        if not tree.wait_for(session, "Repeater", timeout=30):
            raise RuntimeError(
                "the app never rendered its tab bar. Labels seen: "
                + ", ".join(tree.labels(session)[:20])
            )
        session.settle()

        open_tab(session, report, "ListView")
        item_one = virtualization_leg(session, report)
        scroll_leg(session, report)
        if item_one is not None:
            fresh_id_leg(session, report, item_one)
        keyboard_leg(session, report)
        tree_leg(session, report)

    except Exception as exc:  # noqa: BLE001 — any failure here means "could not run"
        # `error`, not `check`: nothing was learned about the app's behaviour,
        # and exit 1 says so. Collapsing this into a failed check would make a
        # missing binary look exactly like a regression.
        report.error(f"{type(exc).__name__}: {exc}")
    finally:
        if session is not None:
            session.close()
        if app is not None:
            if keep:
                report.note(f"--keep: app left running as pid {app.proc.pid}; log {app.log}")
            else:
                app.terminate()

    return report.finish()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
