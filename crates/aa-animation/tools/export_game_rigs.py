"""Export gameplay infantry rigs (role-named clips) from the original NIF/KF files.

Reads only original data plus the recovered catalog and the native material
audit; writes new files under assets-cache/animation/game/ and the manifest
assets-cache/animation/game-rigs.json. Existing rig caches/manifests are never
rewritten.

Model selection mirrors aa-game's resource path derivation: every unit in
catalog.json whose properties include "infantry" contributes
`<definition folder>/<View.resource_prefix><AspectNormal.model>` (default
"%s.nif"). Units that share a model are grouped; the unit with the most
resolvable roles supplies the clip list. Each rig keeps up to six clips, named
by role so the client can pick them without catalog access:

  idle   <- IDLE  "idle" (else first IDLE)                 loops
  move   <- MOVE  "jog", else "walk" (else first MOVE)     loops
  attack <- ABILITY "attack1", else first attack-like ABILITY (never build)
  die1..die3 <- DIE "die1".."die3" (distinct KF files only)

Each clip keeps its original KF path in `source`; the gameplay length
(catalog `length`, seconds), `walk_distance`, `looping`, `role` and the
native sequence name are additional fields ignored by aa_animation's serde.

Geometry and skin export reuse export_families.collect_model (native-audited
visible-mesh mapping, `source_mesh_index`, `static_model_sha256`) and clip
export reuses export_original.export_clip, including the provably-constant
quadratic channel reduction. Unsupported models/clips are skipped and listed
in the manifest with the exporter's reason.
"""
from __future__ import annotations
import argparse, collections, json, math, re, sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import export_original as base  # noqa: E402  (sets up the vendored PyFFI paths)
import export_families as families  # noqa: E402  (installs constant-channel reduction)
from convert_models import Converter, NifFormat, ORIGINAL, string  # noqa: E402
from convert_models_animations import read_nif, euler_matrix, quaternion_matrix  # noqa: E402

WORKSPACE = base.WORKSPACE
RECON = base.RECON
OUT = WORKSPACE / 'assets-cache/animation'
GAME = OUT / 'game'
MANIFEST = OUT / 'game-rigs.json'
CATALOG = RECON / 'game/data/catalog.json'
CONVERTED = RECON / 'assets/converted/models'
ROLES = ['idle', 'move', 'attack', 'die1', 'die2', 'die3']
LOOPING = {'idle': True, 'move': True}
sha = base.sha


def matrix_quaternion(m):
    """Quaternion q with convert_models_animations.quaternion_matrix(q) == m (NIF rows)."""
    # quaternion_matrix(q) is the transpose of the textbook matrix, so take the
    # textbook quaternion of m and conjugate it; then prove it numerically.
    a = m
    trace = a[0][0] + a[1][1] + a[2][2]
    if trace > 0:
        s = math.sqrt(trace + 1.0) * 2
        w, x, y, z = 0.25 * s, (a[2][1] - a[1][2]) / s, (a[0][2] - a[2][0]) / s, (a[1][0] - a[0][1]) / s
    elif a[0][0] > a[1][1] and a[0][0] > a[2][2]:
        s = math.sqrt(1.0 + a[0][0] - a[1][1] - a[2][2]) * 2
        w, x, y, z = (a[2][1] - a[1][2]) / s, 0.25 * s, (a[0][1] + a[1][0]) / s, (a[0][2] + a[2][0]) / s
    elif a[1][1] > a[2][2]:
        s = math.sqrt(1.0 + a[1][1] - a[0][0] - a[2][2]) * 2
        w, x, y, z = (a[0][2] - a[2][0]) / s, (a[0][1] + a[1][0]) / s, 0.25 * s, (a[1][2] + a[2][1]) / s
    else:
        s = math.sqrt(1.0 + a[2][2] - a[0][0] - a[1][1]) * 2
        w, x, y, z = (a[1][0] - a[0][1]) / s, (a[0][2] + a[2][0]) / s, (a[1][2] + a[2][1]) / s, 0.25 * s
    length = math.sqrt(w * w + x * x + y * y + z * z)
    candidates = [[-x / length, -y / length, -z / length, w / length], [x / length, y / length, z / length, w / length]]
    for q in candidates:
        back = base.matrix(quaternion_matrix(q))
        if max(abs(p - r) for rp, rr in zip(back, m) for p, r in zip(rp, rr)) < 1e-6:
            return q
    raise ValueError('constant Euler orientation is not a proper rotation')


