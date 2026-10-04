#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Overlay lifecycle: where a dialog actually *is*, and who has focus.

Drives a live `cargo run -p dialogs-and-popovers` and proves five things about
overlays that a probe has to know before it can assert anything about one:

1. **A `Dialog` is not necessarily an overlay.** Teksilo resolves a modal
   through `ModalPresentation::Auto`, and on a backend that supports native
   modal child windows it opens a real OS **window**. `get_overlays` stays
   empty and a default `snapshot_tree` still shows the parent — so a probe that
   looks in either place concludes the dialog never opened. `list_windows` is
   where it is, and every read after that needs `window_id=`.
2. **Focus moves into the modal, onto its declared default button**, via
   `ModalRequest::focus_target` + `Widget::initial_focus_hint`.
3. **Dismissing returns focus to the trigger** that opened it.
4. **Escape dismisses a `MessageBox`** — and the app's own result readout
   records `escape=true`, so the escape button really fired rather than the
   window merely vanishing.
5. **An in-tree `Popover` is the other shape**: `get_overlays` counts it, focus
   moves into its content, Escape dismisses it and focus comes back to the
   trigger (or where it was, if the press gave the trigger none). And the
   two-press rule — an outside press dismisses the overlay *without* reaching
   the control beneath — holds for a **direct** pointer (touch / pen) and not
   for a mouse. See `two_press_leg`.

Why this needs a live app: presentation is resolved at runtime against the
window system, so which of these shapes you get is not visible in the source at
all. Only a running app on this host can say.

    python3 example_dialogs.py [--binary PATH] [--keep]

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

APP = "dialogs-and-popovers"

#: The parent window. Teksilo numbers windows from 1 and the initial one keeps
#: that id for the app's life, so it is the one constant here — every *other*
#: id in this probe is discovered.
MAIN = 1

#: How long to let a modal window open, close, or an overlay fade. `settle`
#: covers the animation inside one tree; creating or destroying an OS window is
#: the event loop's work and lands a frame or two later.
OVERLAY_PAUSE = 0.8


def window_ids(session) -> set[int]:
    """Every managed window's id, right now."""
    return {w["id"] for w in session.call("list_windows")}


def focused_ids(session, window_id: int | None = None) -> list[int]:
    """Whatever holds focus in a window. Empty, one entry, never more."""
    return [n["id"] for n in tree.nodes(session, window_id=window_id) if n.get("focused")]


def parents_of(nodes: list[dict]) -> dict[int, int]:
    """child id → parent id, built from one snapshot.

    A snapshot is a flat list with a `children` id list per node, so "is this
    node inside that one" is a question about ancestry that has to be
    reconstructed. Doing it from the *same* snapshot the other reads came from
    matters: ids churn between snapshots, so mixing two is how a containment
    test starts answering about nodes that no longer exist.
    """
    return {
        child: node["id"]
        for node in nodes
        for child in (node.get("children") or [])
    }


def ancestors_of(nodes: list[dict], node_id: int) -> list[int]:
    """Every ancestor of `node_id`, innermost first."""
    parents = parents_of(nodes)
    chain, current = [], node_id
    while current in parents:
        current = parents[current]
        chain.append(current)
    return chain


def press(session, node: dict, *, kind: str = "mouse") -> None:
    """Press a node's centre with a synthetic pointer of a given kind.

    A pointer and not the AT `click` action, deliberately: the questions this
    probe asks — does an outside press dismiss, does it also reach what is
    underneath — are questions about *pointer* routing, and an AT action
    bypasses all of it. `kind` matters for the same reason; see `two_press_leg`.
    """
    box = node["bounds"]
    session.call(
        "inject_pointer",
        x=box["x"] + box["width"] / 2.0,
        y=box["y"] + box["height"] / 2.0,
        action="click",
        kind=kind,
    )
    session.settle()
    time.sleep(OVERLAY_PAUSE)
    session.settle()


def button(session, label: str, *, window_id: int | None = None) -> dict:
    """A named button, or a failure that says what was on screen instead."""
    found = tree.find(session, role="Button", label=label, window_id=window_id)
    if found is None:
        raise RuntimeError(
            f"no {label!r} button in window {window_id or 'default'}; buttons: "
            + ", ".join(tree.labels(session, role="Button", window_id=window_id))
        )
    return found


