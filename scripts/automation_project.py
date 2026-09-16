#!/usr/bin/env python3
"""Probe: the Project settings screen.

Covers US-PROJ-01 (the language combo offers both targets and maps to the codes
the manifest stores), US-PROJ-02 (each required field carries its own error),
US-PROJ-03 (the prefix-path placeholder names the selected language's default)
and US-PROJ-04 (an edit is committed when the field is finished with, and reaches
the file on disk).

US-PROJ-04 is asserted against the YAML rather than against the form. A value
typed into a field and never committed still reads back from the field, so
re-reading the widget would pass whether or not anything was written. The probe
therefore edits, saves, and reads the file, which is the only assertion that can
tell the two apart. It works on a throwaway copy, so the save is real.
"""

import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = ("US-PROJ-01", "US-PROJ-02", "US-PROJ-03", "US-PROJ-04")
PROBE = "project"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())

#: What Qleany's own manifest says, and therefore what the form must show.
FROM_THE_MANIFEST = {
    "Application name": "Qleany",
    "Organisation name": "FernTech",
    "Organisation domain": "eu.ferntech",
    "Prefix path": "crates",
}


def placeholder_of(session, node):
    """The placeholder, which the accessibility tree deliberately does not carry.

    Placeholder text is a hint rather than content, so it is not published as a
    text run and no screen reader announces it as a value. `inspect_node` returns
    the widget's own `Debug`, which does carry it.
    """
    _, payload = session.call("inspect_node", {"node": node["id"]})
    debug = (payload or {}).get("debug", "")
    marker = 'placeholder: "'
    if marker not in debug:
        return None
    rest = debug.split(marker, 1)[1]
    return rest.split('"', 1)[0]