def export_clip(path, nodes, objects, names):
    """export_original.export_clip, plus one extension: a constant (all keys
    equal, zero tangents) Euler track whose orientation differs from the bind
    matrix is stored as a single-key quaternion channel holding exactly that
    orientation (the loader only accepts ConstantBind within 1e-6). The Euler
    composition is the one proven against the rifle bind matrices. Varying
    Euler splines still fail."""
    data = read_nif(path)
    seqs = [r for r in data.roots if isinstance(r, NifFormat.NiControllerSequence)]
    if len(seqs) != 1:
        raise ValueError('Expected one original sequence')
    sequence = seqs[0]
    tracks, seen, overrides = [], set(), []
    for block in sequence.controlled_blocks:
        c = block.controller
        target = string(block.target_name)
        if c is None:
            continue
        if not isinstance(c, NifFormat.NiKeyframeController):
            raise ValueError('Unsupported ' + type(c).__name__)
        if target not in names:
            raise ValueError('Unresolved target ' + target)
        index = names[target]
        if index in seen:
            raise ValueError('Duplicate controller target ' + target)
        seen.add(index)
        d = c.data
        if not isinstance(d, NifFormat.NiKeyframeData):
            raise ValueError('Unsupported track data')
        t = {'node': index, 'target': target, 'controller': type(c).__name__, 'frequency': float(c.frequency),
             'phase': float(c.phase), 'start': float(c.start_time), 'stop': float(c.stop_time), 'flags': int(c.flags),
             'translation': base.channel(d.translations.keys, int(d.translations.interpolation)),
             'scale': base.channel(d.scales.keys, int(d.scales.interpolation)), 'rotation': None}
        if int(d.rotation_type) == 4:
            angles = []
            for axis in d.xyz_rotations:
                keys = list(axis.keys)
                if not keys:
                    raise ValueError('Missing Euler axis')
                if any(float(k.value) != float(keys[0].value) or float(getattr(k, 'forward', 0)) != 0
                       or float(getattr(k, 'backward', 0)) != 0 for k in keys):
                    raise ValueError(f'Non-constant Euler spline on {target} is unsupported')
                angles.append(float(keys[0].value))
            computed = base.matrix(euler_matrix(angles))
            authored = nodes[index]['rotation']
            delta = max(abs(a - b) for rowa, rowb in zip(computed, authored) for a, b in zip(rowa, rowb))
            if delta <= 1e-6:
                t['rotation'] = {'kind': 'constant_bind', 'euler_xyz': angles, 'max_bind_matrix_delta': delta}
            else:
                q = matrix_quaternion(computed)
                t['rotation'] = {'kind': 'quaternion', 'channel': {'interpolation': 1, 'keys': [{'time': float(c.start_time), 'value': q}]},
                                 'source_constant_euler_xyz': angles, 'bind_matrix_delta': delta}
                overrides.append({'target': target, 'euler_xyz': angles, 'bind_matrix_delta': delta})
        else:
            t['rotation'] = {'kind': 'quaternion', 'channel': base.channel(d.quaternion_keys, int(d.rotation_type))}
        if t['frequency'] != 1 or t['phase'] != 0:
            raise ValueError('Non-default controller timing needs native audit')
        tracks.append(t)
    events = [{'time': float(k.time), 'text': string(k.value)} for k in sequence.text_keys.text_keys] if sequence.text_keys else []
    duration = max([t['stop'] for t in tracks] + [e['time'] for e in events] + [0])
    if duration <= 0:
        raise ValueError('No positive duration')
    clip = {'name': string(sequence.name), 'source': path.relative_to(ORIGINAL).as_posix(), 'source_sha256': sha(path),
            'duration': duration, 'cycle_type': int(sequence.cycle_type) if data.version >= 0x0A01006A else None,
            'tracks': tracks, 'events': events}
    if overrides:
        clip['constant_euler_overrides'] = overrides
    return clip


def as_list(value):
    if value is None:
        return []
    return value if isinstance(value, list) else [value]


def resource_path(prefix, folder, fmt):
    """Same rules as aa-game data.rs resource_path."""
    expanded = fmt.replace('%s', prefix)
    if expanded.startswith('/'):
        path = expanded[1:]
    elif prefix.startswith('/'):
        path = expanded.lstrip('/')
    else:
        path = f'{folder}/{expanded}'
    return path.lstrip('/')


