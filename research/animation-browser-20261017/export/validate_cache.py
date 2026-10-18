"""Read-only whole-cache source/topology/closure audit; new report only."""
from __future__ import annotations
import argparse
import collections
import hashlib
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
WORKSPACE = HERE.parents[2]
ORIGINAL = WORKSPACE.parent / 'reconstruction/assets/original/Data'
CONVERTED = WORKSPACE.parent / 'reconstruction/assets/converted/models'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=Path, default=HERE / 'cache')
    parser.add_argument('--output', type=Path, default=HERE / 'validation01.json')
    args = parser.parse_args()
    cache = args.cache.resolve()
    manifest_path = cache / 'animation-browser-manifest.json'
    manifest = read(manifest_path)
    inventory_path = WORKSPACE / 'research/animation-browser-20261017/inventory/original-source-catalog.json'
    inventory = read(inventory_path)
    expected = {row['source']: row for row in inventory['assets']}
    material_path = WORKSPACE / 'assets-cache/model-materials.json'
    materials = read(material_path)['models']
    errors = []
    checks = collections.Counter()

    def require(condition, message):
        if not condition:
            errors.append(message)

    require(manifest['complete_source_catalog'] and not manifest['export_filter'], 'catalog must be complete and unfiltered')
    rows = {row['source']: row for row in manifest['sources']}
    require(len(rows) == len(manifest['sources']) == 2481, 'source rows must be unique2481')
    require(set(rows) == set(expected), 'source catalog domain differs from archive-pinned inventory')
    for source, row in expected.items():
        actual = sha(ORIGINAL / source)
        require(actual == row['sha256'] == rows[source]['sha256'], 'original source hash: ' + source)
        require(dict(collections.Counter(c['class'] for c in row['controllers'])) == rows[source]['controller_counts'], 'controller census: ' + source)
        checks['original_source_hashes'] += 1
    all_references, curve_types, euler_orders = set(), collections.Counter(), collections.Counter()
    rich_scalars, unsupported = collections.Counter(), collections.Counter()
    rig_sources = set()
    totals = collections.Counter()
    failures = []
    for number, rig in enumerate(manifest['rigs']):
        source = rig['source']
        require(source not in rig_sources, 'duplicate model family: ' + source)
        rig_sources.add(source)
        relative = Path(rig['file'])
        path = (cache / relative).resolve()
        require(not relative.is_absolute() and cache in path.parents and relative.parts[0] == 'rigs', 'rig path escapes closure: ' + rig['file'])
        require(sha(path) == rig['sha256'], 'rig hash: ' + source)
        data = read(path)
        require(data['source'] == source and data['source_sha256'] == rig['source_sha256'] == expected[source]['sha256'], 'rig source identity: ' + source)
        static_path = CONVERTED / Path(source).with_suffix('.json')
        require(sha(static_path) == data['static_model_sha256'] == materials[source]['converted_sha256'], 'static model identity: ' + source)
        static = read(static_path)
        retained = [entry['mesh_index'] for entry in materials[source]['corrections'] if not entry['skip']]
        require([mesh['source_mesh_index'] for mesh in data['meshes']] == retained, 'material-audited retained mesh ordering: ' + source)
        for mesh in data['meshes']:
            original_mesh = static['meshes'][mesh['source_mesh_index']]
            require(mesh['name'] == original_mesh['name'] and mesh['indices'] == original_mesh['indices'] and len(mesh['positions'])*3 == len(original_mesh['positions']), 'static topology/vertex identity: ' + source + '/' + mesh['name'])
            require(0 <= mesh['node'] < len(data['nodes']), 'mesh node: ' + source)
            require(all(0 <= i < len(mesh['positions']) for i in mesh['indices']), 'index bounds: ' + source)
            checks['mesh_topology'] += 1
            totals['vertices'] += len(mesh['positions'])
            totals['triangles'] += len(mesh['indices']) // 3
        for index, node in enumerate(data['nodes']):
            require(node['parent'] is None or node['parent'] < index, 'hierarchy parent order: ' + source)
        require(len(data['clips']) == len(rig['clips']), 'clip count: ' + source)
        for clip, record in zip(data['clips'], rig['clips']):
            all_references.add(clip['source'])
            require(clip['source_sha256'] == expected[clip['source']]['sha256'] == record['source_sha256'], 'clip original source identity: ' + source + '/' + clip['source'])
            require(clip['name'] == record['name'] and clip['duration'] == record['duration'], 'clip manifest identity: ' + source)
            require(clip['duration'] > 0 and math.isfinite(clip['duration']), 'invalid clip duration: ' + source)
            require(len({t['node'] for t in clip['tracks']}) == len(clip['tracks']), 'duplicate transform binding: ' + source)
            for track in clip['tracks']:
                require(data['nodes'][track['node']]['name'] == track['target'], 'track target index/name identity: ' + source)
                channels = [('translation', track['translation']), ('scale', track['scale'])]
                rotation = track['rotation']
                if rotation['kind'] == 'euler_xyz':
                    euler_orders[rotation['order']] += 1
                    channels += [('euler', axis) for axis in rotation['channels']]
                else:
                    channels += [('quaternion', rotation['channel'])]
                for field, channel in channels:
                    if channel['keys']:
                        curve_types[(field, channel['interpolation'])] += 1
                        if channel['interpolation'] in (2,3):
                            rich_scalars['keys_' + str(channel['interpolation'])] += len(channel['keys'])
                    for left, right in zip(channel['keys'], channel['keys'][1:]):
                        require(left['time'] < right['time'], 'non-increasing source key time: ' + source)
            for alpha in clip['alpha_tracks']:
                require(alpha['meshes'] and all(0 <= mesh < len(data['meshes']) for mesh in alpha['meshes']), 'alpha material target mesh bounds: ' + source)
                curve_types[('alpha', alpha['channel']['interpolation'])] += 1
            unsupported.update(c['controller'] for c in clip['unsupported_controllers'])
            totals['clips'] += 1
            checks['clip_source_and_binding'] += 1
        totals['nodes'] += len(data['nodes'])
        totals['meshes'] += len(data['meshes'])
        totals['rig_bytes'] += path.stat().st_size
        checks['rig_hashes'] += 1
        if (number+1) % 100 == 0:
            print(f'validated {number+1}/{len(manifest["rigs"])} rigs', flush=True)
    require(all_references == {r['source'] for r in manifest['sources'] if r['in_exported_timeline']}, 'source timeline status differs from actual clip references')
    require({str(p.relative_to(cache)).replace('\\','/') for p in (cache/'rigs').rglob('*.rig.json')} == {r['file'] for r in manifest['rigs']}, 'staging has unlisted/absent rigs')
    for name, relative in manifest['inputs'].items():
        require(sha(WORKSPACE/name) == relative, 'frozen exporter input drift: ' + name)
    # Old published cache bytes must still match their own immutable receipts.
    old_root = WORKSPACE / 'assets-cache/animation'
    old_records = []
    for record in read(old_root/'manifest.json')['assets']:
        old_records += [(record['file'], record['sha256']), (record['reference'], record['reference_sha256'])]
    old_records += [(record['file'], record['sha256']) for record in read(old_root/'promoted-families.json')['artifacts']]
    old_records += [(record['file'], record['sha256']) for record in read(old_root/'game-rigs.json')['rigs']]
    for file, digest in old_records:
        require(sha(old_root/file) == digest, 'legacy cache receipt changed: ' + file)
        checks['legacy_cache_receipts'] += 1
    hq_path = WORKSPACE / 'research/animation-browser-20261017/inventory/hq-building-deaths.json'
    hq = read(hq_path)
    categories = {}
    for kind in ('hq_death', 'corps_death', 'all_building_death'):
        sources = {r['source'] for r in hq[kind]}
        categories[kind] = {'source_count': len(sources), 'exported_count': len(sources & rig_sources),
                            'missing': sorted(sources-rig_sources)}
    pack = {s for s in expected if s.startswith('Buildings/') and Path(s).suffix.lower()=='.nif' and Path(s).stem.casefold().endswith(('pack','unpack'))}
    categories['building_pack_unpack'] = {'source_count': len(pack), 'exported_count': len(pack & rig_sources), 'missing': sorted(pack-rig_sources)}
    document = {'passed': not errors, 'scope': 'Whole staged source hash/topology/closure audit and legacy receipt preservation; source-export metadata, not Rust sampling or GPU acceptance.',
                'manifest': str(manifest_path), 'manifest_sha256': sha(manifest_path),
                'inventory_sha256': sha(inventory_path), 'validator_sha256': sha(Path(__file__)),
                'checks': dict(checks), 'totals': dict(totals), 'coverage': categories,
                'curve_types': [{'field': key[0], 'interpolation': key[1], 'channels': count} for key,count in sorted(curve_types.items())],
                'euler_orders': dict(euler_orders), 'rich_key_counts': dict(rich_scalars),
                'unsupported_effect_controller_instances': dict(unsupported),
                'errors': errors, 'evidence_limits': ['Original source files are byte-verified against archive-pinned SDK census; no original source is modified.',
                    'Every exported mesh agrees with independently frozen converted topology and native-audited retained material mesh ordering.',
                    'Raw-channel correctness is additionally exercised by actual HQ tests and native primitive fixtures; this check does not independently execute every original controller.',
                    'Reference geometry/Rust whole-data sampling and real GUI/GPU checks have separate receipts.']}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf-8') as stream:
        json.dump(document, stream, indent=2)
        stream.write('\n')
    print(json.dumps({'passed': document['passed'], 'errors': len(errors), 'checks': document['checks'], 'coverage': categories, 'report_sha256': sha(args.output)}), flush=True)
    raise SystemExit(0 if document['passed'] else 1)


if __name__ == '__main__':
    main()