def modal_scope(
    session, report: Report, before: set[int], what: str
) -> tuple[int, bool] | None:
    """Where a just-opened modal actually lives, asserted in whichever shape it took.

    `ModalPresentation::Auto` is resolved at runtime against the window
    system: a real OS child window where one can be made input-blocking
    (macOS, Windows), an in-tree overlay where none can — Linux and the BSDs,
    where no client can make another surface block input. Both are correct,
    and which one arrives is a property of the host, not of the source. So
    this asserts the invariant that *distinguishes* the two in whichever
    direction it went, rather than demanding one and calling the other a
    failure.

    Returns `(scope, native)` — the window id every later read in the leg must
    address, and which shape it got. `None` means neither shape appeared,
    which is the only real failure here.
    """
    opened = sorted(window_ids(session) - before)
    overlays = session.call("get_overlays").get("count") or 0

    report.check(
        len(opened) == 1 or overlays > 0,
        f"the {what} opened (native window ids: {opened or 'none'}; "
        f"in-tree overlays: {overlays})",
    )

    if opened:
        report.check(
            len(opened) == 1 and overlays == 0,
            "…as a native modal child WINDOW, and `get_overlays` is empty "
            f"because a window is not an overlay (overlays={overlays}). This "
            "is the trap: a probe that looks in `get_overlays` here sees zero "
            "and reports a modal that is plainly on screen as never opened",
        )
        return opened[0], True

    if overlays:
        # A note, not a check: the branch condition *is* the assertion, and the
        # one above already established that a modal appeared. Re-asserting
        # `overlays > 0` inside `if overlays` would read as evidence while
        # testing nothing. The count is not `== 1` because a modal overlay is a
        # scrim *and* a panel — it counts layers, not modals.
        report.note(
            f"…as an in-tree modal overlay ({overlays} layers — scrim and "
            "panel), because this backend has no input-blocking child window "
            "for `Auto` to resolve to. Every read below addresses the parent "
            "tree instead of a modal window"
        )
        return MAIN, False

    return None


def dismissed(session, report: Report, scope: int, native: bool, how: str) -> None:
    """Assert the modal is gone, in the terms its own shape is counted in."""
    if native:
        report.check(scope not in window_ids(session), f"{how} closed the modal window")
    else:
        report.check(
            (session.call("get_overlays").get("count") or 0) == 0,
            f"{how} closed the modal overlay",
        )


def modal_opened(session, before: set[int]) -> bool:
    """Did a modal appear, in whichever shape this backend gives one?

    "A new window appeared" is not the question — it is the answer on macOS
    and Windows only. Asking it directly on Linux makes *did not activate*
    and *activated an overlay* look identical, which is a check that passes
    for the wrong reason rather than one that fails.
    """
    return bool(window_ids(session) - before) or bool(
        session.call("get_overlays").get("count") or 0
    )


def result_readout(session) -> str | None:
    """The demo's own "Last result:" line, once a MessageBox has answered.

    Matched on the `dismissal=` the app formats into it rather than on
    position: the readout is one of ~20 labels in a `ScrollArea` and its index
    moves whenever anything above it reflows.
    """
    for node in tree.nodes(session, window_id=MAIN):
        value = str(node.get("value") or "")
        if node.get("role") == "Label" and "dismissal=" in value:
            return value
    return None


# ---------------------------------------------------------------------------
# The legs
# ---------------------------------------------------------------------------


