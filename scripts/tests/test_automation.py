"""Offline checks for Qleany's integration with the generated Teksilo harness."""
import contextlib
import importlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import automation_fixture as fixture
from teksilo_probe import ProbeError, ToolError

PROBES = sorted(p.stem for p in Path(fixture.repo_path("scripts")).glob("automation_*.py")
                if p.stem != "automation_fixture")


class ProbeLifecycleTests(unittest.TestCase):
    def run_probe(self, name, failure_after_attach):
        probe = importlib.import_module(name)
        app, session = Mock(), Mock()
        app.log_tail.return_value = "test log"
        with tempfile.TemporaryDirectory() as scratch, contextlib.ExitStack() as stack:
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            stack.enter_context(patch.object(fixture, "SCRATCH", scratch))
            stack.enter_context(patch.object(fixture, "app_binary", return_value="/unused/qleany"))
            launch = stack.enter_context(patch.object(probe, "launch_and_attach"))
            if failure_after_attach:
                launch.return_value = app, session
                stack.enter_context(patch.object(probe.tree, "wait_for_node", side_effect=ProbeError("snapshot failed")))
            else:
                launch.side_effect = RuntimeError("bridge unavailable")
            self.assertEqual(probe.main(), 1, name)
        if failure_after_attach:
            self.assertTrue(session.close.called, name)
            self.assertTrue(app.terminate.called, name)

    def test_launch_errors_are_reported_by_every_probe(self):
        for name in PROBES:
            with self.subTest(probe=name):
                self.run_probe(name, False)

    def test_tools_failing_after_attach_always_clean_up(self):
        for name in PROBES:
            with self.subTest(probe=name):
                self.run_probe(name, True)

    def test_shell_absent_behavior_is_exit_two(self):
        import automation_shell as probe
        app, session = Mock(), Mock()
        app.log_tail.return_value = "test log"
        with contextlib.ExitStack() as stack:
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            stack.enter_context(patch.object(fixture, "app_binary", return_value="/unused/qleany"))
            stack.enter_context(patch.object(fixture, "isolated_config", return_value={}))
            stack.enter_context(patch.object(probe, "launch_and_attach", return_value=(app, session)))
            stack.enter_context(patch.object(probe.tree, "wait_for_node", return_value={"disabled": False}))
            stack.enter_context(patch.object(probe.tree, "labels", return_value=["Home"]))
            stack.enter_context(patch.object(probe.navigate, "click"))
            stack.enter_context(patch.object(probe.shot, "save"))
            self.assertEqual(probe.main(), 2)
        session.close.assert_called_once()
        app.terminate.assert_called_once()


class NavigationTests(unittest.TestCase):
    def test_refinds_only_when_the_action_was_not_applied(self):
        session = Mock()
        old, new = {"id": 1}, {"id": 2}
        with patch.object(fixture.tree, "wait_for_node", side_effect=[old, new]), \
             patch.object(fixture.navigate, "click", side_effect=[ToolError("invoke_action", "NOT_FOUND", "stale"), None]) as click:
            fixture.go_to(session, "Project")
        self.assertEqual([call.args[1] for call in click.call_args_list], [old, new])

    def test_does_not_retry_ambiguous_or_rejected_actions(self):
        for error in (ProbeError("connection lost"), ToolError("invoke_action", "UNHANDLED_ACTION", "disabled")):
            with self.subTest(error=error), \
                 patch.object(fixture.tree, "wait_for_node", return_value={"id": 1}), \
                 patch.object(fixture.navigate, "click", side_effect=error) as click:
                with self.assertRaises(ProbeError):
                    fixture.go_to(Mock(), "Project")
                click.assert_called_once()


class FixtureTests(unittest.TestCase):
    def test_copy_preserves_filename_and_isolates_saves(self):
        with tempfile.TemporaryDirectory() as scratch, patch.object(fixture, "SCRATCH", scratch):
            source = Path(scratch) / "input" / "qleany.yaml"
            source.parent.mkdir()
            source.write_text("original")
            source.chmod(0o444)
            copy = Path(fixture.working_copy(source, "save"))
            self.assertEqual(copy.name, "qleany.yaml")
            copy.write_text("edited")
            self.assertEqual(source.read_text(), "original")

    def test_scratch_inside_repository_rejected_before_copy(self):
        with patch.object(fixture, "SCRATCH", fixture.repo_root()), patch.object(fixture.shutil, "copy2") as copy:
            with self.assertRaises(RuntimeError):
                fixture.working_copy(fixture.repo_path("qleany.yaml"), "unsafe")
            copy.assert_not_called()

    def test_topmost_dialog_excludes_background_and_other_dialogs(self):
        nodes = [{"id": 1, "role": "Dialog", "children": [2]},
                 {"id": 2, "role": "Button"},
                 {"id": 3, "role": "Dialog", "children": [4, 5]},
                 {"id": 4, "role": "Button"}, {"id": 5, "role": "Label"}]
        with patch.object(fixture.tree, "nodes", return_value=nodes):
            self.assertEqual([n["id"] for n in fixture.overlay_nodes(Mock())], [3, 4, 5])


if __name__ == "__main__":
    unittest.main()
