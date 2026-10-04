#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Text editing: typing, IME composition, undo — and how to read a document back.

Drives a live `cargo run -p rich-text-editor` and proves five things:

1. **`type_text` inserts at the caret** of a `RichTextEditor`, and the change is
   readable from the accessibility tree.
2. **The document is shared.** The demo binds one `TextDocument` to an editable
   pane and a read-only preview; the typed text appears in *both*, and the
   second one is outside the editor's own subtree, which is how a probe can
   tell them apart.
3. **Undo reverts it**, through the platform accelerator — `⌘Z` on macOS,
   `Ctrl+Z` elsewhere, one `command=True`.
4. **An IME preedit is visible in the document while it is still uncommitted**,
   so composition can be observed rather than only its result.
5. **Committing replaces the preedit** with the committed text.

Why this needs a live app: the editor's text is not a `value` on one node. It
is shaped by the text engine into `Role::TextRun` children — one per visual
line, re-segmented on every edit — and there is no shaping without a live text
backend and a real width to wrap against. Reading a document back is therefore
a walk, which is most of what this file demonstrates.

    python3 example_rich_text.py [--binary PATH] [--keep]

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

from teksilo_probe import Report, launch_and_attach, tree  # noqa: E402

APP = "rich-text-editor"

#: A marker no document in this demo contains. A probe that asserts on ordinary
#: words ("Hello") can be fooled by prose that already had them; a distinctive
#: one makes "did my edit land" a question with one answer.
MARK = "PROBE-42 "

#: Composition text, and what committing it yields. Japanese on purpose: the
#: preedit and the commit are different *strings*, so "the preedit was replaced"
#: is observable rather than inferred. A Latin-script preedit that commits to
#: itself would make the two states indistinguishable.
PREEDIT = "にほん"
COMMIT = "日本"

#: How long to let an edit propagate. The editor batches a typing burst through
#: `pending_chars` (one `text_changed` pulse per 150 ms), then the preview pane
#: re-renders on the following frame tick, then the AT text runs are rebuilt.
EDIT_PAUSE = 0.8


def editor(session) -> dict:
    """The editable pane, re-found on every call.

    Re-found and not cached, on the rule that holds for any widget that
    rebuilds. This particular node happens to survive an edit, but a probe
    written to depend on that stops working the first time an editor is rebuilt
    for an unrelated reason, and the symptom is a `NOT_FOUND` naming nothing.

    `Role::MultilineTextInput` is the editable preset. The read-only preview in
    the right pane does **not** publish one — it is a `Role::Document` — so
    matching on the role finds the editor and only the editor.
    """
    found = tree.find(session, role="MultilineTextInput")
    if found is None:
        raise RuntimeError(
            "no editable RichTextEditor on screen; roles present: "
            + ", ".join(sorted({str(n.get("role")) for n in tree.nodes(session)}))
        )
    return found


def text_runs_under(session, root_id: int) -> list[str]:
    """Every `Role::TextRun` value in a subtree, in reading order.

    This is what "the document's text" means to an assistive technology, and so
    to a probe: one run per visual line, each carrying the characters actually
    laid out. There is no single node holding the whole document, which is why
    asserting on `node["value"]` of the editor finds nothing and looks like the
    editor is empty.
    """
    nodes = tree.nodes(session)
    by_id = {n["id"]: n for n in nodes}
    out: list[str] = []

    def walk(node_id: int) -> None:
        node = by_id.get(node_id)
        if node is None:
            return
        if node.get("role") == "TextRun" and node.get("value"):
            out.append(str(node["value"]))
        for child in node.get("children") or []:
            walk(child)

    walk(root_id)
    return out


def subtree_ids(session, root_id: int) -> set[int]:
    """Every node id in a subtree, so "outside it" is answerable."""
    nodes = tree.nodes(session)
    by_id = {n["id"]: n for n in nodes}
    seen: set[int] = set()
    stack = [root_id]
    while stack:
        current = stack.pop()
        if current in seen or current not in by_id:
            continue
        seen.add(current)
        stack.extend(by_id[current].get("children") or [])
    return seen


def runs_containing(session, needle: str) -> list[str]:
    """Every text run anywhere in the window that contains `needle`."""
    return [
        str(n["value"])
        for n in tree.nodes(session)
        if n.get("role") == "TextRun" and needle in str(n.get("value") or "")
    ]


def opening_line(session) -> str:
    """The editor's first laid-out line — the cheapest before/after witness."""
    runs = text_runs_under(session, editor(session)["id"])
    return runs[0] if runs else ""


