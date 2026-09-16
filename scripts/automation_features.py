#!/usr/bin/env python3
"""Probe: the Features screen.

Covers US-FEAT-01 to US-FEAT-06 (features, use cases, the three flags and the
row subtitle), US-DTO-01/02/03 (enabling, the confirmation before destroying one,
and what a disabled pane says), US-DTO-05/07 (fields and the eight types) and
US-DTO-09/10 (entity associations, and the two panes being independent).

The two DTO panes are the part worth driving rather than reasoning about: the
generated session mints one DTO handle and wires it, so a second pane is easy to
build and easy to leave deaf. The probe reads a real manifest where the two sides
hold different DTOs, which is the assertion that they are two.
"""

import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import automation_fixture as fixture

STORIES = (
    "US-FEAT-01", "US-FEAT-02", "US-FEAT-04", "US-FEAT-06",
    "US-DTO-01", "US-DTO-02", "US-DTO-03", "US-DTO-05", "US-DTO-07",
    "US-DTO-09", "US-DTO-10",
)
PROBE = "features"
SHOT_DIR = os.environ.get("QLEANY_SHOT_DIR", tempfile.gettempdir())
SETTLE = {"settle": {"settle_timeout_ms": 3000}}

#: The three list columns, by where they start.
FEATURE_COLUMN = (0, 393)
USE_CASE_COLUMN = (393, 654)
DETAIL_COLUMN = (654, 1400)


def in_column(node, column):
    x = node.get("bounds", {}).get("x")
    return x is not None and column[0] <= x < column[1]


def rows(session, column):
    listed = [
        n for n in session.nodes()
        if n.get("role") == "ListBoxOption" and in_column(n, column)
    ]
    return [
        n.get("label")
        for n in sorted(listed, key=lambda n: n.get("bounds", {}).get("y", 0))
    ]


def texts(session):
    """Every string on screen.

    `description` is in the list because that is where a list row's subtitle goes:
    the row's own node carries its label, and the subtitle is published as the
    node's description rather than as a separate text run.
    """
    out = []
    for n in session.nodes():
        for key in ("label", "value", "description"):
            if n.get(key):
                out.append(n[key])
    return out


def statuses(session):
    return [n.get("label") for n in session.nodes() if n.get("role") == "Status"]


def checkboxes(session):
    out = {}
    for n in session.nodes():
        if n.get("role") == "CheckBox":
            out.setdefault(n.get("label"), n)
    return out


def text_inputs(session, column=DETAIL_COLUMN):
    return [
        n for n in session.nodes()
        if n.get("role") == "TextInput" and in_column(n, column)
    ]


def named_input(session, label, column=DETAIL_COLUMN):
    for n in text_inputs(session, column):
        if n.get("label") == label:
            return n
    return None


