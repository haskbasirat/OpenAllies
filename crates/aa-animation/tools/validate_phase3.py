"""Validate frozen family bytes/source identities; optionally publish exact files.

Does not generate artwork, modify source assets, or select native game states.
Promotion creates new top-level cache files only after all audits pass.
"""
from __future__ import annotations
import argparse, hashlib, json
from pathlib import Path

WORKSPACE = Path(__file__).resolve().parents[3]
RECON = WORKSPACE.parent / 'reconstruction'
ORIGINAL = RECON / 'assets/original/Data'
CACHE = WORKSPACE / 'assets-cache/animation'
OUT = WORKSPACE / 'research/phase3-animation'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def contained(root, relative):
    path = (root / relative).resolve()
    if Path(relative).is_absolute() or not path.is_relative_to(root.resolve()):
        raise ValueError('Noncontained source path')
    return path


def validate():
    checks = 0
    def check(value, message):
        nonlocal checks
        checks += 1
        if not value:
            raise ValueError(message)
    manifest_path = CACHE / 'families-manifest.json'
    manifest = json.loads(manifest_path.read_text())
    material_path = WORKSPACE / 'assets-cache/model-materials.json'
    material = json.loads(material_path.read_text())
    check(manifest['generator_sha256'] == sha(Path(__file__).with_name('export_families.py')), 'Exporter changed')
    check(manifest['material_audit_sha256'] == sha(material_path), 'Material audit changed')
    old_manifest = CACHE / 'manifest.json'
    check(sha(old_manifest) == '2f1db6249df071eaed4323bc2de1d06a4efcdfe82243aa31a84f932235a3bab1', 'Phase2 manifest changed')
    old = json.loads(old_manifest.read_text())
    for r in old['assets']:
        for field, digest in [('file', 'sha256'), ('reference', 'reference_sha256')]:
            check(sha(contained(CACHE, r[field])) == r[digest], 'Phase2 cache changed')
    artifacts, placeholders, movement = [], [], []
    for row in manifest['exports']:
        for field, digest in [('file', 'sha256'), ('reference', 'reference_sha256')]:
            path = contained(CACHE, row[field])
            check(sha(path) == row[digest], 'Staged byte hash mismatch')
            artifacts.append({'source': row[field], 'file': path.name,
                              'size': path.stat().st_size, 'sha256': row[digest]})
        rig = json.loads(contained(CACHE, row['file']).read_text())
        check(sha(contained(ORIGINAL, rig['source'])) == rig['source_sha256'] == row['source_sha256'], 'Original NIF identity changed')
        converted_path = contained(RECON / 'assets/converted/models', str(Path(rig['source']).with_suffix('.json')))
        converted = json.loads(converted_path.read_text())
        audit = material['models'][rig['source']]
        check(sha(converted_path) == rig['static_model_sha256'] == audit['converted_sha256'], 'Static JSON identity changed')
        check(rig['source_sha256'] == audit['source_sha256'], 'Audit NIF identity changed')
        check([m['source_mesh_index'] for m in rig['meshes']] ==
              [c['mesh_index'] for c in audit['corrections'] if not c['skip']], 'Audited mesh mapping differs')
        for mesh in rig['meshes']:
            source = converted['meshes'][mesh['source_mesh_index']]
            check(mesh['name'] == source['name'] and mesh['indices'] == source['indices'] and
                  len(mesh['positions']) * 3 == len(source['positions']), 'Mesh identity/topology mismatch')
            check(len(mesh['uvs']) == len(mesh['positions']), 'Incomplete UV array')
            texture = mesh['material'].get('texture')
            if texture:
                check(contained(RECON, texture.lstrip('/')).is_file(), 'Texture resource missing')
            if mesh['untextured_uv_placeholder']:
                check(not texture and not source['material'].get('texture') and not source['uvs'] and
                      all(u == [0.0, 0.0] for u in mesh['uvs']), 'Invalid UV placeholder')
                placeholders.append({'model': rig['source'], 'mesh': mesh['name'], 'vertices': len(mesh['positions'])})
        reference = json.loads(contained(CACHE, row['reference']).read_text())
        check(reference['source_sha256'] == rig['source_sha256'], 'Reference source identity mismatch')
        for index, clip in enumerate(rig['clips']):
            check(sha(contained(ORIGINAL, clip['source'])) == clip['source_sha256'], 'Original KF identity changed')
            poses = [p for p in reference['poses'] if p['clip'] == index]
            check(len(poses) == 5, 'Reference sample count')
            first = [v for mesh in poses[0]['meshes'] for point in mesh['positions'] for v in point]
            delta = max(abs(a - b) for pose in poses[1:] for a, b in zip(first,
                        [v for mesh in pose['meshes'] for point in mesh['positions'] for v in point]))
            check(delta > 1e-8, 'No visible movement in original sampled clip')
            movement.append({'model': rig['source'], 'clip': clip['name'], 'maximum_sampled_coordinate_delta': delta})
    check(len({r['file'].casefold() for r in artifacts}) == len(artifacts), 'Output filename collision')
    pinned = {'validator': Path(__file__), 'library': WORKSPACE / 'crates/aa-animation/src/lib.rs',
              'family_exporter': Path(__file__).with_name('export_families.py'),
              'phase2_exporter': Path(__file__).with_name('export_original.py'),
              'original_rig_tests': WORKSPACE / 'crates/aa-animation/tests/original_rigs.rs',
              'family_tests': WORKSPACE / 'crates/aa-animation/tests/phase3_families.rs',
              'constant_reduction_tests': Path(__file__).with_name('test_phase3_export.py'),
              'parser': RECON / 'tools/convert_models_vendor/pyffi/formats/nif/__init__.py',
              'parser_schema': RECON / 'tools/convert_models_vendor/pyffi/formats/nif/nifxml/nif.xml',
              'reference_helper': RECON / 'tools/convert_models_animations.py',
              'material_resolver': RECON / 'tools/convert_models.py'}
    return {'passed': True, 'checks': checks, 'family_manifest_sha256': sha(manifest_path),
            'phase2_manifest_sha256': sha(old_manifest), 'material_audit_sha256': sha(material_path),
            'source_pins': {key: {'path': str(path.relative_to(WORKSPACE) if path.is_relative_to(WORKSPACE) else path),
                                  'sha256': sha(path)} for key, path in pinned.items()},
            'models': len(manifest['exports']), 'clips': len(movement),
            'artifacts': artifacts, 'untextured_uv_placeholders': placeholders,
            'sampled_movement': movement,
            'scope': 'Byte/source/topology/resource audit; Rust/PyFFI pose tests and GPU acceptance are separate.'}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--promote', action='store_true')
    args = parser.parse_args()
    report = validate()
    target = OUT / 'family-byte-validation.json'
    target.write_text(json.dumps(report, indent=2) + '\n')
    if args.promote:
        # Check all destinations before the first new file. Existing identical
        # bytes permit deterministic resume; different bytes are never replaced.
        for row in report['artifacts']:
            dest = contained(CACHE, row['file'])
            if dest.exists() and sha(dest) != row['sha256']:
                raise ValueError('Publication would replace a different cache file')
        for row in report['artifacts']:
            src, dest = contained(CACHE, row['source']), contained(CACHE, row['file'])
            if not dest.exists():
                with dest.open('xb') as output:
                    output.write(src.read_bytes())
            if sha(dest) != row['sha256']:
                raise ValueError('Published byte hash mismatch')
        publication = {'format': 'aa-animation-approved-family-publication-v1',
                       'source_manifest_sha256': report['family_manifest_sha256'],
                       'byte_validation_sha256': sha(target),
                       'models': report['models'], 'clips': report['clips'], 'artifacts': report['artifacts'],
                       'scope': 'Source/reference candidate publication for client GPU acceptance, not gameplay parity.'}
        (CACHE / 'promoted-families.json').write_text(json.dumps(publication, indent=2) + '\n')
    print(json.dumps({'passed': True, 'checks': report['checks'], 'models': report['models'],
                      'clips': report['clips'], 'promoted': args.promote, 'report_sha256': sha(target)}))


if __name__ == '__main__':
    main()
