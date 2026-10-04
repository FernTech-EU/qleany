#!/usr/bin/env python3
"""Pointer reordering and destructive row menus, with exact YAML and undo checks."""
import os
from pathlib import Path
import sys

import yaml
import automation_fixture as fixture
from teksilo_probe import Report, launch_and_attach, navigate, tree
from ui_probe_helpers import click, history, labels, menu_delete, named, read_manifest, rows, save, select, wait

ENTITY = (0, 430)
FIELD = (430, 900)
FEATURE = (0, 393)
USE_CASE = (393, 654)
DTO = (654, 1400)


def reorder(session, report, path, column, collection, title):
    before = [r['name'] for r in collection(read_manifest(path))]
    column = (*column, set(before))
    visible = rows(session, column)
    report.note(f'{title}: visible first rows {[n["label"] for n in visible[:5]]}; stored {before[:5]}')
    source, target = visible[0], visible[2]
    report.check([n['label'] for n in visible[:3]] == before[:3], f'{title}: initial order')
    b = target['bounds']
    session.tools.drag_node(node=source['id'], to_x=b['x'] + b['width'] * 0.4,
                            to_y=b['y'] + b['height'] - 3)
    expected = before[1:3] + before[:1] + before[3:]
    report.check(wait(session, lambda: labels(session, column)[:3] == expected[:3]),
                 f'{title}: pointer drag changes order')
    history(session)
    report.check(wait(session, lambda: labels(session, column)[:3] == before[:3]), f'{title}: undo restores order')
    history(session, redo=True)
    report.check(wait(session, lambda: labels(session, column)[:3] == expected[:3]), f'{title}: Ctrl+Shift+Z redoes order')
    save(session)
    report.check([r['name'] for r in collection(read_manifest(path))] == expected, f'{title}: exact saved order')


def main():
    report = Report('collections')
    app = session = None
    try:
        path = fixture.working_copy(fixture.repo_path('qleany.yaml'), 'collections')
        data = read_manifest(path)
        use_case = named(named(data['features'], 'handling_manifest')['use_cases'], 'load')
        use_case['dto_in']['fields'] += [{'name': 'probe_alpha', 'type': 'string'},
                                        {'name': 'probe_beta', 'type': 'string'}]
        Path(path).write_text(yaml.safe_dump(data, sort_keys=False))
        env = fixture.isolated_config('collections')
        env['QLEANY_DEV'] = '1'
        app, session = launch_and_attach(argv=[fixture.app_binary()], cwd=os.path.dirname(path), env=env, label='collections')
        click(session, 'Open Qleany manifest')
        fixture.go_to(session, 'Entities')
        reorder(session, report, path, ENTITY, lambda m: m['entities'], 'entities')
        select(session, 'EntityBase', ENTITY)
        reorder(session, report, path, FIELD, lambda m: named(m['entities'], 'EntityBase')['fields'], 'fields')
        fixture.go_to(session, 'Features')
        reorder(session, report, path, FEATURE, lambda m: m['features'], 'features')
        select(session, 'handling_manifest', FEATURE)
        feature = lambda m: named(m['features'], 'handling_manifest')
        reorder(session, report, path, USE_CASE, lambda m: feature(m)['use_cases'], 'use cases')
        select(session, 'load', USE_CASE)
        click(session, 'Input DTO')
        load = lambda m: named(feature(m)['use_cases'], 'load')
        reorder(session, report, path, DTO, lambda m: load(m)['dto_in']['fields'], 'DTO fields')

        # Every overflow button below is opened by pointer, including the DTO row.
        before = load(read_manifest(path))['dto_in']['fields']
        menu_delete(session, 'probe_alpha', DTO)
        report.check(wait(session, lambda: 'probe_alpha' not in labels(session, DTO)), 'DTO field delete removes row')
        report.check(tree.find(session, label='Yes') is None, 'DTO field delete has no cascade confirmation')
        save(session)
        report.check(load(read_manifest(path))['dto_in']['fields'] == [r for r in before if r['name'] != 'probe_alpha'], 'DTO deletion saves the exact remaining fields')
        history(session)
        report.check(wait(session, lambda: 'probe_alpha' in labels(session, DTO)), 'undo restores DTO field')
        click(session, 'Output DTO')
        click(session, 'Enable DTO Out', 'CheckBox')
        click(session, 'Yes', 'Button')
        save(session)
        report.check(not load(read_manifest(path)).get('dto_out'), 'confirmed DTO disable removes output from YAML')
        report.check(bool(load(read_manifest(path)).get('dto_in')), 'disabling output preserves input')
        history(session)
        save(session)
        report.check(bool(load(read_manifest(path)).get('dto_out')), 'undo restores output DTO and its fields')

        for name, column, collection, title in [('load', USE_CASE, lambda m: feature(m)['use_cases'], 'use case'),
                                                ('handling_manifest', FEATURE, lambda m: m['features'], 'feature')]:
            before = collection(read_manifest(path))
            menu_delete(session, name, column, f"Delete {title}")
            report.check(any(name in str(n.get(key, '')) and 'DTOs' in str(n.get(key, '')) for n in tree.nodes(session) for key in ('label', 'value')), f'{title}: confirmation names object')
            click(session, 'No', 'Button')
            report.check(name in labels(session, column), f'{title}: cancel preserves row')
            menu_delete(session, name, column, f"Delete {title}")
            click(session, 'Yes', 'Button')
            report.check(wait(session, lambda: name not in labels(session, column)), f'{title}: accepted delete removes row')
            save(session)
            report.check(collection(read_manifest(path)) == [r for r in before if r['name'] != name], f'{title}: cascade persisted exactly')
            history(session)
            save(session)
            report.check(collection(read_manifest(path)) == before, f'{title}: undo restores entire subtree')
        fixture.go_to(session, 'Home')
        click(session, 'Close current manifest')
        click(session, 'Open Qleany manifest')
        fixture.go_to(session, 'Features')
        report.check(labels(session, FEATURE)[:3] == [r['name'] for r in read_manifest(path)['features'][:3]], 'reopen preserves feature order')
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
    return report.finish()


if __name__ == '__main__':
    sys.exit(main())
