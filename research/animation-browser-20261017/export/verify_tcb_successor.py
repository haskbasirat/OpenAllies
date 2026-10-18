"""Independent raw-field and exact predecessor-delta audit for cache-v2-final."""
from __future__ import annotations
import copy
import json
from pathlib import Path

import export_browser as export

HERE = export.HERE


def channels(clip):
    for track in clip['tracks']:
        yield track, 'translation', track['translation']
        yield track, 'scale', track['scale']
        rotation = track['rotation']
        if rotation['kind'] == 'euler_xyz':
            for i, value in enumerate(rotation['channels']):
                yield track, f'euler.{i}', value
        else:
            yield track, 'quaternion', rotation['channel']
    for alpha in clip['alpha_tracks']:
        yield alpha, 'alpha', alpha['channel']


def main():
    before = HERE/'cache'
    after = HERE/'cache-v2-final'
    old = json.loads((before/'animation-browser-manifest.json').read_text())
    new = json.loads((after/'animation-browser-manifest.json').read_text())
    change = new['staging_successor']
    rejects = {}
    for row in change['quarantined_clips']:
        rejects.setdefault(row['checker_receipt']['rig_file'], set()).add(row['clip_index'])
    old_rows = {row['file']: row for row in old['rigs']}
    new_rows = {row['file']: row for row in new['rigs']}
    changed = set(change['modified_published_files']) | set(rejects)
    raw_cache = {}
    tuples, keys, unchanged = 0, 0, 0
    errors = []
    for file, old_row in old_rows.items():
        if export.sha(before/file) != old_row['sha256']:
            errors.append('immutable predecessor rig changed: ' + file)
        if file not in changed:
            if export.sha(after/file) != old_row['sha256']:
                errors.append('unmodified successor differs: ' + file)
            unchanged += 1
            continue
        original = json.loads((before/file).read_text())
        quarantine_path = after/'quarantined'/file
        repaired = json.loads((quarantine_path if file in rejects else after/file).read_text())
        stripped = copy.deepcopy(repaired)
        for ci, clip in enumerate(repaired['clips']):
            for (track, field, channel), (_, _, remove_channel) in zip(channels(clip), channels(stripped['clips'][ci])):
                if channel['interpolation'] != 3 or not channel['keys']:
                    continue
                source = clip['source']
                if source not in raw_cache:
                    if export.sha(export.ORIGINAL/source) != clip['source_sha256']:
                        errors.append('raw TCB source hash: ' + source)
                    raw_cache[source] = export.base.read_nif(export.ORIGINAL/source)
                controller = raw_cache[source].blocks[track['source_block']]
                data = controller.data
                if field == 'translation':
                    raw_keys, kind = data.translations.keys, int(data.translations.interpolation)
                elif field == 'scale':
                    raw_keys, kind = data.scales.keys, int(data.scales.interpolation)
                elif field == 'quaternion':
                    raw_keys, kind = data.quaternion_keys, int(data.rotation_type)
                elif field.startswith('euler.'):
                    group = data.xyz_rotations[int(field[-1])]
                    raw_keys, kind = group.keys, int(group.interpolation)
                else:
                    raw_keys, kind = data.data.keys, int(data.data.interpolation)
                if kind != 3 or len(raw_keys) != len(channel['keys']):
                    errors.append('raw TCB kind/count: ' + source)
                for raw, key, stripped_key in zip(raw_keys, channel['keys'], remove_channel['keys']):
                    expected = (float(raw.tbc.t), float(raw.tbc.b), float(raw.tbc.c))
                    if (key['tension'], key['bias'], key['continuity']) != expected:
                        errors.append('raw TCB serialized tuple mismatch: ' + source)
                    if float(raw.time) != key['time'] or export.base.value(raw.value) != key['value']:
                        errors.append('raw TCB key time/value mismatch: ' + source)
                    for name in ('tension','bias','continuity'):
                        stripped_key.pop(name)
                    keys += 1
                tuples += 1
        if stripped != original:
            errors.append('repair changed fields beyond the exact serialized TCB tuples: ' + file)
        if file in new_rows:
            actual = json.loads((after/file).read_text())
            expected = copy.deepcopy(repaired)
            expected['clips'] = [clip for i, clip in enumerate(repaired['clips']) if i not in rejects.get(file,set())]
            if actual != expected:
                errors.append('published successor is not exact repaired original minus explicit quarantined clips: ' + file)
        elif file not in rejects:
            errors.append('removed whole rig without per-clip rejection: ' + file)
    if keys != 392 or tuples != 91:
        errors.append('expected actual raw TCB field census differs')
    animated = [row for row in new['sources'] if row['kind']=='kf' or row['has_embedded_controllers']]
    supported = [row for row in animated if row['in_exported_timeline']]
    document = {'passed': not errors, 'scope': 'Exact whole predecessor delta and every repaired raw serialized TCB tuple; no native quaternion support claim.',
                'manifest_sha256': export.sha(after/'animation-browser-manifest.json'), 'validator_sha256': export.sha(Path(__file__)),
                'unchanged_rig_bytes': unchanged, 'changed_or_quarantined_rigs': len(changed),
                'actual_raw_tcb_channels': tuples, 'actual_raw_tcb_keys': keys, 'raw_source_files': len(raw_cache),
                'quarantined_clips': len(change['quarantined_clips']), 'published_rigs': len(new['rigs']),
                'published_clips': sum(len(r['clips']) for r in new['rigs']),
                'animated_source_catalog': len(animated), 'sources_with_exported_timeline': len(supported),
                'unavailable_animated_sources': len(animated)-len(supported),
                'available_by_kind': {kind: sum(r['kind']==kind for r in supported) for kind in ('nif','kf')},
                'unavailable_by_kind': {kind: sum(r['kind']==kind and not r['in_exported_timeline'] for r in animated) for kind in ('nif','kf')},
                'errors': errors}
    export.write_new(HERE/'tcb-successor-validation.json', document)
    print(json.dumps(document), flush=True)
    raise SystemExit(0 if not errors else 1)


if __name__ == '__main__':
    main()
