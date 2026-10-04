"""Offline regression checks for application-specific probe navigation."""
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ui_probe_helpers as helpers


class RowNavigationTests(unittest.TestCase):
    def test_realized_row_does_not_scroll(self):
        session = Mock()
        row = {'id': 1}
        with patch.object(helpers.tree, 'wait_for_node', return_value=row), \
             patch.object(helpers.navigate, 'scroll_until_found') as scroll, \
             patch.object(helpers.navigate, 'click') as click:
            helpers.select(session, 'Global', (0, 430))
        scroll.assert_not_called()
        click.assert_called_once_with(session, row)

    def test_unrealized_row_is_scrolled_into_the_correct_column(self):
        session = Mock()
        container, row = {'id': 2}, {'id': 3}
        with patch.object(helpers.tree, 'wait_for_node', side_effect=[None, container]), \
             patch.object(helpers.navigate, 'scroll_until_found', return_value=row) as scroll, \
             patch.object(helpers.navigate, 'click') as click:
            helpers.select(session, 'Global', (0, 430))
        self.assertEqual(scroll.call_args.args, (session, container))
        predicate = scroll.call_args.kwargs['pred']
        self.assertTrue(predicate({'bounds': {'x': 172}}))
        self.assertFalse(predicate({'bounds': {'x': 670}}))
        click.assert_called_once_with(session, row)

    def test_missing_row_is_a_behavior_failure_and_is_not_clicked(self):
        with patch.object(helpers.tree, 'wait_for_node', return_value=None), \
             patch.object(helpers.navigate, 'click') as click:
            with self.assertRaisesRegex(AssertionError, 'missing list row: Missing'):
                helpers.select(Mock(), 'Missing', (0, 430))
        click.assert_not_called()


if __name__ == '__main__':
    unittest.main()
