#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""A live picture: frames that flow, a guest whose input lands on its pixels,
and a window that wakes for nothing while the picture holds still.

Drives a live `cargo run -p live-image-demo`: a 720 x 1280 guest screen that a
producer thread redraws at 60 Hz, shown by a `LiveImage`. The demo maps every
event the picture receives to the guest pixel it lands on and writes it into
the picture's accessible description, so the probe reads back what the guest
got. Seven things, each of which a VM screen, a video or a camera preview
needs, and none of which the accessibility tree alone can show:

1. **Frames flow.** Two screenshots half a second apart hold later commits,
   neither drawn from an older picture, and their pixels differ. The record
   comes from the screenshot itself (`live_images` in its metadata), not from
   `live_image_stats`, because the producer keeps committing between the two
   calls.
2. **They cost no repaint.** Over half a second, the window's generation
   follows the source's while the widget's paint count stays where it was.
3. **A press lands on its pixel.** `inject_pointer {node, source}` at the four
   corners and the centre reaches the guest at exactly that pixel, in the
   portrait frame and in the landscape one, at whatever scale the window has
   (`--scale-factor` picks one on X11).
4. **Two fingers are two contacts**, and `cancel_pointer` releases each.
5. **Every key reaches the guest** once the picture has focus (it is a
   keyboard capture surface, as a terminal is), Ctrl+C included, and
   Ctrl+Tab still leaves it.
6. **A rotation relayouts the bezel.** The producer resizes its frame, and
   the picture's box takes the new shape.
7. **A paused producer costs nothing.** With `TEKSILO_IDLE_TRACE=1`, the
   window prints no trace line for ten seconds while nothing commits.
8. **Neither does a producer that hands over whole, identical frames.** With
   the demo's *Whole frames* on, the producer gives a `LiveImageDiffWriter`
   its whole framebuffer at every frame; each commit carries only what moved,
   and once the guest holds still, sixty identical frames a second commit
   nothing and the window stays silent.

The probe sends every pointer event the guest sees. A real cursor over the
window would add its own moves; the demo only reports a move while a contact
is down, so a hovering cursor changes nothing here, but keep it off the window
anyway.

    python3 example_live_image.py [--binary PATH] [--keep] [--scale-factor F]

Exit codes: 0 every check passed, 1 the probe could not run, 2 it ran cleanly
and the behaviour is absent. See `teksilo_probe.report`.
"""

from __future__ import annotations

import os
import re
import sys
import time
from pathlib import Path


def _bootstrap() -> None:
    """Put the `teksilo_probe` package on `sys.path`, from either layout: see
    `example_dialogs.py`."""
    for ancestor in Path(__file__).resolve().parents:
        candidate = ancestor.parent if ancestor.name == "teksilo_probe" else ancestor
        if (candidate / "teksilo_probe" / "__init__.py").is_file():
            sys.path.insert(0, str(candidate))
            return


_bootstrap()

from teksilo_probe import Report, launch_and_attach, live_image, tree  # noqa: E402

APP = "live-image-demo"
PORTRAIT = (720, 1280)
LANDSCAPE = (1280, 720)

#: What the demo writes for a pointer event, in source pixels.
POINTER = re.compile(r"^(down|move|up) (\w+) (\d+) at (\d+),(\d+) · (\d+) down$")
CANCEL = re.compile(r"^cancel (\w+) (\d+) · (\d+) down$")


def guest(session) -> dict:
    """The guest screen's node, or a failure that says what was there."""
    found = tree.find(session, role="Image", label="Guest screen")
    if found is None:
        raise RuntimeError("no guest screen; labels seen: " + ", ".join(tree.labels(session)[:20]))
    return found


def said(session, node: dict) -> str:
    """What the guest received last: the picture's description."""
    return session.call("read_node", node=node["id"]).get("description") or ""


def stats(session, node: dict) -> dict:
    return session.tools.live_image_stats(node=node["id"])