def status_labels(session):
    return [n.get("label") for n in session.nodes() if n.get("role") == "Status"]


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
        session.call("settle", {"settle": {"settle_timeout_ms": 2000}})
        session.click(session.find("Project", timeout=10))
        session.call("settle", {"settle": {"settle_timeout_ms": 2000}})

        # The screen is a named form, so a screen-reader user can jump to it.
        form = session.find("Project settings", timeout=5, role="Form")
        checks.check(form is not None, "the settings form is a named landmark")

        # Every field shows what the manifest holds.
        for label, expected in FROM_THE_MANIFEST.items():
            node = session.find(label, timeout=5, role="TextInput")
            checks.check(
                node is not None and node.get("value") == expected,
                f"{label!r} reads {expected!r} from the manifest",
            )

        combo = session.find("Language", timeout=5, role="ComboBox")
        checks.check(
            combo is not None and combo.get("value") == "Rust",
            "US-PROJ-01 the language combo shows the manifest's target",
        )

        # US-PROJ-03: an empty prefix path names the default the backend will use.
        prefix = session.find("Prefix path", timeout=5, role="TextInput")
        session.call("set_value", {"node": prefix["id"], "value": ""})
        session.call("settle", {"settle": {"settle_timeout_ms": 1500}})
        prefix = session.find("Prefix path", timeout=5, role="TextInput")
        checks.check(
            placeholder_of(session, prefix) == "default: crates",
            "US-PROJ-03 the placeholder names Rust's default prefix",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-project.png"))

        # US-PROJ-01: both targets are offered.
        session.click(combo)
        session.call("settle", {"settle": {"settle_timeout_ms": 1500}})
        offered = [
            n.get("label")
            for n in session.nodes()
            if n.get("role") == "ListBoxOption"
        ]
        checks.check(
            offered == ["Rust", "C++ / Qt"],
            f"US-PROJ-01 the combo offers both targets, got {offered}",
        )

        # Chosen with the keyboard: a list-box option publishes no click action, so
        # this is both the reachable path and the one a keyboard user takes.
        combo = session.find("Language", timeout=5, role="ComboBox")
        session.call("focus_node", {"node": combo["id"]})
        session.call("inject_key", {"key": "ArrowDown"})
        session.call("inject_key", {"key": "Enter"})
        session.call("settle", {"settle": {"settle_timeout_ms": 2000}})
        combo = session.find("Language", timeout=5, role="ComboBox")
        checks.check(
            combo is not None and combo.get("value") == "C++ / Qt",
            "US-PROJ-01 choosing a target updates the combo",
        )

        # US-PROJ-03: and the placeholder follows it.
        prefix = session.find("Prefix path", timeout=5, role="TextInput")
        checks.check(
            placeholder_of(session, prefix) == "default: src",
            "US-PROJ-03 the placeholder follows the chosen language",
        )

        # US-PROJ-04: choosing a language is an edit, so the manifest is now dirty.
        checks.check(
            session.find("Save manifest", timeout=5, role="Button") is not None,
            "US-PROJ-04 an edit marks the manifest dirty",
        )

        # US-PROJ-02: each required field carries its own message, as the value
        # changes rather than after a commit.
        app_name = session.find("Application name", timeout=5, role="TextInput")
        session.call("set_value", {"node": app_name["id"], "value": ""})
        session.call("settle", {"settle": {"settle_timeout_ms": 1500}})
        checks.check(
            "Application name is required" in status_labels(session),
            "US-PROJ-02 an empty application name says so",
        )
        checks.check(
            "Organisation name is required" not in status_labels(session),
            "US-PROJ-02 a field that is filled in stays quiet",
        )

        org = session.find("Organisation name", timeout=5, role="TextInput")
        session.call("set_value", {"node": org["id"], "value": "  "})
        session.call("settle", {"settle": {"settle_timeout_ms": 1500}})
        checks.check(
            "Organisation name is required" in status_labels(session),
            "US-PROJ-02 whitespace is not a name",
        )
        session.call("set_value", {"node": org["id"], "value": "FernTech"})
        session.call("settle", {"settle": {"settle_timeout_ms": 1500}})
        checks.check(
            "Organisation name is required" not in status_labels(session),
            "US-PROJ-02 the message clears when the field is filled again",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-project-invalid.png"))

        # US-PROJ-04: an edit is committed when the field is finished with. Focus
        # the field, change it, then move focus away, which is the blur path; Enter
        # is the other and is exercised below.
        app_name = session.find("Application name", timeout=5, role="TextInput")
        session.call("focus_node", {"node": app_name["id"]})
        session.call("set_value", {"node": app_name["id"], "value": "ProbedByBlur"})
        org = session.find("Organisation name", timeout=5, role="TextInput")
        session.call("focus_node", {"node": org["id"]})
        session.call("settle", {"settle": {"settle_timeout_ms": 1500}})

        domain = session.find("Organisation domain", timeout=5, role="TextInput")
        session.call("focus_node", {"node": domain["id"]})
        session.call("set_value", {"node": domain["id"], "value": "eu.probed"})
        session.call("inject_key", {"key": "Enter"})
        session.call("settle", {"settle": {"settle_timeout_ms": 1500}})

        # Write it out, then read the file. This is the assertion the form itself
        # cannot make: an uncommitted edit reads back from the widget either way.
        save = session.find("Save manifest", timeout=5, role="Button")
        checks.check(save is not None, "US-PROJ-04 there is something to save")
        if save is not None:
            session.click(save)
            session.call("settle", {"settle": {"settle_timeout_ms": 3000}})

        with open(manifest, "r", encoding="utf-8") as fh:
            written = fh.read()
        checks.check(
            "application_name: ProbedByBlur" in written,
            "US-PROJ-04 an edit committed on blur reaches the file",
        )
        checks.check(
            "domain: eu.probed" in written,
            "US-PROJ-04 an edit committed on Enter reaches the file",
        )
        checks.check(
            "language: cpp-qt" in written,
            "US-PROJ-01 the combo writes the code, not the label",
        )
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