def model_path(unit):
    data = unit.get('data', {})
    view = data.get('View') if isinstance(data.get('View'), dict) else {}
    prefix = view.get('resource_prefix') or unit.get('resourcePrefix')
    if not isinstance(prefix, str):
        return None, None
    folder = unit['source'].rsplit('/', 1)[0] if '/' in unit['source'] else ''
    normal = view.get('AspectNormal') if isinstance(view.get('AspectNormal'), dict) else {}
    fmt = normal.get('model') if isinstance(normal.get('model'), str) else '%s.nif'
    return resource_path(prefix, folder, fmt), prefix


def resolve_kf(kf, ids, prefix, folder, originals):
    if not isinstance(kf, str):
        return None
    if '%s' in kf:
        if not isinstance(ids, str):
            return None
        try:
            kf = kf % (prefix, ids)
        except TypeError:
            return None
    rel = kf.lstrip('/') if kf.startswith('/') else f'{folder}/{kf}'
    return originals.get(rel.casefold())


def candidates(anims, kind, preferred, exclude=()):
    """Ordered candidate rows: preferred ids first, then other rows of the
    kind whose ids do not start with an excluded prefix."""
    rows = [a for a in anims if str(a.get('type', '')).upper() == kind]
    ordered = []
    for name in preferred:
        ordered += [a for a in rows if str(a.get('ids', '')).lower() == name and a not in ordered]
    ordered += [a for a in rows if a not in ordered
                and not any(str(a.get('ids', '')).lower().startswith(x) for x in exclude)]
    return ordered


def role_rows(unit):
    cc = unit.get('data', {}).get('CharacterComponent') or {}
    anims = [a for a in as_list(cc.get('Animation')) if isinstance(a, dict)]
    rows = {
        'idle': candidates(anims, 'IDLE', ['idle'], exclude=('idleattack', 'idleprojectile', 'idlemelee')),
        'move': candidates(anims, 'MOVE', ['jog', 'walk']),
        'attack': candidates(anims, 'ABILITY', ['attack1'], exclude=('build', 'repair', 'projectile', 'melee'))
        or candidates(anims, 'ABILITY', ['attack1'], exclude=('build', 'repair')),
    }
    dies = [a for a in anims if str(a.get('type', '')).upper() == 'DIE']
    for i in (1, 2, 3):
        rows[f'die{i}'] = [a for a in dies if str(a.get('ids', '')).lower() == f'die{i}'][:1]
    return rows


def gather(catalog, originals):
    """Group infantry units by resolved model path."""
    groups = collections.OrderedDict()
    unresolved = []
    for uid, unit in sorted(catalog['units'].items()):
        if 'infantry' not in [str(p).lower() for p in unit.get('properties', [])]:
            continue
        path, prefix = model_path(unit)
        if path is None:
            unresolved.append({'unit': uid, 'reason': 'no View.resource_prefix'})
            continue
        original = originals.get(path.casefold())
        if original is None:
            unresolved.append({'unit': uid, 'model': path, 'reason': 'model NIF not found in original data'})
            continue
        folder = unit['source'].rsplit('/', 1)[0]
        rows = role_rows(unit)
        clips = {}
        for role, options in rows.items():
            if options:
                clips[role] = [{'row': row, 'kf': resolve_kf(row.get('kf'), row.get('ids'), prefix, folder, originals)}
                               for row in options]
        group = groups.setdefault(path.casefold(), {'model': path, 'original': original, 'units': []})
        group['units'].append({'unit': uid, 'nation': unit.get('nation'), 'clips': clips})
    return groups, unresolved


def primary(units):
    def score(u):
        resolved = sum(1 for c in u['clips'].values() if any(o['kf'] is not None for o in c))
        penalty = 1 if re.search('dummy|amphib', u['unit']) else 0
        return (-resolved, penalty, u['unit'])
    return sorted(units, key=score)[0]


def converted_check(original_rel, meshes):
    static = json.loads((CONVERTED / Path(original_rel).with_suffix('.json')).read_text())
    counts = []
    for mesh in meshes:
        converted = static['meshes'][mesh['source_mesh_index']]
        if len(converted['positions']) != 3 * len(mesh['positions']):
            raise ValueError('vertex count differs from converted model')
        counts.append(len(mesh['positions']))
    return counts


