"""Create a NEW source-guarded TCB repair + per-clip quarantine successor.

The predecessor cache remains immutable. Unchanged rigs copy byte-for-byte;
only actual serialized nested TCB tuples are added. Unsupported quaternion
clips are retained as evidence separately and omitted from playable rigs until
an actual native proof and strict preview check accept them.
"""
from __future__ import annotations
import argparse
import collections
import copy
import json
from pathlib import Path
import shutil

import export_browser as export


def raw_groups(controller):
    data = controller.data
    if isinstance(controller, export.NifFormat.NiKeyframeController):
        result = {'translation': (data.translations.keys, int(data.translations.interpolation)),
                  'scale': (data.scales.keys, int(data.scales.interpolation))}
        if int(data.rotation_type) == 4:
            result.update({f'euler.{i}': (axis.keys, int(axis.interpolation)) for i, axis in enumerate(data.xyz_rotations)})
        else:
            result['quaternion'] = (data.quaternion_keys, int(data.rotation_type))
        return result
    if isinstance(controller, export.NifFormat.NiAlphaController):
        return {'alpha': (data.data.keys, int(data.data.interpolation))}
    raise ValueError('unexpected TCB owner controller ' + type(controller).__name__)


def clip_channels(clip):
    for track in clip['tracks']:
        yield track, 'translation', track['translation']
        yield track, 'scale', track['scale']
        rotation = track['rotation']
        if rotation['kind'] == 'euler_xyz':
            for i, channel in enumerate(rotation['channels']):
                yield track, f'euler.{i}', channel
        else:
            yield track, 'quaternion', rotation['channel']
    for alpha in clip['alpha_tracks']:
        yield alpha, 'alpha', alpha['channel']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--predecessor', type=Path, default=export.HERE/'cache')
    parser.add_argument('--output', type=Path, default=export.HERE/'cache-v2')
    parser.add_argument('--quarantine', type=Path, default=export.HERE.parent/'review/unsupported-quaternion-clips.json')
    args = parser.parse_args()
    before, after = args.predecessor.resolve(), args.output.resolve()
    if after.exists():
        raise FileExistsError('successor output must be a NEW directory')
    if export.HERE.resolve() not in after.parents:
        raise ValueError('successor must stay in assigned export staging')
    manifest_path = before/'animation-browser-manifest.json'
    manifest = json.loads(manifest_path.read_text())
    original = copy.deepcopy(manifest)
    quarantine_document = json.loads(args.quarantine.read_text())
    if isinstance(quarantine_document, list):
        quarantines = quarantine_document
    else:
        # The independently produced checker receipt declares its clip rows.
        quarantines = quarantine_document.get('clips', quarantine_document.get('unsupported_clips'))
    if quarantines is None:
        raise ValueError('unrecognized explicit per-clip checker receipt')
    by_file = collections.defaultdict(dict)
    for row in quarantines:
        by_file[row['rig_file']][row['clip_index']] = row
    source_data = {}
    published, modified, removed, quarantined = [], [], [], []
    repairs = collections.Counter()
    for number, row in enumerate(original['rigs']):
        path = before/row['file']
        if export.sha(path) != row['sha256']:
            raise ValueError('predecessor rig hash changed: ' + row['file'])
        data = json.loads(path.read_text())
        changed = False
        for clip in data['clips']:
            for track, field, channel in clip_channels(clip):
                if channel['interpolation'] != 3 or not channel['keys']:
                    continue
                source = clip['source']
                if source not in source_data:
                    source_data[source] = export.base.read_nif(export.ORIGINAL/source)
                raw = source_data[source]
                controller = raw.blocks[track['source_block']]
                if type(controller).__name__ != track['controller']:
                    raise ValueError('raw controller class mismatch')
                keys, kind = raw_groups(controller)[field]
                if kind != 3 or len(keys) != len(channel['keys']):
                    raise ValueError('raw TCB kind/count mismatch')
                updated = export.channel(keys, kind)
                for old_key, new_key in zip(channel['keys'], updated['keys']):
                    if any(new_key.get(key) != value for key, value in old_key.items()):
                        raise ValueError('source key time/value/tangent mismatch before TCB repair')
                    if not all(key in new_key for key in ('tension','bias','continuity')):
                        raise ValueError('raw serialized TCB tuple absent')
                channel.clear()
                channel.update(updated)
                repairs[field] += 1
                repairs['keys'] += len(keys)
                changed = True
        bad = by_file.get(row['file'], {})
        for index, receipt in bad.items():
            clip = data['clips'][index]
            if (data['source'] != receipt['source'] or clip['source'] != receipt['clip_source']
                    or clip['name'] != receipt['clip_name']):
                raise ValueError('per-clip checker receipt does not match predecessor clip identity')
            evidence = {'model': data['source'], 'clip_index': index, 'source': clip['source'],
                        'source_sha256': clip['source_sha256'], 'name': clip['name'],
                        'reason': 'preview refuses unproved original quaternion interpolation; no default or curve reduction',
                        'checker_receipt': receipt}
            quarantined.append(evidence)
            manifest['skipped_clips'].append(evidence)
        if bad:
            export.write_new(after/'quarantined'/row['file'], data)
            data['clips'] = [clip for i, clip in enumerate(data['clips']) if i not in bad]
            row['clips'] = [clip for i, clip in enumerate(row['clips']) if i not in bad]
            changed = True
        if not data['clips']:
            reason = 'all source clips refused pending native quaternion interpolation proof; original source and explicit clip receipts retained'
            removed.append({'source': row['source'], 'reason': reason})
            manifest['rejected_models'].append(removed[-1])
            continue
        destination = after/row['file']
        if changed:
            export.write_new(destination, data)
            row['sha256'] = export.sha(destination)
            modified.append(row['file'])
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            # Exclusive creation, no hardlink or alias to the frozen predecessor.
            with path.open('rb') as source, destination.open('xb') as target:
                shutil.copyfileobj(source, target)
        published.append(row)
        if (number+1)%100 == 0:
            print(f'staged {number+1}/{len(original["rigs"])} predecessor rigs', flush=True)
    manifest['rigs'] = published
    referenced = {clip['source'] for rig in published for clip in rig['clips']}
    for row in manifest['sources']:
        row['in_exported_timeline'] = row['source'] in referenced
        row['status'] = 'exported_authored_timeline' if row['in_exported_timeline'] else 'source_catalog_only'
        if row['in_exported_timeline']:
            row['catalog_only_reason'] = None
        elif any(clip['source'] == row['source'] for clip in quarantined):
            row['catalog_only_reason'] = 'original quaternion interpolation is currently refused; explicit source/clip receipts retained, no invented fallback'
    manifest['counts'].update(rigs=len(published), clips=sum(len(r['clips']) for r in published),
                              distinct_clip_sources=len(referenced), rejected_models=len(manifest['rejected_models']),
                              skipped_clips=len(manifest['skipped_clips']))
    manifest['generator_sha256'] = export.sha(Path(export.__file__))
    manifest['inputs'][Path(export.__file__).relative_to(export.WORKSPACE).as_posix()] = export.sha(Path(export.__file__))
    manifest['staging_successor'] = {'predecessor_manifest_sha256': export.sha(manifest_path),
                                    'repair_generator_sha256': export.sha(Path(__file__)),
                                    'explicit_quarantine_receipt_sha256': export.sha(args.quarantine),
                                    'actual_raw_tcb_repair_channels': dict(repairs), 'modified_published_files': modified,
                                    'removed_rigs': removed, 'quarantined_clips': quarantined,
                                    'scope': 'Actual serialized TCB tuple repair with every original key time/value/tangent guarded; explicit per-clip quaternion refusal. Unchanged rig bytes copied exactly; predecessor immutable.'}
    export.write_new(after/'animation-browser-manifest.json', manifest)
    print(json.dumps({'manifest': str(after/'animation-browser-manifest.json'), 'sha256': export.sha(after/'animation-browser-manifest.json'),
                      'counts': manifest['counts'], 'repairs': dict(repairs), 'quarantined_clips': len(quarantined)}), flush=True)


if __name__ == '__main__':
    main()
