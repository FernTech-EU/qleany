#!/usr/bin/env python3
"""Probe: the Generate screen.

Covers US-GEN-01 (the pipeline runs on entering, with progress), US-GEN-02
(groups), US-GEN-03 (the path display and the status stripe), US-GEN-04 (the four
filters), US-GEN-05 (wholesale selection), US-GEN-06 (the count on the button),
US-GEN-07 (the two previews), US-GEN-09 (writing under temp) and US-GEN-10
(generation reports what it wrote).

This probe is the slow one. Rendering every file Qleany's own manifest implies is
a few hundred templates, and the point of the screen is that it happens for real,
so there is nothing here to make faster without testing something else.
"""

import os
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = (
    "US-GEN-01", "US-GEN-02", "US-GEN-03", "US-GEN-04",
    "US-GEN-05", "US-GEN-06", "US-GEN-07", "US-GEN-09", "US-GEN-10",
)
PROBE = "generate"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle": {"settle_timeout_ms": 5000}}

#: How long the rendering pass may take before the probe gives up on it.
PIPELINE_TIMEOUT = 600


def texts(session):
    out = []
    for n in session.nodes():
        for key in ("label", "value", "description"):
            if n.get(key):
                out.append(n[key])
    return out


def buttons(session):
    return {
        n.get("label"): n
        for n in session.nodes()
        if n.get("role") == "Button" and n.get("label")
    }


def generate_button(session):
    for label, node in buttons(session).items():
        if label.startswith("Generate (") and label.endswith(")"):
            return node, int(label[len("Generate ("):-1])
    return None, 0


def rows_in(session, low, high):
    listed = [
        n for n in session.nodes()
        if n.get("role") == "ListBoxOption"
        and low <= n.get("bounds", {}).get("x", -1) < high
    ]
    return [
        n for n in sorted(listed, key=lambda n: n.get("bounds", {}).get("y", 0))
    ]


def groups(session):
    return [n.get("label") for n in rows_in(session, 0, 353)]


def files(session):
    return rows_in(session, 353, 780)


def checkbox(session, label):
    for n in session.nodes():
        if n.get("role") == "CheckBox" and n.get("label") == label:
            return n
    return None


def wait_for_pipeline(session, checks):
    """Wait until nothing is running, and confirm progress was shown while it was."""
    saw_progress = False
    deadline = time.time() + PIPELINE_TIMEOUT
    while time.time() < deadline:
        session.call("settle", SETTLE)
        shown = texts(session)
        if any("Computing file status" in t for t in shown):
            saw_progress = True
            continue
        if "Cancel" in [n.get("label") for n in session.nodes() if n.get("role") == "Button"]:
            saw_progress = True
            continue
        break
    checks.check(
        saw_progress,
        "US-GEN-01 the pipeline runs on entering, and says so while it does",
    )
    return time.time() < deadline