def stem_names(groups):
    stems = collections.Counter(Path(g['model']).stem.casefold() for g in groups.values())
    names = {}
    for key, g in groups.items():
        stem = Path(g['model']).stem  # catalog casing (on-disk names are sometimes lowercase)
        if stems[stem.casefold()] > 1:
            stem = Path(g['model']).parent.name + '_' + stem
        names[key] = stem
    if len(set(n.casefold() for n in names.values())) != len(names):
        raise ValueError('output name collision')
    return names


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--match', default='', help='regex filter on model path (testing)')
    parser.add_argument('--reference-dir', default='', help='also write PyFFI reference poses here')
    parser.add_argument('--dry-run', action='store_true')
    args = parser.parse_args()
    started = time.perf_counter()
    catalog = json.loads(CATALOG.read_text())
    originals = {p.relative_to(ORIGINAL).as_posix().casefold(): p
                 for p in ORIGINAL.rglob('*') if p.suffix.lower() in ('.nif', '.kf')}
    groups, unresolved = gather(catalog, originals)
    names = stem_names(groups)
    material_path = WORKSPACE / 'assets-cache/model-materials.json'
    material_document = json.loads(material_path.read_text())
    if material_document['schema'] != 1 or not material_document['complete']:
        raise ValueError('Native material audit is incomplete')
    converter = Converter()
    converter.audited_materials = material_document['models']
    if not args.dry_run:
        GAME.mkdir(parents=True, exist_ok=True)
    reference_dir = Path(args.reference_dir) if args.reference_dir else None
    if reference_dir:
        reference_dir.mkdir(parents=True, exist_ok=True)
    rigs, skipped_models, skipped_clips = [], [], []
    for key, group in groups.items():
        if args.match and not re.search(args.match, group['model'], re.I):
            continue
        chosen = primary(group['units'])
        original = group['original']
        original_rel = original.relative_to(ORIGINAL).as_posix()
        name = names[key]
        unit_ids = [u['unit'] for u in group['units']]
        disagreements = []
        for other in group['units']:
            for role in ROLES:
                a = [o['kf'] for o in chosen['clips'].get(role, [])]
                b = [o['kf'] for o in other['clips'].get(role, [])]
                b = b[0] if b and b != a else None
                if b is not None:
                    disagreements.append({'unit': other['unit'], 'role': role,
                                          'kf': b.relative_to(ORIGINAL).as_posix()})
        try:
            data, nodes, meshes, objects, names_map, excluded = families.collect_model(original, converter)
            if not meshes:
                raise ValueError('no visible meshes')
            if not any(m['skin'] for m in meshes):
                raise ValueError('no skinned mesh (not an infantry body)')
            vertex_counts = converted_check(original_rel, meshes)
        except Exception as error:  # noqa: BLE001 - recorded in manifest
            skipped_models.append({'model': group['model'], 'original': original_rel, 'units': unit_ids,
                                   'reason': f'{type(error).__name__}: {error}'})
            print(f'SKIP model {group["model"]}: {error}', flush=True)
            continue
        clips, records, used = [], [], {}
        for role in ROLES:
            clip = None
            for option in chosen['clips'].get(role, []):
                row, kf = option['row'], option['kf']
                if kf is None:
                    skipped_clips.append({'model': group['model'], 'role': role, 'kf': row.get('kf'),
                                          'reason': 'KF file not found in original data'})
                    continue
                kf_rel = kf.relative_to(ORIGINAL).as_posix()
                if kf_rel.casefold() in used:
                    skipped_clips.append({'model': group['model'], 'role': role, 'kf': kf_rel,
                                          'reason': f'duplicate of role {used[kf_rel.casefold()]} (same KF)'})
                    continue
                try:
                    clip = export_clip(kf, nodes, objects, names_map)
                    break
                except Exception as error:  # noqa: BLE001
                    skipped_clips.append({'model': group['model'], 'role': role, 'kf': kf_rel,
                                          'reason': f'{type(error).__name__}: {error}'})
                    print(f'  skip clip {group["model"]} {role} {kf_rel}: {error}', flush=True)
            if clip is None:
                continue
            used[kf_rel.casefold()] = role
            length = row.get('length')
            length = float(length) if isinstance(length, (int, float)) and length > 0 else clip['duration']
            walk = row.get('walk_distance')
            walk = float(walk) if isinstance(walk, (int, float)) and walk > 0 else 1.0
            clip['sequence_name'] = clip['name']
            clip['name'] = role
            clip['role'] = 'die' if role.startswith('die') else role
            clip['catalog_ids'] = row.get('ids')
            clip['gameplay_length'] = length
            clip['walk_distance'] = walk
            clip['looping'] = LOOPING.get(role, False)
            hides = sorted({str(e.get('equipment_type_ids') or e.get('location'))
                            for e in as_list(row.get('WhenStart')) if isinstance(e, dict)
                            and e.get('type') == 'EQUIPMENT' and str(e.get('action')).upper() == 'HIDE'})
            if hides:
                clip['hides_equipment'] = hides
            clips.append(clip)
            records.append({'name': role, 'role': clip['role'], 'index': len(clips) - 1, 'source': kf_rel,
                            'catalog_ids': row.get('ids'), 'duration': clip['duration'], 'gameplay_length': length,
                            'walk_distance': walk, 'looping': clip['looping'], 'hides_equipment': hides,
                            'constant_euler_overrides': [o['target'] for o in clip.get('constant_euler_overrides', [])]})
        if not any(r['role'] == 'idle' for r in records):
            skipped_models.append({'model': group['model'], 'original': original_rel, 'units': unit_ids,
                                   'reason': 'no exportable idle clip'})
            print(f'SKIP model {group["model"]}: no idle', flush=True)
            continue
        target = GAME / f'{name}.rig.json'
        result = {
            'format': 'aa-original-rig-v1', 'source': original_rel, 'source_sha256': sha(original),
            'static_model_sha256': sha(CONVERTED / Path(original_rel).with_suffix('.json')),
            'coordinates': 'native Z-up local data; output(x,z,-y); browser-oriented UVs',
            'nif_version': '.'.join(str((data.version >> n) & 255) for n in (24, 16, 8, 0)),
            'nodes': nodes, 'meshes': [{k: v for k, v in m.items() if k != '_object'} for m in meshes],
            'clips': clips, 'excluded_geometry': excluded,
            'gameplay': {'catalog_unit': chosen['unit'], 'units': unit_ids, 'model': group['model']},
            'limitations': [
                'Gameplay role clips chosen from the catalog CharacterComponent of the listed unit; no blending or native state machine.',
                'Only linear curves, quaternion SLERP, verified constant Euler bind orientation and provably constant zero-tangent quadratic channels.',
                'Python/PyFFI differential reference, not executed native Gamebryo interpolation.']}
        if reference_dir:
            refs = [{'clip': None, 'time': 0, 'meshes': base.reference_pose(nodes, meshes, objects, None, 0)}]
            for i, clip in enumerate(clips):
                for fraction in [0, .5, 1]:
                    t = fraction * clip['duration']
                    refs.append({'clip': i, 'time': t, 'meshes': base.reference_pose(nodes, meshes, objects, clip, t)})
            (reference_dir / f'{name}.reference.json').write_text(
                json.dumps({'source_sha256': result['source_sha256'], 'poses': refs}, separators=(',', ':')) + '\n')
        if not args.dry_run:
            target.write_text(json.dumps(result, separators=(',', ':')) + '\n')
        rigs.append({
            'model': group['model'], 'model_key': group['model'].casefold().replace('\\', '/'),
            'original': original_rel, 'file': f'game/{name}.rig.json',
            'sha256': sha(target) if not args.dry_run else None,
            'nation': chosen['nation'], 'catalog_unit': chosen['unit'], 'units': unit_ids,
            'meshes': len(meshes), 'vertices': sum(vertex_counts), 'mesh_vertices': vertex_counts,
            'source_mesh_indices': [m['source_mesh_index'] for m in meshes],
            'clips': records, 'unit_clip_disagreements': disagreements})
        print(f'Exported {name}: {[r["name"] for r in records]} ({sum(vertex_counts)} verts)', flush=True)
    manifest = {
        'format': 'aa-game-rig-manifest-v1',
        'generator_sha256': sha(Path(__file__)),
        'phase2_exporter_sha256': sha(Path(base.__file__)),
        'families_exporter_sha256': sha(Path(families.__file__)),
        'catalog_sha256': sha(CATALOG), 'material_audit_sha256': sha(material_path),
        'roles': ROLES,
        'notes': 'model = aa-game resource path (catalog casing); compare via model_key (lowercase). '
                 'Clip time: native KF seconds = gameplay seconds * duration / gameplay_length '
                 '(move additionally * speed / (visual_scale * walk_distance)).',
        'rigs': rigs, 'skipped_models': skipped_models, 'skipped_clips': skipped_clips,
        'unresolved_units': unresolved,
    }
    if not args.dry_run and not args.match:
        MANIFEST.write_text(json.dumps(manifest, indent=1) + '\n')
    print(json.dumps({'rigs': len(rigs), 'clips': sum(len(r['clips']) for r in rigs),
                      'skipped_models': len(skipped_models), 'skipped_clips': len(skipped_clips),
                      'unresolved_units': len(unresolved), 'seconds': round(time.perf_counter() - started, 1)}))


if __name__ == '__main__':
    main()