def settle_edit(session) -> None:
    """Let an edit reach the AT tree.

    Both halves matter: `settle` runs the app's animations and layout and
    re-syncs the accessibility tree, and the sleep covers the editor's own
    150 ms typing-burst batch, which `settle` has no way to know about.
    """
    session.settle()
    time.sleep(EDIT_PAUSE)
    session.settle()


# ---------------------------------------------------------------------------
# The legs
# ---------------------------------------------------------------------------


def typing_leg(session, report: Report) -> str:
    """`type_text` inserts at the caret, in both panes. Returns the line before."""
    before = opening_line(session)
    report.note(f"document opens with {before!r}")
    report.check(
        MARK.strip() not in before,
        f"the document does not already contain {MARK.strip()!r} — so finding it "
        "later can only mean this probe put it there",
    )

    # `type_text` focuses the node itself, which matters here: this demo starts
    # with focus in the highlighter's search field, so a bare `inject_key` would
    # type into the search box and report the editor as ignoring input.
    session.call("type_text", node=editor(session)["id"], text=MARK)
    settle_edit(session)

    after = opening_line(session)
    report.check(
        after.startswith(MARK),
        f"typed text landed at the caret: {before!r} → {after!r}",
    )

    # The preview pane. Asserting "the marker appears somewhere" would be
    # satisfied by the editor alone, so the test is specifically that it appears
    # OUTSIDE the editor's subtree — which is the only way to show that the two
    # views share one `TextDocument` rather than that one view echoed itself.
    inside = subtree_ids(session, editor(session)["id"])
    elsewhere = [
        str(n["value"])
        for n in tree.nodes(session)
        if n.get("role") == "TextRun"
        and MARK.strip() in str(n.get("value") or "")
        and n["id"] not in inside
    ]
    report.check(
        bool(elsewhere),
        f"…and in the read-only preview, outside the editor's subtree: "
        f"{elsewhere[:1]}. One TextDocument, two views",
    )
    return before


def undo_leg(session, report: Report, before: str) -> None:
    """Undo, through the platform accelerator rather than a literal Control."""
    # `command=True`, never `ctrl=True`. A shortcut declared `Ctrl+Z` resolves
    # to ⌘Z on macOS, so a literal Control there injects a chord that matches no
    # binding — and `inject_key` still reports success, which makes the failure
    # look like undo being broken rather than the probe addressing the wrong key.
    session.call("inject_key", key="Z", command=True)
    settle_edit(session)

    report.check(
        opening_line(session) == before,
        f"undo restored the opening line exactly: {opening_line(session)!r}",
    )
    report.check(
        not runs_containing(session, MARK.strip()),
        "…and the marker is gone from every pane, preview included",
    )


def ime_leg(session, report: Report) -> None:
    """A preedit is in the document before it is committed; committing replaces it."""
    session.call("type_ime", node=editor(session)["id"], preedit=PREEDIT)
    settle_edit(session)

    composing = runs_containing(session, PREEDIT)
    report.check(
        bool(composing),
        f"the uncommitted preedit {PREEDIT!r} is laid out in the document "
        f"({len(composing)} run(s)) — composition is observable, not just its result",
    )

    session.call("type_ime", node=editor(session)["id"], commit=COMMIT)
    settle_edit(session)

    report.check(
        bool(runs_containing(session, COMMIT)),
        f"committing replaced it with {COMMIT!r}",
    )
    report.check(
        not runs_containing(session, PREEDIT),
        f"…and left no trace of the preedit — {PREEDIT!r} is gone from the document",
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

    report = Report("rich-text-editor — typing, IME and undo")
    app = session = None
    try:
        # Debug build, not release: the automation bridge is
        # `#[cfg(debug_assertions)]`, so a release binary publishes no endpoint
        # and the wait below would time out on one that cannot exist.
        app, session = launch_and_attach(
            argv=[binary] if binary else None, app=APP, label=APP,
        )

        # A generous timeout, and a marker rather than a sleep: this demo parses
        # a large markdown sample and reformats every block before its first
        # frame, so "the bridge answered" is well short of "the document is up".
        if not tree.wait_for(session, "RichTextEditor", timeout=45):
            raise RuntimeError(
                "the document never rendered. Labels seen: "
                + ", ".join(tree.labels(session)[:20])
            )
        session.settle()

        before = typing_leg(session, report)
        undo_leg(session, report, before)
        ime_leg(session, report)

    except Exception as exc:  # noqa: BLE001 — any failure here means "could not run"
        # `error`, not `check`: exit 1 says nothing was learned about the app.
        # A false `check` (exit 2) is a claim that the behaviour is absent, and
        # a suite where the two look alike teaches its readers to trust neither.
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