def main():
    checks = fixture.Checks()
    log = os.path.join(fixture.SCRATCH, f"qleany-{PROBE}-{os.getpid()}.log")

    manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), PROBE)
    workdir = os.path.dirname(manifest)
    env = fixture.isolated_config(PROBE)
    env["QLEANY_DEV"] = "1"

    app = subprocess.Popen(
        [fixture.app_binary()],
        cwd=workdir,
        stdout=open(log, "w"),
        stderr=subprocess.STDOUT,
        env=env,
    )
    session = None
    try:
        bridge = fixture.wait_for_bridge(log, app, timeout=60)
        session = fixture.Session(bridge)

        session.click(session.find("Open Qleany manifest", timeout=20))
        session.call("settle", SETTLE)
        session.click(session.find("Generate", timeout=10))
        checks.check(
            wait_for_pipeline(session, checks),
            "US-GEN-01 the pipeline finishes",
        )

        # US-GEN-09 and US-GEN-07: the two settings every launch starts from.
        checks.check(
            checkbox(session, "In temp/").get("toggled") == "true",
            "US-GEN-09 writing under temp is the default",
        )
        checks.check(
            checkbox(session, "View diff").get("toggled") == "true",
            "US-GEN-07 the difference is the default preview",
        )

        # US-GEN-04: the filter defaults.
        for label, expected in (
            ("Modified", "true"),
            ("New", "true"),
            ("Unchanged", "false"),
            ("Infra", "true"),
            ("Aggregate", "true"),
            ("Scaffold", "true"),
        ):
            node = checkbox(session, label)
            checks.check(
                node is not None and node.get("toggled") == expected,
                f"US-GEN-04 {label!r} starts {expected}",
            )

        # US-GEN-02: All first, then the groups alphabetically.
        group_names = groups(session)
        checks.check(
            group_names[:1] == ["All"],
            f"US-GEN-02 the group list starts with All, got {group_names[:3]}",
        )
        checks.check(
            group_names[1:] == sorted(group_names[1:]),
            f"US-GEN-02 and the rest are alphabetical, got {group_names[1:5]}",
        )
        checks.check(len(group_names) > 2, "US-GEN-02 there is more than one group")

        session.shot(os.path.join(SHOT_DIR, "qleany-generate.png"))

        # US-GEN-03: a row's accessible name is its whole path, however it is drawn.
        listed = files(session)
        checks.check(bool(listed), "US-GEN-01 the file list is populated")
        checks.check(
            any("/" in (n.get("label") or "") for n in listed),
            "US-GEN-03 a row names the file's path in full",
        )

        # US-GEN-02: picking a group narrows the list.
        before = len(files(session))
        target = group_names[1]
        session.click(next(n for n in rows_in(session, 0, 353) if n.get("label") == target))
        session.call("settle", SETTLE)
        narrowed = len(files(session))
        checks.check(
            narrowed <= before,
            f"US-GEN-02 a group narrows the list, {before} to {narrowed}",
        )
        session.click(next(n for n in rows_in(session, 0, 353) if n.get("label") == "All"))
        session.call("settle", SETTLE)

        # US-GEN-04: the text filter.
        search = next(
            n for n in session.nodes()
            if n.get("role") == "TextInput" and (n.get("bounds", {}).get("x") or 0) > 353
        )
        session.call("set_value", {"node": search["id"], "value": "zzzz-no-such-file"})
        session.call("settle", SETTLE)
        checks.check(
            not files(session),
            "US-GEN-04 a filter that matches nothing shows nothing",
        )
        checks.check(
            "No files match" in texts(session),
            "US-GEN-04 and says so rather than showing an empty pane",
        )
        search = next(
            n for n in session.nodes()
            if n.get("role") == "TextInput" and (n.get("bounds", {}).get("x") or 0) > 353
        )
        session.call("set_value", {"node": search["id"], "value": "Cargo"})
        session.call("settle", SETTLE)
        matched = files(session)
        checks.check(
            matched and all("Cargo" in (n.get("label") or "") for n in matched),
            "US-GEN-04 the text filter matches the path",
        )

        # US-GEN-05 and US-GEN-06: wholesale selection, and the count on the button.
        node, count = generate_button(session)
        checks.check(count == 0, f"US-GEN-06 nothing is ticked yet, got {count}")
        checks.check(
            node is not None and node.get("disabled", False),
            "US-GEN-06 the button is disabled with nothing to write",
        )

        session.click(buttons(session)["Select all"])
        session.call("settle", SETTLE)
        node, count = generate_button(session)
        checks.check(
            count == len(matched),
            f"US-GEN-05 select all ticks every visible row, {count} of {len(matched)}",
        )
        checks.check(
            node is not None and not node.get("disabled", False),
            "US-GEN-06 and the button is live",
        )

        # US-GEN-07: the preview, both ways. The list is re-read first: ticking
        # every row rebuilt it, and a node id is only valid for the instance it
        # named.
        session.click(files(session)[0])
        session.call("settle", SETTLE)
        shown = texts(session)
        checks.check(
            "No file selected" not in shown,
            "US-GEN-07 picking a file shows a preview",
        )
        session.click(checkbox(session, "View diff"))
        session.call("settle", SETTLE)
        checks.check(
            checkbox(session, "View diff").get("toggled") == "false",
            "US-GEN-07 the preview can be switched to the source",
        )

        # US-GEN-10: generate, for real, into temp.
        session.click(generate_button(session)[0])
        deadline = time.time() + PIPELINE_TIMEOUT
        while time.time() < deadline:
            session.call("settle", SETTLE)
            if not any(
                "Generating files" in t for t in texts(session)
            ):
                break
        wrote = [t for t in texts(session) if t.startswith("Generated ")]
        checks.check(
            bool(wrote),
            f"US-GEN-10 the run reports what it wrote, saw {wrote}",
        )

        # US-GEN-09: under temp, and nowhere else.
        temp = os.path.join(workdir, "temp")
        checks.check(
            os.path.isdir(temp),
            f"US-GEN-09 the files were written under {temp}",
        )
        checks.check(
            not os.path.isdir(os.path.join(workdir, "crates")),
            "US-GEN-09 and not into the project root",
        )
        session.shot(os.path.join(SHOT_DIR, "qleany-generate-done.png"))
    finally:
        if session:
            session.close()
        app.terminate()
        try:
            app.wait(timeout=5)
        except subprocess.TimeoutExpired:
            app.kill()

    return checks.finish(PROBE, log)


if __name__ == "__main__":
    sys.exit(main())
