#!/usr/bin/env python3
"""Probe: the new-manifest wizard.

Covers US-WIZ-01 (four numbered steps, Back from step 2, Create on step 4, and
three ways to dismiss), US-WIZ-02 (the language choice), US-WIZ-03 (the two names
gate Next), US-WIZ-04 (the four templates) and US-WIZ-05 (the frontends each
language offers).

US-WIZ-06's write is not driven here. Create opens the OS save dialog, which a
probe cannot answer, so what the file ends up containing is covered by the
view-model's unit tests instead.
"""

import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = ("US-WIZ-01", "US-WIZ-02", "US-WIZ-03", "US-WIZ-04", "US-WIZ-05")
PROBE = "wizard"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle": {"settle_timeout_ms": 3000}}


def roles(session, role):
    return [n for n in session.nodes() if n.get("role") == role]


def labels(session, role):
    return [n.get("label") for n in roles(session, role) if n.get("label")]


def button(session, label):
    for n in roles(session, "Button"):
        if n.get("label") == label:
            return n
    return None


def wizard_is_open(session):
    return "Cancel" in labels(session, "Button") and bool(labels(session, "ListItem"))


def open_wizard(session):
    session.click(button(session, "New manifest"))
    session.call("settle", SETTLE)


def main():
    checks = fixture.Checks()
    log = os.path.join(fixture.SCRATCH, f"qleany-{PROBE}-{os.getpid()}.log")

    manifest = fixture.working_copy(fixture.repo_path("qleany.yaml"), PROBE)
    env = fixture.isolated_config(PROBE)
    env["QLEANY_DEV"] = "1"

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

        open_wizard(session)

        # US-WIZ-01: four steps, named and in order.
        steps = labels(session, "ListItem")
        checks.check(
            steps == ["Language", "Names", "Template", "User interfaces"],
            f"US-WIZ-01 the four steps are named and in order, got {steps}",
        )
        checks.check(
            button(session, "Back") is None,
            "US-WIZ-01 step 1 offers no Back",
        )
        checks.check(button(session, "Next") is not None, "US-WIZ-01 step 1 offers Next")
        checks.check(button(session, "Cancel") is not None, "US-WIZ-01 and Cancel")

        # US-WIZ-02: the two languages, one at a time.
        radios = labels(session, "RadioButton")
        checks.check(
            radios == ["Rust", "C++ / Qt"],
            f"US-WIZ-02 step 1 offers both languages, got {radios}",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-wizard.png"))

        # US-WIZ-03: the names gate Next.
        session.click(button(session, "Next"))
        session.call("settle", SETTLE)
        checks.check(
            button(session, "Back") is not None,
            "US-WIZ-01 Back appears from step 2",
        )
        next_button = button(session, "Next")
        checks.check(
            next_button is not None and next_button.get("disabled", False),
            "US-WIZ-03 Next is disabled while the names are empty",
        )

        app_name = next(
            (n for n in roles(session, "TextInput") if n.get("label") == "Application name"),
            None,
        )
        checks.check(app_name is not None, "US-WIZ-03 step 2 asks for an application name")
        if app_name is not None:
            session.call("set_value", {"node": app_name["id"], "value": "my app"})
            session.call("settle", SETTLE)
            statuses = [n.get("label") for n in roles(session, "Status")]
            checks.check(
                "Must be PascalCase" in statuses,
                f"US-WIZ-03 a badly cased name says so, got {statuses}",
            )
            app_name = next(
                n for n in roles(session, "TextInput") if n.get("label") == "Application name"
            )
            session.call("set_value", {"node": app_name["id"], "value": "ProbedApp"})
            session.call("settle", SETTLE)

        next_button = button(session, "Next")
        checks.check(
            next_button is not None and next_button.get("disabled", False),
            "US-WIZ-03 and still disabled with no organisation",
        )
        org = next(
            n for n in roles(session, "TextInput") if n.get("label") == "Organisation name"
        )
        session.call("set_value", {"node": org["id"], "value": "FernTech"})
        session.call("settle", SETTLE)
        next_button = button(session, "Next")
        checks.check(
            next_button is not None and not next_button.get("disabled", False),
            "US-WIZ-03 both names make Next live",
        )

        # US-WIZ-04: the four templates.
        session.click(button(session, "Next"))
        session.call("settle", SETTLE)
        templates = labels(session, "RadioButton")
        checks.check(
            templates == ["Blank", "Minimal", "Document editor", "Data management"],
            f"US-WIZ-04 the four templates are offered, got {templates}",
        )

        # US-WIZ-05: the frontends of the chosen language, and Create instead of Next.
        session.click(button(session, "Next"))
        session.call("settle", SETTLE)
        ticks = labels(session, "CheckBox")
        checks.check(
            ticks == ["CLI", "Teksilo (recommended)", "Slint"],
            f"US-WIZ-05 a Rust manifest offers the Rust frontends, got {ticks}",
        )
        teksilo = next(
            (n for n in roles(session, "CheckBox") if n.get("label") == "Teksilo (recommended)"),
            None,
        )
        checks.check(
            teksilo is not None and teksilo.get("toggled") == "true",
            "US-WIZ-05 Teksilo is ticked by default",
        )
        checks.check(
            button(session, "Create") is not None,
            "US-WIZ-01 Create replaces Next on the last step",
        )
        checks.check(
            button(session, "Next") is None,
            "US-WIZ-01 and Next is gone",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-wizard-targets.png"))

        # US-WIZ-01: Cancel dismisses, and nothing was created.
        session.click(button(session, "Cancel"))
        session.call("settle", SETTLE)
        checks.check(not wizard_is_open(session), "US-WIZ-01 Cancel dismisses the wizard")
        checks.check(
            button(session, "Save manifest").get("disabled", False),
            "US-WIZ-01 and created nothing",
        )

        # US-WIZ-01: Escape dismisses too.
        open_wizard(session)
        checks.check(wizard_is_open(session), "the wizard reopens")
        session.call("inject_key", {"key": "Escape"})
        session.call("settle", SETTLE)
        checks.check(not wizard_is_open(session), "US-WIZ-01 Escape dismisses the wizard")

        # US-WIZ-02: the language drives step 4.
        open_wizard(session)
        cpp = next(
            (n for n in roles(session, "RadioButton") if n.get("label") == "C++ / Qt"), None
        )
        checks.check(cpp is not None, "US-WIZ-02 the other language can be chosen")
        if cpp is not None:
            session.click(cpp)
            session.call("settle", SETTLE)
        for _ in range(3):
            session.click(button(session, "Next"))
            session.call("settle", SETTLE)
        ticks = labels(session, "CheckBox")
        checks.check(
            ticks == ["Qt Quick", "Qt Widgets"],
            f"US-WIZ-05 a C++/Qt manifest offers the Qt frontends, got {ticks}",
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