def corners(size: tuple[int, int]) -> list[tuple[int, int]]:
    w, h = size
    return [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1), (w // 2, h // 2)]


def wait_for_size(session, node: dict, size: tuple[int, int], timeout: float = 5.0) -> bool:
    """Poll until the picture's source has `size`: a resize reaches layout at the
    next frame."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        session.settle()
        if tuple(session.tools.live_image_map(node=node["id"])["source_size"]) == size:
            return True
        time.sleep(0.05)
    return False


def click(session, label: str, role: str = "Button") -> None:
    found = tree.find(session, role=role, label=label)
    if found is None:
        raise RuntimeError(f"no {role} labelled {label!r}")
    session.call("invoke_action", node=found["id"], action="click")


def flow_leg(session, report: Report) -> None:
    """1 and 2: frames flow, and cost no repaint."""
    node = guest(session)
    first = live_image.capture(session, node)
    time.sleep(0.5)
    second = live_image.capture(session, node)
    report.check(first is not None and second is not None,
                 "both screenshots drew the guest screen")
    if first is None or second is None:
        return
    report.check(second.generation > first.generation,
                 f"the later screenshot holds a later commit ({first.generation} then "
                 f"{second.generation})")
    report.check(not first.deferred and not second.deferred,
                 "neither screenshot drew an older picture")
    report.check(first.rgba != second.rgba, "their pixels differ")

    before = stats(session, node)
    time.sleep(0.5)
    after = stats(session, node)
    committed = after["source"]["generation"] - before["source"]["generation"]
    shown = after["attachment"]["window_generation"] - before["attachment"]["window_generation"]
    report.check(committed >= 10, f"the producer committed {committed} frames in half a second")
    if (after.get("wakes") or {}).get("window_hidden"):
        # A hidden window draws nothing, by design: an X server with no
        # window manager reports every window obscured. The screenshots
        # above still showed the latest commits.
        report.note(f"the window is hidden here, so it drew none of the {committed} commits")
    else:
        report.check(shown >= committed // 2,
                     f"the window followed: {shown} of {committed} new generations shown")
    report.check(after["attachment"]["paints"] == before["attachment"]["paints"],
                 f"and the widget was not repainted ({before['attachment']['paints']} paints, "
                 f"then {after['attachment']['paints']})")


def press_leg(session, report: Report, size: tuple[int, int], label: str) -> None:
    """3: a press at each corner and the centre lands on its pixel."""
    node = guest(session)
    for x, y in corners(size):
        live_image.click_source(session, node, x, y, action="down")
        down = POINTER.match(said(session, node))
        live_image.click_source(session, node, x, y, action="up")
        up = POINTER.match(said(session, node))
        report.check(
            down is not None and down.group(1) == "down"
            and (int(down.group(4)), int(down.group(5))) == (x, y)
            and down.group(6) == "1",
            f"{label}: a press at pixel ({x}, {y}) reaches the guest there "
            f"({down.group(0) if down else said(session, node)!r})",
        )
        report.check(
            up is not None and up.group(1) == "up"
            and (int(up.group(4)), int(up.group(5))) == (x, y)
            and up.group(6) == "0",
            f"{label}: and its release, with nothing left down",
        )


def touch_leg(session, report: Report) -> None:
    """4: two fingers, two contacts, each released by a cancel."""
    node = guest(session)
    reply = session.call("inject_touch_sequence", steps=[
        {"contact": 0, "phase": "down", "node": node["id"], "source": [100, 200]},
        {"contact": 1, "phase": "down", "node": node["id"], "source": [600, 1100],
         "advance_ms": 16},
    ])
    ids = [step["pointer_id"] for step in reply["steps"]]
    second = POINTER.match(said(session, node))
    report.check(len(set(ids)) == 2, f"the two fingers have two identities: {ids}")
    report.check(
        second is not None and second.group(2) == "touch" and second.group(3) == str(ids[1])
        and (second.group(4), second.group(5)) == ("600", "1100") and second.group(6) == "2",
        f"the second finger lands on its pixel with two down ({said(session, node)!r})",
    )
    for left, pointer in zip((1, 0), ids):
        session.call("cancel_pointer", pointer_id=pointer)
        cancelled = CANCEL.match(said(session, node))
        report.check(
            cancelled is not None and cancelled.group(2) == str(pointer)
            and cancelled.group(3) == str(left),
            f"cancelling finger {pointer} releases it, {left} left down "
            f"({said(session, node)!r})",
        )


def key_leg(session, report: Report) -> None:
    """5: keys reach the guest, and Ctrl+Tab leaves."""
    node = guest(session)
    session.call("focus_node", node=node["id"])
    for key, ctrl, expected in (("a", False, "key A"), ("Enter", False, "key Enter"),
                                ("c", True, "key Ctrl+C")):
        session.tools.inject_key(key=key, ctrl=ctrl)
        report.check(said(session, node) == expected,
                     f"{expected[4:]} reaches the guest ({said(session, node)!r})")
    session.tools.inject_key(key="Tab", ctrl=True)
    focused = session.call("read_node", node=node["id"]).get("focused")
    report.check(not focused, "Ctrl+Tab leaves the picture: it is not a keyboard trap")


def rotate_leg(session, report: Report) -> None:
    """6, then 3 again in landscape: a rotation relayouts the bezel."""
    node = guest(session)
    click(session, "Rotate")
    report.check(wait_for_size(session, node, LANDSCAPE), "the guest turned to 1280 x 720")
    box = guest(session)["bounds"]
    report.check(box["width"] > box["height"],
                 f"and the picture's box is landscape ({box['width']:.0f} x {box['height']:.0f})")
    press_leg(session, report, LANDSCAPE, "landscape")
    click(session, "Rotate")
    report.check(wait_for_size(session, node, PORTRAIT), "and turned back to 720 x 1280")
    box = guest(session)["bounds"]
    report.check(box["height"] > box["width"],
                 f"the box portrait again ({box['width']:.0f} x {box['height']:.0f})")


def trace_lines(log: str, start: int, end: int | None = None) -> list[str]:
    """The idle-trace lines the app printed between two offsets of its log."""
    with open(log, encoding="utf-8", errors="replace") as handle:
        handle.seek(start)
        text = handle.read() if end is None else handle.read(end - start)
    return [line for line in text.splitlines() if line.startswith("teksilo_idle_trace")]


def whole_frames_leg(session, report: Report, log: str) -> None:
    """8: whole frames through the diff writer cost only what moved."""
    node = guest(session)
    frame_bytes = PORTRAIT[0] * PORTRAIT[1] * 4
    click(session, "Whole frames", role="CheckBox")
    time.sleep(0.5)
    before = stats(session, node)["source"]
    time.sleep(0.5)
    after = stats(session, node)["source"]
    commits = after["commits"] - before["commits"]
    written = after["bytes_written"] - before["bytes_written"]
    report.check(commits >= 10, f"whole frames still flow: {commits} commits in half a second")
    report.check(commits > 0 and written // commits < frame_bytes // 10,
                 f"and each carries only what moved: {written // max(commits, 1)} bytes a "
                 f"commit, against {frame_bytes} for the frame")

    click(session, "Still guest", role="CheckBox")
    time.sleep(1.0)
    held = stats(session, node)["source"]["generation"]
    start = os.path.getsize(log)
    # Five seconds without a call, while the producer hands over sixty
    # identical frames a second.
    time.sleep(5.0)
    lines = trace_lines(log, start)
    report.check(stats(session, node)["source"]["generation"] == held,
                 "identical whole frames commit nothing")
    report.check(not lines, f"and the window stays silent ({len(lines)} lines: {lines[:2]})")

    click(session, "Still guest", role="CheckBox")
    time.sleep(0.5)
    report.check(stats(session, node)["source"]["generation"] > held,
                 "and frames flow again once the guest moves")
    click(session, "Whole frames", role="CheckBox")


def idle_leg(session, report: Report, log: str) -> None:
    """7: a paused producer costs nothing."""
    node = guest(session)
    click(session, "Pause producer", role="CheckBox")
    # Let the click's own frames, and a commit in flight, go by.
    time.sleep(2.0)
    held = stats(session, node)["source"]["generation"]
    time.sleep(0.5)
    report.check(stats(session, node)["source"]["generation"] == held,
                 "the paused producer commits nothing")
    time.sleep(2.0)
    start = os.path.getsize(log)
    flowing = trace_lines(log, 0, start)
    # Without it, the silences checked here and in the whole-frames leg
    # would prove nothing.
    report.check(bool(flowing), f"the trace is on: {len(flowing)} lines while frames flowed")
    # Ten seconds without a single call: any trace line now is the window
    # waking on its own.
    time.sleep(10.0)
    lines = trace_lines(log, start)
    report.check(not lines, f"no idle-trace line in ten seconds ({len(lines)}: {lines[:2]})")
    click(session, "Pause producer", role="CheckBox")
    time.sleep(0.5)
    report.check(stats(session, node)["source"]["generation"] > held,
                 "and it resumes")


def main(argv: list[str]) -> int:
    binary = None
    keep = False
    scale = None
    rest = list(argv)
    if "--keep" in rest:
        rest.remove("--keep")
        keep = True
    for flag in ("--binary", "--scale-factor"):
        if flag in rest:
            i = rest.index(flag)
            if i + 1 >= len(rest):
                print(f"{flag} needs a value", file=sys.stderr)
                return 1
            if flag == "--binary":
                binary = rest[i + 1]
            else:
                scale = rest[i + 1]
            del rest[i : i + 2]
    if rest:
        print(f"unexpected arguments: {' '.join(rest)}\n\n{__doc__}", file=sys.stderr)
        return 1

    report = Report("live-image-demo — frames, input and idle")
    app = session = None
    env = dict(os.environ, TEKSILO_IDLE_TRACE="1")
    if scale is not None:
        env["WINIT_X11_SCALE_FACTOR"] = scale
    try:
        app, session = launch_and_attach(
            argv=[binary] if binary else None, app=APP, label=APP, env=env,
        )
        if not tree.wait_for(session, "Guest screen", timeout=30):
            raise RuntimeError("the guest screen never appeared. Labels seen: "
                               + ", ".join(tree.labels(session)[:20]))
        session.settle()
        if scale is not None:
            shot = session.call_result("screenshot").payload
            report.note(f"window scale {shot.get('scale')} (asked {scale}; only X11 reads "
                        "WINIT_X11_SCALE_FACTOR)")

        flow_leg(session, report)
        press_leg(session, report, PORTRAIT, "portrait")
        touch_leg(session, report)
        key_leg(session, report)
        rotate_leg(session, report)
        whole_frames_leg(session, report, app.log)
        idle_leg(session, report, app.log)

    except Exception as exc:  # noqa: BLE001 — any failure here means "could not run"
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
