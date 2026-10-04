#!/usr/bin/env python3
"""Persist real editor option changes, including relationship and DTO flag combinations."""
import os
import sys
import automation_fixture as fixture
import automation_entities as entities
from teksilo_probe import Report, launch_and_attach, navigate, tree
from ui_probe_helpers import click, history, named, read_manifest, save, select


def main():
    report = Report('options')
    app = session = None
    try:
        path = fixture.working_copy(fixture.repo_path('qleany.yaml'), 'options')
        env = fixture.isolated_config('options')
        env['QLEANY_DEV'] = '1'
        app, session = launch_and_attach(argv=[fixture.app_binary()], cwd=os.path.dirname(path), env=env, label='options')
        click(session, 'Open Qleany manifest')
        fixture.go_to(session, 'Entities')
        select(session, 'Global', (0, 430))
        for value in ('None', 'EntityBase'):
            report.check(entities.set_combo(session, 'Inherits from', value), f'select inheritance {value}')
            save(session)
            report.check(named(read_manifest(path)['entities'], 'Global').get('inherits_from') == (None if value == 'None' else value), f'inheritance {value} persists')
        for label, key in [('Single model', 'single_model'), ('Undoable', 'undoable')]:
            before = named(read_manifest(path)['entities'], 'Global').get(key, False)
            click(session, label, 'CheckBox')
            save(session)
            report.check(named(read_manifest(path)['entities'], 'Global').get(key, False) == (not before), f'{label} persists')
            history(session)
            save(session)
            report.check(named(read_manifest(path)['entities'], 'Global').get(key, False) == before, f'{label} undo persists')
        select(session, 'Root', (0, 430))
        select(session, 'workspace', (430, 900))
        field = lambda: named(named(read_manifest(path)['entities'], 'Root')['fields'], 'workspace')
        for relationship in ('one_to_many', 'ordered_one_to_many', 'many_to_many', 'one_to_one'):
            report.check(entities.set_combo(session, 'Relationship type', relationship), f'select {relationship}')
            save(session)
            result = field()
            report.check(result.get('relationship') == relationship, f'{relationship} persists')
            many = relationship in ('one_to_many', 'ordered_one_to_many', 'many_to_many')
            report.check((tree.find(session, label='Optional', role='CheckBox') is None) == many, f'{relationship}: Optional visibility')
            if many:
                report.check(not result.get('optional', False), f'{relationship}: optional cleared')
                click(session, 'List model', 'CheckBox')
                save(session)
                report.check(field().get('list_model', False) == (not result.get('list_model', False)), f'{relationship}: list model persists')
            else:
                click(session, 'Optional', 'CheckBox')
                save(session)
                report.check(field().get('optional', False) == (not result.get('optional', False)), f'{relationship}: Optional persists')
                report.check(not field().get('list_model', False), f'{relationship}: list model cleared')
            if relationship in ('many_to_one', 'many_to_many'):
                report.check(not field().get('strong', False), f'{relationship}: cascade cleared')
            else:
                before = field().get('strong', False)
                click(session, 'Strong (cascade delete)', 'CheckBox')
                save(session)
                report.check(field().get('strong', False) == (not before), f'{relationship}: cascade persists')

        fixture.go_to(session, 'Features')
        select(session, 'handling_manifest', (0, 393))
        select(session, 'load', (393, 654))
        use_case = lambda: named(named(read_manifest(path)['features'], 'handling_manifest')['use_cases'], 'load')
        for label, key in [('Long operation', 'long_operation'), ('Undoable', 'undoable')]:
            before = use_case().get(key, False)
            click(session, label, 'CheckBox')
            save(session)
            report.check(use_case().get(key, False) == (not before), f'use case {label} persists')
        # Activate the checkbox inside the entity association row.
        node = tree.wait_for_node(session, label='Workspace', role='ListBoxOption', timeout=5)
        before = set(use_case().get('entities', []))
        tick = next(n for n in tree.descendants(session, node) if n.get("role") == "CheckBox")
        navigate.click(session, tick)
        save(session)
        report.check(set(use_case().get('entities', [])) == before.symmetric_difference({'Workspace'}), 'association change persists exactly')
        history(session)
        save(session)
        report.check(set(use_case().get('entities', [])) == before, 'association undo restores exact set')
        click(session, 'Input DTO')
        select(session, 'manifest_path', (654, 1400))
        dto_field = lambda: named(use_case()['dto_in']['fields'], 'manifest_path')
        for label, key in [('Optional', 'optional'), ('List', 'is_list')]:
            before = dto_field().get(key, False)
            click(session, label, 'CheckBox')
            save(session)
            report.check(dto_field().get(key, False) == (not before), f'DTO {label} persists')
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
