"""Source-complete original-animation browser export, in a NEW staging tree.

The existing rig/material exporters provide geometry and skin identity only.
This exporter preserves authored curve types/tangents and embedded controller
target identity. It never invents a KF association, retargets by basename, bakes
approximate motion, or writes the old cache/release. Unsupported effect
controllers remain visible metadata beside the supported geometry timeline.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import math
import os
from pathlib import Path
import re
import struct
import sys
import time

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
WORKSPACE = HERE.parents[2]
sys.path.insert(0, str(WORKSPACE / 'crates/aa-animation/tools'))
import export_original as base
import export_families as geometry
from convert_models import Converter, NifFormat, ORIGINAL, string

sha = base.sha
SOURCE_INVENTORY = WORKSPACE / 'research/animation-browser-20261017/inventory/original-source-catalog.json'
ASSOCIATIONS = WORKSPACE / 'research/phase3-animation/animation-catalog.json'
RECOVERED = base.RECON / 'game/data/catalog.json'
MATERIALS = WORKSPACE / 'assets-cache/model-materials.json'


def write_new(path: Path, document):
    """Exclusive creation: reruns cannot silently replace any staged evidence."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(document, stream, separators=(',', ':'), allow_nan=False)
        stream.write('\n')


def channel(group, kind):
    result = []
    for key in group:
        row = {'time': float(key.time), 'value': base.value(key.value)}
        for field in ('forward', 'backward'):
            if hasattr(key, field):
                row[field] = base.value(getattr(key, field))
        for field in ('tension', 'bias', 'continuity'):
            if hasattr(key, field):
                row[field] = float(getattr(key, field))
        result.append(row)
    return {'interpolation': int(kind), 'keys': result}


def controller_fields(controller, block_index):
    return {'controller': type(controller).__name__, 'source_block': block_index,
            'frequency': float(controller.frequency), 'phase': float(controller.phase),
            'start': float(controller.start_time), 'stop': float(controller.stop_time),
            'flags': int(controller.flags)}


def transform_track(controller, node, nodes, block_index):
    data = controller.data
    if not isinstance(data, NifFormat.NiKeyframeData):
        raise ValueError('transform controller has ' + type(data).__name__)
    row = controller_fields(controller, block_index)
    row.update(node=node, target=nodes[node]['name'],
               translation=channel(data.translations.keys, data.translations.interpolation),
               scale=channel(data.scales.keys, data.scales.interpolation))
    if int(data.rotation_type) == 4:
        # This serialized word is misnamed Unknown Float by old PyFFI XML.
        # Preserve its exact bits: AA reads it as the Euler axis-order DWORD.
        order = struct.unpack('<I', struct.pack('<f', float(data.unknown_float)))[0]
        row['rotation'] = {'kind': 'euler_xyz', 'order': order, 'channels': [
            channel(axis.keys, axis.interpolation) for axis in data.xyz_rotations]}
    else:
        row['rotation'] = {'kind': 'quaternion', 'channel': channel(data.quaternion_keys, data.rotation_type)}
    return row


def actual_materials(objects):
    """The exact inherited material-property object used for each retained node."""
    materials = {}
    for obj in objects:
        current = obj
        while current is not None:
            found = [p for p in getattr(current, 'properties', [])
                     if isinstance(p, NifFormat.NiMaterialProperty)]
            if found:
                materials[id(obj)] = found[-1]
                break
            current = getattr(current, '_parent', None)
    return materials


def material_mesh_map(nodes, objects, meshes):
    # collect_model records parent indices; do not rely on PyFFI _parent aliases.
    inherited = []
    for index, (node, obj) in enumerate(zip(nodes, objects)):
        material = inherited[node['parent']] if node['parent'] is not None else None
        for prop in getattr(obj, 'properties', []):
            if isinstance(prop, NifFormat.NiMaterialProperty):
                material = prop
        inherited.append(material)
    mapped = collections.defaultdict(list)
    for index, mesh in enumerate(meshes):
        prop = inherited[mesh['node']]
        if prop is not None:
            mapped[id(prop)].append(index)
    return mapped


