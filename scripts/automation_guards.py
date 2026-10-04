#!/usr/bin/env python3
"""Exercise Save/Discard/Cancel guards and failed-save recovery through the UI."""
import os
from pathlib import Path
import sys

import automation_fixture as fixture
from teksilo_probe import ProbeError, Report, launch_and_attach, tree
from ui_probe_helpers import click, read_manifest, wait


def dirty(session, value):
    fixture.go_to(session, 'Project')
    node = tree.wait_for_node(session, role='TextInput', label='Application name', timeout=5)
    session.tools.focus_node(node=node['id'])
    session.tools.set_value(node=node['id'], value=value)
    session.settle()
    fixture.go_to(session, 'Home')


def close_window(session):
    node = tree.wait_for_node(session, role='Button', label='Close', timeout=5)
    x, y = tree.center(node)
    session.tools.inject_pointer(x=x, y=y, action='click')
    session.settle()


def main():
    report = Report('guards')
    app = session = None
    path = backup = None
    try:
        path = Path(fixture.working_copy(fixture.repo_path('qleany.yaml'), 'guards'))
        env = fixture.isolated_config('guards')
        env['QLEANY_DEV'] = '1'
        app, session = launch_and_attach(argv=[fixture.app_binary()], cwd=str(path.parent), env=env, label='guards')
        click(session, 'Open Qleany manifest')
        for action, value in [('New manifest', 'SavedBeforeNew'),
                              ('Open Qleany manifest', 'SavedBeforeOpen'),
                              ('Close current manifest', 'SavedBeforeClose')]:
            dirty(session, value)
            click(session, action)
            report.check(tree.find(session, label='Save', role='Button') is not None, f'{action}: Save choice appears')
            click(session, 'Save', 'Button')
            report.check(wait(session, lambda: read_manifest(path)['global']['application_name'] == value), f'{action}: Save writes pending text before proceeding')
            if action == 'New manifest':
                report.check(tree.find(session, label='Next', role='Button') is not None, 'Save then New opens wizard')
                click(session, 'Cancel', 'Button')
            elif action == 'Close current manifest':
                report.check(tree.find(session, label='Entities').get('disabled', False), 'Save then Close clears workspace')
                click(session, 'Open Qleany manifest')
            else:
                fixture.go_to(session, 'Project')
                report.check(tree.find(session, label='Application name')['value'] == value, 'replacement reloads the saved value')

        dirty(session, 'RecoverAfterFailedSave')
        before = path.read_bytes()
        backup = path.with_suffix('.backup')
        path.rename(backup)
        path.mkdir()  # deterministic write failure, including when run as root
        click(session, 'Close current manifest')
        click(session, 'Save', 'Button')
        report.check(not tree.find(session, label='Entities').get('disabled', False), 'failed Save blocks Close')
        report.check(tree.find(session, label='Save manifest') is not None, 'failed Save retains dirty state')
        report.check(backup.read_bytes() == before, 'failed Save preserves previous file')
        report.check(any('Error:' in str(n.get(key, '')) for n in tree.nodes(session) for key in ('label', 'value')), 'failed Save reports an error')

        close_window(session)
        click(session, 'Cancel', 'Button')
        report.check(app.proc.poll() is None, 'Cancel keeps dirty window alive')
        close_window(session)
        click(session, 'Save', 'Button')
        report.check(app.proc.poll() is None, 'failed Save blocks Quit')
        path.rmdir()
        backup.rename(path)
        backup = None
        close_window(session)
        try:
            click(session, 'Save', 'Button')
        except ProbeError:
            pass  # the actual process exit and saved file are the assertions
        report.check(app.proc.wait(timeout=10) == 0, 'successful Save then Quit exits cleanly')
        report.check(read_manifest(path)['global']['application_name'] == 'RecoverAfterFailedSave', 'Quit saves the pending value after recovery')
        session.close()
        session = None
        app.terminate()
        app = None

        # Discard on a dirty window must exit without changing the file.
        before = path.read_bytes()
        app, session = launch_and_attach(argv=[fixture.app_binary()], cwd=str(path.parent), env=env, label='guards-discard')
        click(session, 'Open Qleany manifest')
        dirty(session, 'MustNotBeSaved')
        close_window(session)
        try:
            click(session, 'Discard', 'Button')
        except ProbeError:
            pass
        report.check(app.proc.wait(timeout=10) == 0, 'Discard then Quit exits cleanly')
        report.check(path.read_bytes() == before, 'Discard leaves saved manifest unchanged')
    except AssertionError as exc:
        report.check(False, str(exc))
    except Exception as exc:
        report.error(f'{type(exc).__name__}: {exc}')
    finally:
        if session:
            session.close()
        if app:
            if report.exit_code:
                report.note(app.log_tail())
            app.terminate()
        if backup and backup.exists():
            if path.is_dir():
                path.rmdir()
            backup.rename(path)
    return report.finish()


if __name__ == '__main__':
    sys.exit(main())
