"""Source-complete inventory index and explicitly qualified TGI associations.

This is a research catalog, never an animation-state-machine input. The original
TGI declarations and controller records stay authoritative source evidence;
the older recovered template projection supplies candidate associations only.
"""
from __future__ import annotations
import collections, hashlib, json
from pathlib import Path

WORKSPACE = Path(__file__).resolve().parents[3]
RECON = WORKSPACE.parent / 'reconstruction'
OUT = WORKSPACE / 'research/phase3-animation'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def items(value):
    return value if isinstance(value, list) else [value] if isinstance(value, dict) else []


def unsupported(row):
    reasons = set()
    for c in row['controllers']:
        if c['class'] != 'NiKeyframeController':
            reasons.add('unimplemented controller: ' + c['class'])
        if c['frequency'] != 1 or c['phase'] != 0:
            reasons.add('nondefault controller frequency/phase')
        for field in ('translation', 'scale', 'quaternion'):
            channel = c.get(field)
            if channel and channel['keys'] and channel['kind'] != 1:
                if channel['kind'] != 2 or not channel['constant_with_zero_tangents']:
                    reasons.add(f"varying/unsupported {field} key type {channel['kind']}")
        if any(a['keys'] and not a['constant_with_zero_tangents'] for a in c.get('euler_xyz', [])):
            reasons.add('varying Euler rotation')
    return sorted(reasons)


