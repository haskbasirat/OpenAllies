"""Compare bounded native restoration censuses without calling them game completion."""
from pathlib import Path
import argparse
import hashlib
import json


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summary(report):
    return {
        'maps': report['map_count'],
        'native_load_mode': report.get('native_load_mode'),
        'maps_without_record_failures': report['maps_without_failures'],
        'record_failures': sum(group['count'] for group in report['groups']),
        'converted_records_awaiting_replacement': report['converted_records'],
        'native_skips': report['skipped_records_by_cause'],
        'post_load_passes': sum(len(row['maps']) for row in report['post_load_by_outcome'] if row['outcome'] == 'ok'),
        'groups': [{'count': row['count'], 'error': row['normalized_error']} for row in report['groups']],
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--before', required=True, type=Path)
    parser.add_argument('--after', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    before, after = read(args.before), read(args.after)
    assert before['precision'] == after['precision'] == 'Double53'
    assert before['mode'] == after['mode'] == 'InspectPartial'
    assert before['map_count'] == after['map_count'] == 80
    prior = {row['map']: row for row in before['maps']}
    current = {row['map']: row for row in after['maps']}
    assert prior.keys() == current.keys()
    changes = []
    for name, row in current.items():
        old = prior[name]
        if old['failures'] != row['failures'] or old['post_load'] != row['post_load']:
            changes.append({
                'map': name, 'record_failures_before': len(old['failures']),
                'record_failures_after': len(row['failures']),
                'first_failure_before': old['failures'][:1],
                'first_failure_after': row['failures'][:1],
                'post_load_before': old['post_load'], 'post_load_after': row['post_load'],
            })
    result = {
        'scope': 'Original map Load/PostLoad diagnostic; not playable-match acceptance or a game-completion percentage',
        'before_source': {'path': str(args.before.resolve()), 'sha256': sha(args.before)},
        'after_source': {'path': str(args.after.resolve()), 'sha256': sha(args.after)},
        'before': summary(before), 'after': summary(after), 'map_changes': changes,
        'caveats': [
            'InspectPartial continues after errors and later failures can be cascades.',
            'Fixing an early stop can expose additional nested records or later failures.',
            'Compare native_load_mode explicitly; an unspecified baseline mode is not proof of the same session input.',
            'A native warning/drop is counted separately from an unsupported operation.',
            'Class conversions still require the native replacement tail.',
            'No runtime publication, simulation tick, battle, or multiplayer acceptance is implied.',
        ],
    }
    with args.output.open('x', encoding='utf-8') as stream:
        json.dump(result, stream, indent=2)
    print(json.dumps({'before': result['before'], 'after': result['after']}))


if __name__ == '__main__':
    main()