def click_row(session, label, column):
    for n in session.nodes():
        if n.get("role") == "ListBoxOption" and n.get("label") == label and in_column(n, column):
            session.click(n)
            session.call("settle", SETTLE)
            return True
    return False


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

        session.click(session.find("Open Qleany manifest", timeout=20))
        session.call("settle", SETTLE)
        session.click(session.find("Features", timeout=10))
        session.call("settle", SETTLE)

        features = rows(session, FEATURE_COLUMN)
        checks.check(
            features[:2] == ["handling_app_lifecycle", "handling_manifest"],
            f"the features are listed in manifest order, got {features[:2]}",
        )
        checks.check(
            rows(session, USE_CASE_COLUMN) == [],
            "no feature is selected, so no use cases are listed",
        )
        checks.check(
            checkboxes(session).get("Enable DTO In") is None,
            "US-DTO-03 the DTO panes are not there without a use case",
        )

        # US-FEAT-06: the subtitle joins the flags that are set, and only those.
        checks.check(click_row(session, "handling_manifest", FEATURE_COLUMN),
                     "a feature can be selected")
        use_cases = rows(session, USE_CASE_COLUMN)
        checks.check(
            use_cases == ["load", "save", "create", "close", "export_to_mermaid", "check"],
            f"US-FEAT-04 the use cases are listed, got {use_cases}",
        )
        shown = texts(session)
        checks.check("input · output" in shown, "US-FEAT-06 load has both DTOs")
        checks.check("input" in shown, "US-FEAT-06 save has only an input")
        checks.check("RO · input · output" in shown, "US-FEAT-06 create is read only")
        checks.check(
            "RO · output" in shown,
            "US-FEAT-06 the flags join with a middle dot in a fixed order",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-features.png"))

        # US-DTO-09: the entities a use case touches, read from the manifest.
        checks.check(click_row(session, "load", USE_CASE_COLUMN), "a use case can be selected")
        boxes = checkboxes(session)
        checks.check(
            boxes.get("Entity", {}).get("toggled") == "true",
            "US-DTO-09 an associated entity is ticked",
        )
        checks.check(
            boxes.get("File", {}).get("toggled") == "false",
            "US-DTO-09 an entity the use case does not touch is not",
        )
        checks.check(
            "Workspace" in boxes,
            "US-DTO-09 every entity that can be instantiated is offered",
        )
        checks.check(
            "EntityBase" not in boxes,
            "US-DTO-09 a heritage-only entity is not: it is never instantiated",
        )

        # US-DTO-10: the two panes hold different DTOs. The input pane is the open
        # tab; the output one is behind its own.
        checks.check(
            named_input(session, "Name") is not None,
            "US-DTO-01 the input DTO pane shows a name",
        )
        names = [n.get("value") for n in text_inputs(session)]
        checks.check(
            "LoadDto" in names,
            f"US-DTO-01 the input DTO is named after its use case, got {names}",
        )
        out_tab = session.find("Output DTO", timeout=3)
        checks.check(out_tab is not None, "US-DTO-10 there is an output tab")
        if out_tab is not None:
            session.click(out_tab)
            session.call("settle", SETTLE)
            names = [n.get("value") for n in text_inputs(session)]
            checks.check(
                "LoadReturnDto" in names,
                f"US-DTO-10 the output pane holds its own DTO, got {names}",
            )
            checks.check(
                "LoadDto" not in names,
                "US-DTO-10 and not the input one",
            )

        # US-DTO-05/07: a field, and the eight types with no Entity among them.
        add = session.find("Add field", timeout=3)
        checks.check(add is not None, "US-DTO-05 the DTO pane offers a plus")
        if add is not None:
            session.click(add)
            session.call("settle", SETTLE)
            checks.check(
                "new_field" in texts(session),
                "US-DTO-05 the new field is appended",
            )

        type_combo = next(
            (n for n in session.nodes()
             if n.get("role") == "ComboBox" and n.get("label") == "Type"),
            None,
        )
        checks.check(type_combo is not None, "US-DTO-07 the field has a type combo")
        if type_combo is not None:
            before = {n["id"] for n in session.nodes() if n.get("role") == "ListBoxOption"}
            session.click(type_combo)
            session.call("settle", SETTLE)
            offered = [
                n.get("label")
                for n in sorted(
                    (n for n in session.nodes()
                     if n.get("role") == "ListBoxOption"
                     and n["id"] not in before and n.get("label")),
                    key=lambda n: n.get("bounds", {}).get("y", 0),
                )
            ]
            checks.check(
                offered == ["Boolean", "Integer", "UInteger", "Float", "String",
                            "Uuid", "DateTime", "Enum"],
                f"US-DTO-07 a DTO field offers eight primitives, got {offered}",
            )
            checks.check(
                "Entity" not in offered,
                "US-DTO-07 and never an entity: that is what a DTO exists to avoid",
            )
            session.call("inject_key", {"key": "Escape"})
            session.call("settle", SETTLE)

        # US-DTO-02: unticking asks first, and cancelling leaves it ticked.
        toggle = checkboxes(session).get("Enable DTO Out")
        checks.check(toggle is not None, "US-DTO-02 the output pane has its toggle")
        if toggle is not None:
            session.click(toggle)
            session.call("settle", SETTLE)
            spoken = texts(session)
            checks.check(
                any("will be deleted" in t for t in spoken),
                f"US-DTO-02 unticking asks before destroying the DTO",
            )
            no = session.find("No", timeout=3)
            checks.check(no is not None, "US-DTO-02 the question can be declined")
            if no is not None:
                session.click(no)
                session.call("settle", SETTLE)
                checks.check(
                    checkboxes(session).get("Enable DTO Out", {}).get("toggled") == "true",
                    "US-DTO-02 declining leaves the DTO, and the box ticked",
                )

        # US-FEAT-01/02: a feature, and its name validation.
        session.click(session.find("Add feature", timeout=3))
        session.call("settle", SETTLE)
        checks.check(
            rows(session, FEATURE_COLUMN)[-1] == "new_feature",
            "US-FEAT-01 the new feature is appended",
        )
        # The feature's name sits over its use cases, in the middle column, which is
        # where the Slint UI had it too: a feature has exactly one property, and the
        # detail column belongs to the use case.
        name = named_input(session, "Name", USE_CASE_COLUMN)
        checks.check(name is not None, "US-FEAT-02 the feature form is showing")
        if name is not None:
            session.call("set_value", {"node": name["id"], "value": "Not Snake"})
            session.call("settle", SETTLE)
            checks.check(
                "Feature name must be in snake_case" in statuses(session),
                "US-FEAT-02 a badly cased feature name says so",
            )
            name = named_input(session, "Name", USE_CASE_COLUMN)
            session.call("focus_node", {"node": name["id"]})
            session.call("set_value", {"node": name["id"], "value": "probed_feature"})
            session.call("inject_key", {"key": "Enter"})
            session.call("settle", SETTLE)
            checks.check(
                "probed_feature" in rows(session, FEATURE_COLUMN),
                "US-FEAT-02 the committed name reaches the list",
            )

        # US-FEAT-04: a use case, with every flag off.
        session.click(session.find("Add use case", timeout=3))
        session.call("settle", SETTLE)
        checks.check(
            rows(session, USE_CASE_COLUMN) == ["new_use_case"],
            "US-FEAT-04 the new use case is the only one of the new feature",
        )
        boxes = checkboxes(session)
        checks.check(
            all(boxes.get(flag, {}).get("toggled") == "false"
                for flag in ("Read only", "Undoable", "Long operation")),
            "US-FEAT-04 a new use case has every flag off",
        )
        # The output tab is still the open one: the tab selection survives a
        # rebuild, which is the point of it. Switch back to the input pane.
        in_tab = session.find("Input DTO", timeout=3)
        checks.check(in_tab is not None, "US-DTO-10 the input tab is still reachable")
        if in_tab is not None:
            session.click(in_tab)
            session.call("settle", SETTLE)
        boxes = checkboxes(session)
        checks.check(
            boxes.get("Enable DTO In", {}).get("toggled") == "false",
            "US-FEAT-04 and no DTOs",
        )

        # US-FEAT-06: ticking Read only takes Undoable away, because a use case that
        # writes nothing has nothing to undo.
        session.click(boxes["Read only"])
        session.call("settle", SETTLE)
        checks.check(
            "Undoable" not in checkboxes(session),
            "US-FEAT-06 Undoable is hidden while Read only is ticked",
        )

        # US-DTO-01: enabling creates a DTO named after the use case.
        toggle_in = checkboxes(session).get("Enable DTO In")
        checks.check(toggle_in is not None, "US-DTO-01 the input pane has its toggle")
        if toggle_in is not None:
            session.click(toggle_in)
            session.call("settle", SETTLE)
        names = [n.get("value") for n in text_inputs(session)]
        checks.check(
            "NewUseCaseDto" in names,
            f"US-DTO-01 the DTO is named after its use case, got {names}",
        )

        session.shot(os.path.join(SHOT_DIR, "qleany-features-dto.png"))

        # Everything above could pass against forms that write nothing.
        save = session.find("Save manifest", timeout=5, role="Button")
        checks.check(save is not None, "there is something to save")
        if save is not None:
            session.click(save)
            session.call("settle", SETTLE)
        with open(manifest, "r", encoding="utf-8") as fh:
            written = fh.read()
        checks.check("probed_feature" in written, "US-FEAT-02 the feature is in the file")
        checks.check("new_use_case" in written, "US-FEAT-04 the use case is in the file")
        checks.check("NewUseCaseDto" in written, "US-DTO-01 the DTO is in the file")
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