def main():
    inventory_path = OUT / 'source-inventory.json'
    inventory = json.loads(inventory_path.read_text())
    if inventory['errors'] or inventory['parsed_count'] != inventory['source_file_count']:
        raise ValueError('Source inventory is incomplete')
    assets = {r['source']: r for r in inventory['assets']}
    canonical = {k.casefold(): k for k in assets}
    catalog_path = RECON / 'game/data/catalog.json'
    recovered = json.loads(catalog_path.read_text())
    projected, unresolved = [], []

    def resolve(raw, source, prefix, animation_id=None):
        if not isinstance(raw, str):
            return None
        if '%s' in raw:
            substitutions = (prefix,) if raw.count('%s') == 1 else (prefix, animation_id)
            if any(not isinstance(v, str) for v in substitutions):
                return None
            try:
                raw = raw % substitutions
            except (ValueError, TypeError):
                return None
        if '%' in raw:
            return None
        path = raw.lstrip('/') if raw.startswith('/') else (Path(source).parent / raw).as_posix()
        return canonical.get(path.casefold())

    def animation_record(actor, category, thing, view, animation, scope, aspect_id, model):
        if not isinstance(animation.get('kf'), str):
            return
        kf = resolve(animation['kf'], thing['source'], view.get('resource_prefix'), animation.get('ids'))
        row = {'actor': actor, 'category': category, 'source_tgi': thing['source'],
               'scope': scope, 'aspect_ids': aspect_id, 'model': model, 'kf': kf,
               'declared': {k: animation.get(k) for k in ('ids', 'type', 'kf', 'length', 'key_ratio',
                              'walk_distance', 'animation_blend', 'sister_anim_type', 'weight')}}
        (projected if kf else unresolved).append(row)

    for category in ('units', 'buildings', 'things'):
        for actor, thing in sorted(recovered[category].items()):
            view = thing.get('data', {}).get('View', {})
            if not isinstance(view, dict):
                continue
            prefix = view.get('resource_prefix')
            aspect_models = collections.defaultdict(list)
            for key, value in view.items():
                if not key.casefold().startswith('aspect'):
                    continue
                for aspect in items(value):
                    aspect_id = aspect.get('ids')
                    model_prefix = prefix + (aspect.get('aspect_prefix') or '') if isinstance(prefix, str) else None
                    model = resolve(aspect.get('model'), thing['source'], model_prefix)
                    if model:
                        aspect_models[aspect_id].append(model)
                    for animation in items(aspect.get('Animation')):
                        animation_record(actor, category, thing, view, animation,
                                         'View.' + key, aspect_id, model)
            character = thing.get('data', {}).get('CharacterComponent', {})
            if isinstance(character, dict):
                # Do not silently choose an aspect for the default character
                # animation list: native fallback/state selection is separate.
                for animation in items(character.get('Animation')):
                    animation_record(actor, category, thing, view, animation,
                                     'CharacterComponent.Animation', None, None)
                for animation_set in items(character.get('AspectAnimationSet')):
                    aspect_id = animation_set.get('aspect_ids')
                    models = aspect_models.get(aspect_id, []) if isinstance(aspect_id, str) else []
                    model = models[0] if len(set(models)) == 1 else None
                    for animation in items(animation_set.get('Animation')):
                        animation_record(actor, category, thing, view, animation,
                                         'CharacterComponent.AspectAnimationSet', aspect_id, model)
    by_kf = collections.defaultdict(list)
    for index, row in enumerate(projected):
        by_kf[row['kf']].append(index)
    families_path = WORKSPACE / 'assets-cache/animation/families-manifest.json'
    families = json.loads(families_path.read_text())
    published = {r['source']: r for r in families['exports']}
    skipped = {r['model']: r['reason'] for r in families['skipped']}
    rows = []
    for path, source in sorted(assets.items()):
        controllers = source['controllers']
        ranges = [(c['start'], c['stop']) for c in controllers if c['start'] is not None and c['stop'] is not None]
        pack = path.startswith('Buildings/') and Path(path).suffix.lower() == '.nif' and Path(path).stem.casefold().endswith(('pack', 'unpack'))
        rows.append({'source': path, 'sha256': source['sha256'], 'size': source['size'],
                     'kind': Path(path).suffix.lower().lstrip('.'),
                     'controller_counts': dict(sorted(collections.Counter(c['class'] for c in controllers).items())),
                     'controller_range': [min(a for a, b in ranges), max(b for a, b in ranges)] if ranges else None,
                     'sequence_names': [s['name'] for s in source['sequences']],
                     'source_inventory_key': path, 'projected_association_indices': by_kf.get(path, []),
                     'embedded_building_pack_unpack': pack,
                     'known_unsupported_features': unsupported(source),
                     'family_status': 'staged_reference_validated_candidate' if path in published else 'rejected_selected_family' if path in skipped else 'inventoried_not_exported',
                     'family_rejection': skipped.get(path)})
    counts = {'source_files': len(rows), 'nif': inventory['nif_count'], 'kf': inventory['kf_count'],
              'nif_with_controllers': sum(r['kind'] == 'nif' and bool(r['controller_counts']) for r in rows),
              'building_pack_unpack_nif': sum(r['embedded_building_pack_unpack'] for r in rows),
              'projected_resolved_kf_declarations': len(projected), 'projected_unresolved_kf_declarations': len(unresolved),
              'kf_with_projected_references': len(by_kf),
              'new_models': len(families['exports']), 'new_clips': sum(r['clips'] for r in families['exports']),
              'rejected_selected_models': len(families['skipped'])}
    report = {'format': 'aa-animation-source-catalog-v1', 'counts': counts,
              'generator_sha256': sha(Path(__file__)), 'source_inventory_sha256': sha(inventory_path),
              'recovered_projection_sha256': sha(catalog_path), 'family_manifest_sha256': sha(families_path),
              'assets': rows, 'projected_associations': projected, 'unresolved_projected_associations': unresolved,
              'tgi_source_hashes': inventory['tgi_sources'],
              'evidence_limits': [
                  'Complete NIF/KF source enumeration, not complete animation playback.',
                  'Projected associations come from an older recovered template catalog; native list merging and runtime selection are not proven here.',
                  'Raw source TGI declaration lines and ancestors are retained in source-inventory.json.',
                  'Authored controller range and TGI length are separate; inspector does not apply gameplay retiming or transition rules.',
                  'No varying spline, path, look-at, particle, alpha, flip, texture-transform or material-color controller playback.',
                  'Building Pack/Unpack files remain inventoried only; no partial visual sequence is presented as a complete animation.',
                  'No truck/wheel motion or battlefield movement is fabricated from model names.',
                  'This original artwork evidence does not replace the modified v14 gameplay baseline.']}
    dest = OUT / 'animation-catalog.json'
    dest.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'counts': counts, 'sha256': sha(dest)}))


if __name__ == '__main__':
    main()