def make_clip(source, data, nodes, objects, meshes, sequence=None):
    block_indices = {id(block): i for i, block in enumerate(data.blocks)}
    node_indices = {id(obj): i for i, obj in enumerate(objects)}
    material_meshes = material_mesh_map(nodes, objects, meshes)
    names = collections.defaultdict(list)
    for index, node in enumerate(nodes):
        names[node['name']].append(index)
    if sequence is None:
        entries = [(c, None) for c in data.blocks if isinstance(c, NifFormat.NiTimeController)]
    else:
        entries = [(entry.controller, string(entry.target_name))
                   for entry in sequence.controlled_blocks if entry.controller is not None]
    tracks, alpha, unsupported = [], [], []
    used = set()
    for controller, name in entries:
        fields = controller_fields(controller, block_indices.get(id(controller)))
        target = getattr(controller, 'target', None)
        fields['target'] = name if name is not None else string(getattr(target, 'name', b''))
        try:
            if isinstance(controller, NifFormat.NiKeyframeController):
                if sequence is None:
                    if id(target) not in node_indices:
                        raise ValueError('controller target is not in retained hierarchy/selected LOD')
                    index = node_indices[id(target)]
                else:
                    matches = names.get(name, [])
                    if len(matches) != 1:
                        raise ValueError('KF target unresolved or ambiguous in this exact model: ' + name)
                    index = matches[0]
                if index in used:
                    raise ValueError('multiple transform controllers target the same retained node')
                row = transform_track(controller, index, nodes, block_indices.get(id(controller)))
                tracks.append(row)
                used.add(index)
            elif isinstance(controller, NifFormat.NiAlphaController):
                indices = material_meshes.get(id(target), [])
                if not indices:
                    raise ValueError('alpha target has no retained triangle mesh (effect/particle or excluded geometry)')
                if not isinstance(controller.data, NifFormat.NiFloatData):
                    raise ValueError('alpha controller has ' + type(controller.data).__name__)
                group = controller.data.data
                alpha.append({**fields, 'meshes': indices, 'channel': channel(group.keys, group.interpolation)})
            else:
                raise ValueError('controller effect is not implemented by the isolated geometry preview')
        except (ValueError, KeyError, IndexError) as error:
            unsupported.append({**fields, 'data_class': type(getattr(controller, 'data', None)).__name__,
                                'reason': str(error)})
            # Binding errors affect the entire external sequence, never silently
            # leave half a KF pose on the wrong model. Embedded unsupported FX
            # are separately retained metadata and do not remove valid tracks.
            if sequence is not None and isinstance(controller, NifFormat.NiKeyframeController):
                raise ValueError(str(error)) from error
    relevant = tracks + alpha
    # Some effect/static controllers serialize FLT_MAX extrema. A browser
    # timeline comes from authored populated key/event ranges, not those
    # initialization sentinels; every original controller range stays above.
    times = []
    for row in relevant:
        channels = [row['translation'], row['scale']] if 'translation' in row else [row['channel']]
        if 'rotation' in row:
            rotation = row['rotation']
            channels += rotation.get('channels', [rotation.get('channel')])
        times += [key['time'] for c in channels if c for key in c['keys']]
    events = []
    if sequence is not None and sequence.text_keys:
        events = [{'time': float(k.time), 'text': string(k.value)} for k in sequence.text_keys.text_keys]
    elif sequence is None:
        for block in data.blocks:
            if isinstance(block, NifFormat.NiTextKeyExtraData):
                events += [{'time': float(k.time), 'text': string(k.value)} for k in block.text_keys]
    duration = max(times + [e['time'] for e in events] + [0.0])
    if not relevant or duration <= 0:
        raise ValueError('no positive-duration supported transform/alpha timeline; controllers retained in source catalog')
    return {'name': string(sequence.name) if sequence is not None else Path(source).stem,
            'source': source, 'source_sha256': sha(ORIGINAL / source),
            'source_kind': 'external_kf' if sequence is not None else 'embedded_nif',
            'duration': duration, 'cycle_type': int(sequence.cycle_type)
            if sequence is not None and data.version >= 0x0A01006A else None,
            'tracks': tracks, 'alpha_tracks': alpha, 'events': events,
            'unsupported_controllers': unsupported}


