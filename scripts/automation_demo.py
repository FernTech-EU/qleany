#!/usr/bin/env python3
"""Probe: the demo generator.

Covers US-DEMO-01 (a whole sample project from one dialog), US-DEMO-02 (language
and destination), US-DEMO-03 (progress, and nothing can interrupt it), US-DEMO-04
(the summary), US-DEMO-05 (copy the command, open the folder) and US-DEMO-06 (a
blocked demo says why).

This one generates for real, into a temporary folder, and asserts against what
landed on disk. It is the slowest probe in the set for the same reason it is the
most convincing: nothing here is a stub.
"""

import os
import shutil
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = (
    "US-DEMO-01",
    "US-DEMO-02",
    "US-DEMO-03",
    "US-DEMO-04",
    "US-DEMO-05",
    "US-DEMO-06",
)
PROBE = "demo"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle": {"settle_timeout_ms": 3000}}

# Rendering every file of a Data management manifest is minutes of work on a cold
# debug build, and this probe waits for all of it.
RUN_TIMEOUT = int(os.environ.get("QLEANY_DEMO_TIMEOUT", "900"))

# What the running face shows whatever step it is on. Everything else it says is
# a report of progress, which is what US-DEMO-03 is about.
FIXED_WHILE_RUNNING = {"Run the demo", "Generating the demo project", "Close"}


def dialog(session):
    """Only the modal's own nodes.

    Everything here is scoped, because the application behind the scrim has a
    "Generate" button of its own in the navigation rail and a "Close" button of
    its own in the title bar, and both come first in an unscoped snapshot.
    """
    return fixture.overlay_nodes(session)


def roles(nodes, role):
    return [n for n in nodes if n.get("role") == role]


def button(nodes, label):
    for n in roles(nodes, "Button"):
        if n.get("label") == label:
            return n
    return None


def texts(nodes):
    """Every visible string, from the nodes that carry one either way."""
    out = []
    for n in nodes:
        for key in ("value", "label", "description"):
            v = n.get(key)
            if isinstance(v, str) and v:
                out.append(v)
    return out


def says(nodes, fragment):
    return any(fragment in t for t in texts(nodes))


def dialog_is_open(session):
    return bool(dialog(session))


def unsaved(session):
    """Whether the title bar says there is work that is not on disk.

    Read from the title bar's save button, whose tooltip is one of two sentences,
    rather than from the Home screen's button, which only exists on Home.
    """
    return "No unsaved changes" not in [n.get("label") for n in session.nodes()]


def open_demo(session):
    session.click(button(session.nodes(), "Run demo"))
    session.call("settle", SETTLE)


def set_destination(session, path):
    field = next(
        n for n in roles(dialog(session), "TextInput") if n.get("label") == "Destination"
    )
    session.call("set_value", {"node": field["id"], "value": path})
    session.call("settle", SETTLE)