def dialog_leg(session, report: Report) -> None:
    """A `Dialog`: where it opens, who gets focus, where focus goes back to."""
    trigger = button(session, "Open dialog")
    before = window_ids(session)
    press(session, trigger)

    resolved = modal_scope(session, report, before, "Dialog")
    if resolved is None:
        return
    modal, native = resolved

    panel = tree.find(session, role="Dialog", window_id=modal)
    report.check(
        panel is not None and panel.get("label") == "Review Changes",
        f"the modal window carries Role::Dialog named 'Review Changes' "
        f"(got {panel and panel.get('label')!r})",
    )

    focused = focused_ids(session, window_id=modal)
    default_button = tree.find(session, role="Button", label="Cancel", window_id=modal)
    report.check(
        bool(focused) and default_button is not None and focused[0] == default_button["id"],
        "focus moved into the dialog, onto its default button ('Cancel') — "
        "`ModalRequest::focus_target`, so a platform button order that puts the "
        "default last still gets it focused",
    )

    # Dismiss through the dialog's own Cancel, which calls `ctx.dismiss_modal()`
    # — the route an app author writes, and the one this leg is about.
    #
    # Escape is the other route and is checked in `message_box_leg` instead, on
    # a surface that has it on every platform. It is deliberately not asserted
    # here: which presentation this Dialog gets is decided at runtime by
    # `supports_native_modal_windows()` — a real OS window on macOS and
    # Windows, an in-tree overlay on Linux and the BSDs, where no client can
    # make another surface input-blocking. Both honour Escape (the native arm
    # gained it when finding 1 was fixed), but they reach it by different
    # machinery, so a probe that asserted it here would be asserting a
    # different thing depending on where it ran.
    if default_button is not None:
        session.call("invoke_action", node=default_button["id"], action="click", window_id=modal)
        session.settle()
        time.sleep(OVERLAY_PAUSE)
        session.settle()

    dismissed(session, report, modal, native, "dismissing through the dialog's own action")

    # Focus lands on the trigger — or on a node inside it. The trigger here is a
    # custom widget wrapped by an `OverlayTrigger`, so the focusable node is the
    # wrapper's, one level below the named Button. Asserting "the trigger or a
    # descendant of it" is what the contract actually promises; demanding the
    # exact id would break on any trigger that is not a bare Button.
    back = focused_ids(session, window_id=MAIN)
    nodes = tree.nodes(session, window_id=MAIN)
    report.check(
        bool(back)
        and (back[0] == trigger["id"] or trigger["id"] in ancestors_of(nodes, back[0])),
        f"…and focus returned to the trigger that opened it (focus={back}, "
        f"trigger={trigger['id']})",
    )


def message_box_leg(session, report: Report) -> None:
    """Escape dismisses a `MessageBox`, and the app can prove which button fired."""
    before = window_ids(session)
    navigate.click(session, button(session, "Save changes?"))
    session.settle()
    time.sleep(OVERLAY_PAUSE)
    session.settle()

    resolved = modal_scope(session, report, before, "MessageBox")
    if resolved is None:
        return
    modal, native = resolved

    focused = focused_ids(session, window_id=modal)
    save = tree.find(session, role="Button", label="Save", window_id=modal)
    report.check(
        bool(focused) and save is not None and focused[0] == save["id"],
        "focus landed on the declared default button ('Save'), not the first "
        "focusable descendant ('Discard')",
    )

    # `window_id=modal`, not the default: keys route to the focused widget of
    # the tree they are addressed to, and the default tree is still the parent
    # window's — where Escape would mean nothing at all.
    session.call("inject_key", key="Escape", window_id=modal)
    session.settle()
    time.sleep(OVERLAY_PAUSE)
    session.settle()

    dismissed(session, report, modal, native, "Escape")
    answer = result_readout(session)
    report.check(
        answer is not None and "dismissal=Escape" in answer,
        f"…through its escape button, which the app recorded: {answer!r}. The "
        "modal vanishing is not on its own evidence that the right path ran — "
        "and the readout names the route, so a click-outside cannot pass as "
        "an Escape",
    )


def popover_leg(session, report: Report) -> None:
    """An in-tree `Popover`: counted as an overlay, Escape-dismissable."""
    trigger = button(session, "Show popover")
    focus_before = focused_ids(session)
    press(session, trigger)

    report.check(
        session.call("get_overlays").get("count") == 1,
        "the popover IS an in-tree overlay — `get_overlays` counts it, and it "
        "opened no window",
    )

    nodes = tree.nodes(session)
    body = tree.find(
        nodes,
        role="Label",
        value="Use popovers for compact contextual actions without leaving the current surface.",
    )
    focused = [n["id"] for n in nodes if n.get("focused")]
    report.check(
        bool(focused) and body is not None and focused[0] in ancestors_of(nodes, body["id"]),
        "focus moved into the popover's content (the focused node is an "
        "ancestor of the popover's body text, in the same snapshot)",
    )

    session.call("inject_key", key="Escape")
    session.settle()
    time.sleep(OVERLAY_PAUSE)
    session.settle()

    report.check(
        session.call("get_overlays").get("count") == 0, "Escape dismissed the popover",
    )
    # Where focus comes back to depends on the press that opened it. A trigger
    # that is a focusable button takes focus from the press, as a button does,
    # so it holds focus when the popover opens and gets it back; one a press
    # gives no focus leaves focus where it was. Anywhere else is a loss.
    back = focused_ids(session)
    report.check(
        back in ([trigger["id"]], focus_before),
        f"…and focus returned to the trigger that opened it, or, where the "
        f"press gave the trigger no focus, to where it was before "
        f"(before {focus_before}, trigger {trigger['id']}, now {back})",
    )


