"""Strict Qleany assertions shared by live UI probes (not a generated harness)."""
import time
from pathlib import Path

import yaml
from teksilo_probe import navigate, tree


def read_manifest(path):
    return yaml.safe_load(Path(path).read_text())


def named(items, name):
    return next(item for item in items if item['name'] == name)


def wait(session, predicate, timeout=5):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        session.settle()
        if predicate():
            return True
        time.sleep(0.05)
    return False


def click(session, label, role=None):
    node = tree.wait_for_node(session, label=label, role=role, timeout=5)
    if node is None:
        raise AssertionError(f'missing {role or "control"}: {label}')
    navigate.click(session, node)


def rows(session, column):
    return sorted((n for n in tree.nodes(session) if n.get('role') == 'ListBoxOption'
                   and column[0] <= n.get('bounds', {}).get('x', -1) < column[1]
                   and (len(column) < 3 or n.get('label') in column[2])),
                  key=lambda n: n['bounds']['y'])


def labels(session, column):
    return [n.get('label') for n in rows(session, column)]


def select(session, name, column):
    in_column = lambda n: column[0] <= n.get('bounds', {}).get('x', -1) < column[1]
    node = tree.wait_for_node(session, role='ListBoxOption', label=name,
                              pred=in_column, timeout=3)
    if node is None:
        container = tree.wait_for_node(session, role='ListBox', pred=in_column, timeout=5)
        if container is not None:
            node = navigate.scroll_until_found(session, container, role='ListBoxOption',
                                               label=name, pred=in_column, row_role='ListBoxOption')
    if node is None:
        raise AssertionError(f'missing list row: {name}')
    navigate.click(session, node)


def menu_delete(session, name, column, action="Delete field"):
    # Undo notifications can cover the trailing button. Dismiss them as a user
    # would before testing the actual pointer target.
    for _ in range(20):
        statuses = tree.find_all(session, role='Status')
        dismiss = next((n for status in statuses for n in tree.descendants(session, status)
                        if n.get('role') == 'Button' and n.get('label') == 'Clear'), None)
        if dismiss is None:
            break
        navigate.click(session, dismiss)
    # Exercise the user's pointer click, not show_context_menu directly.
    row = next(n for n in rows(session, column) if n.get('label') == name)
    bounds = row['bounds']
    buttons = [n for n in tree.nodes(session) if n.get('role') == 'Button'
               and n.get('label') == 'More actions']
    button = next(n for n in buttons if bounds['x'] <= tree.center(n)[0] < bounds['x'] + bounds['width']
                  and bounds['y'] <= tree.center(n)[1] < bounds['y'] + bounds['height'])
    x, y = tree.center(button)
    session.tools.inject_pointer(x=x, y=y, action='click')
    session.settle()
    item = tree.wait_for_node(session, label=action, timeout=4)
    if item is None:
        raise AssertionError(f'{name}: pointer did not open its {action} menu')
    navigate.click(session, item)


def save(session):
    session.tools.inject_key(key='s', command=True)
    if not wait(session, lambda: tree.find(session, label='No unsaved changes') is not None):
        raise AssertionError('Save did not finish cleanly')


def history(session, redo=False):
    # Both redo accelerators are exercised by the callers.
    session.tools.inject_key(key='z', command=True, shift=redo)
    session.settle()