def candidate_pairs(canonical):
    document = json.loads(ASSOCIATIONS.read_text(encoding='utf-8'))
    recovered = json.loads(RECOVERED.read_text(encoding='utf-8'))
    by_model = collections.defaultdict(lambda: collections.defaultdict(list))
    unbound = []
    for row in document['projected_associations']:
        model = row['model']
        basis = 'original TGI recovered aspect projection; runtime state selection not asserted'
        if model is None and row['scope'] == 'CharacterComponent.Animation':
            thing = recovered[row['category']][row['actor']]
            view = thing.get('data', {}).get('View', {})
            normal = view.get('AspectNormal', {}) if isinstance(view, dict) else {}
            if isinstance(normal, dict):
                prefix = view.get('resource_prefix')
                fmt = normal.get('model')
                if isinstance(prefix, str) and isinstance(fmt, str):
                    expanded = fmt.replace('%s', prefix + (normal.get('aspect_prefix') or ''))
                    path = expanded.lstrip('/') if expanded.startswith('/') else (Path(thing['source']).parent / expanded).as_posix()
                    model = canonical.get(path.casefold())
                    basis = 'default Character list + same actor authored AspectNormal candidate; exact KF targets must bind'
        model = canonical.get(model.casefold()) if isinstance(model, str) else None
        kf = canonical.get(row['kf'].casefold()) if isinstance(row['kf'], str) else None
        receipt = {**row, 'association_basis': basis}
        if model and kf:
            by_model[model][kf].append(receipt)
        else:
            unbound.append(receipt)
    # Include every existing exact model/KF source binding, without treating
    # role aliases or the six-role game subset as complete authored lists.
    old_rigs = list((WORKSPACE / 'assets-cache/animation').glob('*.rig.json'))
    old_rigs += list((WORKSPACE / 'assets-cache/animation/game').glob('*.rig.json'))
    for path in sorted(old_rigs):
        old = json.loads(path.read_text(encoding='utf-8'))
        model = canonical.get(old['source'].casefold())
        for clip in old['clips']:
            kf = canonical.get(clip['source'].casefold())
            if model and kf:
                by_model[model][kf].append({'association_basis': 'existing source-hashed original rig binding',
                                           'previous_cache': path.relative_to(WORKSPACE).as_posix()})
    return by_model, unbound