def two_press_leg(session, report: Report) -> None:
    """Dismissing an overlay and activating what is under it are two presses.

    The app's module docs state this rule, and it holds — for a **direct**
    pointer. `pointer_router` arms the dismissal on a direct pointer's press and
    commits it on the release, so the gesture that dismissed leaves no trace
    beneath. An **indirect** pointer (a mouse) dismisses on the press and falls
    through, so one mouse click does both.

    That difference is the reason this leg drives a `touch` pointer explicitly,
    and the reason a probe cannot state the rule without naming a pointer kind:
    the same script with `kind="mouse"` observes the opposite and is equally
    correct. (The app's docs say "on every pointer kind", which overstates it.)
    """
    press(session, button(session, "Show popover"), kind="touch")
    if session.call("get_overlays").get("count") != 1:
        raise RuntimeError("the popover did not reopen for the two-press check")

    # A control outside the popover's bounds, underneath it in z-order, whose
    # activation is unmistakable: it opens a modal — a window or an in-tree
    # overlay, whichever this backend resolves `Auto` to.
    underneath = button(session, "Welcome")
    before = window_ids(session)

    press(session, underneath, kind="touch")
    report.check(
        (session.call("get_overlays").get("count") or 0) == 0,
        "press 1 (touch, outside the popover) dismissed it",
    )
    report.check(
        not modal_opened(session, before),
        "…and did NOT reach the button underneath — the press that dismissed is "
        "spent on the dismissal",
    )

    press(session, underneath, kind="touch")
    report.check(
        modal_opened(session, before),
        "press 2 activated the control underneath (new windows: "
        f"{sorted(window_ids(session) - before) or 'none'}; overlays: "
        f"{session.call('get_overlays').get('count')})",
    )
    opened = sorted(window_ids(session) - before)
    # Leave nothing open behind us: the next reader of this file will copy it.
    # `try_call`, because a window that closed on its own between the check and
    # this line is fine — and a tidy-up that can fail the probe would turn a
    # clean run into an exit 1 for no reason anyone could act on.
    for extra in opened:
        session.try_call("inject_key", key="Escape", window_id=extra)
        session.settle()
    if not opened:
        # The in-tree arm: no window to address, so Escape goes to the tree
        # the overlay is in.
        session.try_call("inject_key", key="Escape", window_id=MAIN)
        session.settle()


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

    report = Report("dialogs-and-popovers — overlay lifecycle and focus")
    app = session = None
    try:
        # Debug build, not release: the automation bridge is
        # `#[cfg(debug_assertions)]`, so a release binary has nothing to attach
        # to. `--binary` skips the cargo-metadata lookup that finds it.
        app, session = launch_and_attach(
            argv=[binary] if binary else None, app=APP, label=APP,
        )

        # The bridge answers as soon as the event loop is up, which is before
        # the window has laid anything out — so wait on a marker rather than
        # taking one snapshot and concluding the app is empty.
        if not tree.wait_for(session, "Dialogs and Popovers", timeout=30):
            raise RuntimeError(
                "the app never rendered its heading. Labels seen: "
                + ", ".join(tree.labels(session)[:20])
            )
        session.settle()

        dialog_leg(session, report)
        message_box_leg(session, report)
        popover_leg(session, report)
        two_press_leg(session, report)

    except Exception as exc:  # noqa: BLE001 — any failure here means "could not run"
        # `error`, not `check`: exit 1 means nothing was learned. A failed
        # `check` (exit 2) is a claim about the app, and the two must not be
        # allowed to look alike.
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