def main():
    checks = fixture.Checks()
    log = os.path.join(fixture.SCRATCH, f"qleany-{PROBE}-{os.getpid()}.log")

    manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), PROBE)
    env = fixture.isolated_config(PROBE)
    env["QLEANY_DEV"] = "1"

    sandbox = tempfile.mkdtemp(prefix="qleany-demo-probe-")
    occupied = os.path.join(sandbox, "occupied")
    os.makedirs(occupied)
    with open(os.path.join(occupied, "qleany.yaml"), "w") as fh:
        fh.write("# not a real manifest\n")
    target = os.path.join(sandbox, "fresh")

    app = subprocess.Popen(
        [fixture.app_binary()],
        cwd=os.path.dirname(manifest),
        stdout=open(log, "w"),
        stderr=subprocess.STDOUT,
        env=env,
    )
    session = None
    try:
        bridge = fixture.wait_for_bridge(log, app, timeout=60)
        session = fixture.Session(bridge)

        # The demo loads a manifest of its own, so it goes through the same guard
        # quitting does. The Slint UI discarded unsaved work without asking.
        session.click(session.find("Open Qleany manifest", timeout=10))
        session.call("settle", SETTLE)
        session.click(session.find("Project", timeout=10))
        session.call("settle", SETTLE)
        name = session.find("Application name", timeout=10, role="TextInput")
        session.call("set_value", {"node": name["id"], "value": "DirtiedByProbe"})
        # The form commits on Enter or on blur, not per keystroke, so nothing has
        # reached the store until one of those happens.
        session.call("focus_node", {"node": name["id"]})
        session.call("inject_key", {"key": "Enter"})
        session.call("settle", SETTLE)
        checks.check(unsaved(session), "the manifest is unsaved before the demo is asked for")
        session.click(session.find("Home", timeout=10))
        session.call("settle", SETTLE)
        session.click(button(session.nodes(), "Run demo"))
        session.call("settle", SETTLE)
        asked = texts(session.nodes())
        checks.check(
            any("Save before running the demo?" in t for t in asked),
            f"US-DEMO-01 running the demo over unsaved work asks first",
        )
        cancel = session.find("Cancel", timeout=3)
        checks.check(cancel is not None, "and the question can be declined")
        if cancel is not None:
            session.click(cancel)
            session.call("settle", SETTLE)
        checks.check(
            not dialog_is_open(session),
            "US-DEMO-01 declining does not open the demo",
        )
        checks.check(unsaved(session), "and leaves the unsaved work alone")
        session.click(button(session.nodes(), "Close current manifest"))
        session.call("settle", SETTLE)

        open_demo(session)
        checks.check(dialog_is_open(session), "US-DEMO-01 Run demo opens a dialog")

        # US-DEMO-02: both languages, and the default destination.
        offered = [n.get("label") for n in roles(dialog(session), "RadioButton")]
        checks.check(
            offered == ["Rust", "C++ / Qt"],
            f"US-DEMO-02 both languages are offered, got {offered}",
        )
        field = next(
            (n for n in roles(dialog(session), "TextInput") if n.get("label") == "Destination"),
            None,
        )
        checks.check(
            field is not None and field.get("value") == "~/qleany-demo",
            "US-DEMO-02 the destination defaults to ~/qleany-demo",
        )
        checks.check(
            button(dialog(session), "Browse...") is not None,
            "US-DEMO-02 and offers a Browse",
        )
        session.shot(os.path.join(SHOT_DIR, "qleany-demo-form.png"))

        # US-DEMO-06: a folder that already holds a project is refused, by name,
        # before anything is written.
        set_destination(session, occupied)
        session.click(button(dialog(session), "Generate"))
        session.call("settle", SETTLE)
        panel = dialog(session)
        checks.check(
            says(panel, "already exists") and says(panel, occupied),
            "US-DEMO-06 an occupied folder is refused, and named",
        )
        checks.check(
            button(panel, "Generate") is not None,
            "US-DEMO-06 and the dialog stays usable",
        )
        checks.check(
            os.listdir(occupied) == ["qleany.yaml"],
            "US-DEMO-06 and nothing was written into it",
        )
        session.shot(os.path.join(SHOT_DIR, "qleany-demo-blocked.png"))

        # US-DEMO-01: the real run.
        set_destination(session, target)
        session.click(button(dialog(session), "Generate"))
        session.call("settle", {"settle": {"settle_timeout_ms": 500}})

        # US-DEMO-03: while it runs, nothing gets out of it.
        panel = dialog(session)
        close = button(panel, "Close")
        checks.check(
            close is not None and close.get("disabled", False),
            "US-DEMO-03 Close is present and inert while it runs",
        )
        checks.check(
            button(panel, "Cancel") is None and button(panel, "Generate") is None,
            "US-DEMO-03 and the form's buttons are gone",
        )
        session.call("inject_key", {"key": "Escape"})
        session.call("settle", {"settle": {"settle_timeout_ms": 500}})
        panel = dialog(session)
        checks.check(
            bool(panel) and button(panel, "Generate") is None,
            "US-DEMO-03 Escape does not dismiss a running demo",
        )
        session.shot(os.path.join(SHOT_DIR, "qleany-demo-running.png"))

        # US-DEMO-03: the report moves while it runs.
        #
        # Every string the running face shows, not only the ones ending in an
        # ellipsis: the rendering step hands the backend's own progress through,
        # and "Snapshot 40/123" is exactly the kind of report this is checking for.
        seen = set()
        deadline = time.time() + RUN_TIMEOUT
        finished = False
        started = time.time()
        samples = 0
        while time.time() < deadline:
            said = texts(dialog(session))
            seen.update(said)
            samples += 1
            if any("generated successfully" in t for t in said):
                finished = True
                break
            # No pause for the first stretch: a demo of this size finishes in about
            # a second, and a probe that slept half of it would assert nothing about
            # the reporting and would do so intermittently. A round trip is already
            # about a tenth of a second, so this is the fastest honest sampling.
            if samples > 40:
                time.sleep(0.5)
        elapsed = time.time() - started
        checks.check(
            finished,
            f"US-DEMO-01 the demo runs to completion, in {elapsed:.1f}s over {samples} samples",
        )
        # Either a step of the pipeline naming itself, or the backend's own
        # progress passed straight through. Both are the running face reporting.
        reports = sorted(t for t in seen if t.endswith("...") or t.startswith("Snapshot "))
        checks.check(
            bool(reports),
            f"US-DEMO-03 the running face reports what it is doing, saw {reports}",
        )

        if not finished:
            checks.finish(PROBE, log)
            return

        # US-DEMO-04: what you got.
        panel = dialog(session)
        checks.check(
            says(panel, "Demo project generated successfully!"),
            "US-DEMO-04 the summary says it worked",
        )
        stats = next((t for t in texts(panel) if t.startswith("Generated ")), "")
        checks.check(
            "files from a manifest of" in stats and " 0 files" not in stats,
            f"US-DEMO-04 the summary counts files and manifest lines, got {stats!r}",
        )
        for bullet in (
            "CRUD infrastructure",
            "undo and redo",
            "event system",
            "Relationship management",
            "test suite",
        ):
            checks.check(says(panel, bullet), f"US-DEMO-04 the summary lists {bullet}")
        checks.check(
            says(panel, "CLI") and says(panel, "Teksilo"),
            "US-DEMO-04 and names the frontends a Rust demo generates",
        )
        checks.check(
            not says(panel, "and Slint"),
            "US-DEMO-04 and does not claim one it did not generate",
        )
        session.shot(os.path.join(SHOT_DIR, "qleany-demo-summary.png"))

        # US-DEMO-05: the command, and the copy that confirms itself.
        checks.check(
            says(panel, "cargo run --bin demo") and says(panel, target),
            "US-DEMO-05 the next-step command names the project and how to run it",
        )
        copy = button(panel, "Copy the command")
        checks.check(copy is not None, "US-DEMO-05 a copy button is offered")
        if copy is not None:
            session.click(copy)
            session.call("settle", SETTLE)
            panel = dialog(session)
            checks.check(
                button(panel, "Copied") is not None
                and button(panel, "Copy the command") is None,
                "US-DEMO-05 and it confirms the copy",
            )
        checks.check(
            button(panel, "Open folder") is not None,
            "US-DEMO-05 and the folder can be opened",
        )

        # US-DEMO-01: it is on disk, not just on screen.
        checks.check(
            os.path.exists(os.path.join(target, "qleany.yaml")),
            "US-DEMO-01 a manifest was written",
        )
        checks.check(
            os.path.exists(os.path.join(target, "Cargo.toml")),
            "US-DEMO-01 and a Cargo workspace beside it",
        )
        with open(os.path.join(target, "qleany.yaml")) as fh:
            manifest_lines = len(fh.read().splitlines())
        checks.check(
            f"of {manifest_lines} lines" in stats,
            f"US-DEMO-04 the line count is the manifest's own {manifest_lines}, said {stats!r}",
        )
        written = sum(len(files) for _, _, files in os.walk(target))
        checks.check(written > 50, f"US-DEMO-01 and a whole project, {written} files")
        if stats:
            counted = int(stats.split()[1])
            checks.check(
                counted <= written,
                f"US-DEMO-04 the count is of files that exist, said {counted}, found {written}",
            )

        # The demo's manifest is what the application now has open, which is the
        # honest answer: it is what is in the store.
        session.click(button(dialog(session), "Close"))
        session.call("settle", SETTLE)
        checks.check(not dialog_is_open(session), "US-DEMO-05 Close dismisses the summary")
        checks.check(
            says(session.nodes(), os.path.join(target, "qleany.yaml")),
            "the demo's manifest is the one the application says is open",
        )
    finally:
        if session:
            session.close()
        app.terminate()
        try:
            app.wait(timeout=5)
        except subprocess.TimeoutExpired:
            app.kill()
        shutil.rmtree(sandbox, ignore_errors=True)

    checks.finish(PROBE, log)


if __name__ == "__main__":
    main()