def export_model(source, inventory, bindings, converter, destination):
    path = ORIGINAL / source
    if sha(path) != inventory['sha256']:
        raise ValueError('original source differs from archive-pinned inventory')
    data, nodes, meshes, objects, names, excluded = geometry.collect_model(path, converter)
    if not meshes:
        raise ValueError('no native-audited retained triangle meshes')
    clips, skipped = [], []
    if inventory.get('controllers'):
        try:
            clips.append(make_clip(source, data, nodes, objects, meshes))
        except Exception as error:
            skipped.append({'source': source, 'source_kind': 'embedded_nif', 'reason': str(error)})
    for kf, associations in sorted(bindings.items()):
        try:
            clip_data = base.read_nif(ORIGINAL / kf)
            sequences = [r for r in clip_data.roots if isinstance(r, NifFormat.NiControllerSequence)]
            if not sequences:
                raise ValueError('no NiControllerSequence root in KF')
            for sequence_index, sequence in enumerate(sequences):
                clip = make_clip(kf, clip_data, nodes, objects, meshes, sequence)
                clip['source_sequence_index'] = sequence_index
                clip['associations'] = associations
                clips.append(clip)
        except Exception as error:
            skipped.append({'source': kf, 'source_kind': 'external_kf', 'reason': str(error), 'associations': associations})
    if not clips:
        return None, skipped
    relative = Path('rigs') / Path(source).with_suffix('.rig.json')
    result = {'format': 'aa-original-rig-v1', 'source': source, 'source_sha256': sha(path),
              'static_model_sha256': sha(base.RECON / 'assets/converted/models' / Path(source).with_suffix('.json')),
              'coordinates': 'native Z-up local data; output(x,z,-y); browser-oriented UVs',
              'nif_version': '.'.join(str((data.version >> n) & 255) for n in (24, 16, 8, 0)),
              'nodes': nodes, 'meshes': [{k: v for k, v in mesh.items() if k != '_object'} for mesh in meshes],
              'clips': clips, 'excluded_geometry': excluded,
              'limitations': ['Original authored geometry/curve inspection only; no battlefield state machine, movement, retiming, event execution or blending.',
                              'Controller classes listed in unsupported_controllers are not simulated; retained geometry timelines may omit particle/effect behavior.',
                              'Source fields are preserved exactly as float32-decoded values; rich preview math is independently validated, not a claim of native Gamebryo execution parity.']}
    write_new(destination / relative, result)
    return {'name': str(Path(source).with_suffix('')).replace('\\', '/'),
            'aliases': [Path(source).stem], 'limitations': result['limitations'], 'source': source,
            'source_sha256': result['source_sha256'], 'file': relative.as_posix(), 'sha256': sha(destination / relative),
            'nodes': len(nodes), 'meshes': len(meshes), 'vertices': sum(len(m['positions']) for m in meshes),
            'clips': [{'name': c['name'], 'source': c['source'], 'source_sha256': c['source_sha256'],
                       'source_kind': c['source_kind'], 'duration': c['duration'], 'tracks': len(c['tracks']),
                       'alpha_tracks': len(c['alpha_tracks']), 'unsupported_controllers': c['unsupported_controllers']}
                      for c in clips]}, skipped


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=HERE / 'cache')
    parser.add_argument('--match', default='', help='explicit partial source regex; manifest records this restriction')
    args = parser.parse_args()
    destination = args.output.resolve()
    allowed = HERE.resolve()
    if destination != allowed and allowed not in destination.parents:
        raise ValueError('output must remain inside NEW research/animation-browser-20261017/export staging')
    manifest_path = destination / 'animation-browser-manifest.json'
    if manifest_path.exists():
        raise FileExistsError('refusing to overwrite staged manifest; choose a new output subdirectory')
    inventory = json.loads(SOURCE_INVENTORY.read_text(encoding='utf-8'))
    assets = inventory['assets']
    canonical = {r['source'].casefold(): r['source'] for r in assets}
    by_source = {r['source']: r for r in assets}
    bindings, unbound = candidate_pairs(canonical)
    material_document = json.loads(MATERIALS.read_text(encoding='utf-8'))
    if material_document['schema'] != 1 or not material_document['complete']:
        raise ValueError('native material audit is incomplete')
    converter = Converter()
    converter.audited_materials = material_document['models']
    selected = sorted(source for source, row in by_source.items()
                      if source.lower().endswith('.nif') and (row.get('controllers') or source in bindings)
                      and (not args.match or re.search(args.match, source, re.I)))
    rigs, skipped_clips, rejected = [], [], []
    started = time.perf_counter()
    for index, source in enumerate(selected):
        try:
            rig, skipped = export_model(source, by_source[source], bindings.get(source, {}), converter, destination)
            skipped_clips += [{**row, 'model': source} for row in skipped]
            if rig:
                rigs.append(rig)
            else:
                rejected.append({'source': source, 'reason': 'no supported bound positive-duration timeline; see skipped_clips'})
        except Exception as error:
            rejected.append({'source': source, 'reason': f'{type(error).__name__}: {error}'})
        if (index + 1) % 10 == 0 or index + 1 == len(selected):
            print(f'{index + 1}/{len(selected)} models; {len(rigs)} rigs; {len(rejected)} rejected; {time.perf_counter()-started:.1f}s', flush=True)
    referenced = {c['source'] for r in rigs for c in r['clips']}
    catalog = []
    for row in assets:
        source = row['source']
        catalog.append({'source': source, 'sha256': row['sha256'], 'kind': Path(source).suffix[1:].lower(),
                        'controller_counts': dict(collections.Counter(c.get('class', c.get('controller')) for c in row.get('controllers', []))),
                        'in_exported_timeline': source in referenced, 'has_embedded_controllers': source.lower().endswith('.nif') and bool(row.get('controllers')),
                        'status': 'exported_authored_timeline' if source in referenced else 'source_catalog_only',
                        'catalog_only_reason': None if source in referenced else ('no authored controller' if not row.get('controllers') else 'unbound KF, unsupported effect-only, or explicit model/clip rejection; see rejected/skipped/unbound records')})
    manifest = {'format': 'aa-animation-browser-manifest-v1', 'complete_source_catalog': len(catalog) == 2481,
                'export_filter': args.match or None, 'generator_sha256': sha(Path(__file__)),
                'inputs': {os.path.relpath(p, WORKSPACE).replace('\\', '/'): sha(p) for p in [SOURCE_INVENTORY, ASSOCIATIONS, RECOVERED, MATERIALS, Path(base.__file__), Path(geometry.__file__)]},
                'rigs': rigs, 'sources': catalog, 'rejected_models': rejected, 'skipped_clips': skipped_clips,
                'unbound_projected_associations': unbound,
                'counts': {'source_files': len(catalog), 'selected_models': len(selected), 'rigs': len(rigs),
                           'clips': sum(len(r['clips']) for r in rigs), 'distinct_clip_sources': len(referenced),
                           'rejected_models': len(rejected), 'skipped_clips': len(skipped_clips)},
                'scope': 'Complete archive-pinned source catalog with explicit compatible model/KF and embedded-NIF geometry timelines; no gameplay/native full-effects parity. Existing cache, release and original files remain unchanged.'}
    write_new(manifest_path, manifest)
    print(json.dumps({'manifest': str(manifest_path), 'sha256': sha(manifest_path), 'counts': manifest['counts']}), flush=True)


if __name__ == '__main__':
    main()
