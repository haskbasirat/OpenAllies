"""Read-only scene-node metadata census using the existing PyFFI reader.

Original research code; no SDK implementation is imported or executed. Input
membership and complete source hashes come from scan_native_model_headers.py.
"""
from __future__ import annotations

import argparse
from collections import Counter
from concurrent.futures import ProcessPoolExecutor
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
NifFormat = None
string = None
SOURCE_ROOT = None


def initialize(reconstruction: str):
    global NifFormat, string, SOURCE_ROOT
    reconstruction = Path(reconstruction)
    sys.path.insert(0, str(reconstruction / 'tools'))
    from convert_models import NifFormat as reader, string as decode_name
    NifFormat = reader
    string = decode_name
    SOURCE_ROOT = reconstruction / 'assets' / 'original' / 'Data'


def scan(row):
    output = {'path': row['path'], 'sha256': row['sha256']}
    try:
        data = NifFormat.Data()
        with (SOURCE_ROOT / row['path']).open('rb') as source:
            data.read(source)
        types = Counter()
        seen = set()
        billboard_nodes = []
        switch_nodes = []
        lod_nodes = []
        locally_culled_nodes = []
        inherited_cull_geometry = []
        strip_geometries = 0
        strip_primitives = 0
        strip_nondegenerate = 0

        def walk(node, hierarchy=(), culled_ancestors=()):
            nonlocal strip_geometries, strip_primitives, strip_nondegenerate
            if node is None or not isinstance(node, NifFormat.NiAVObject) or id(node) in seen:
                return
            seen.add(id(node))
            kind = type(node).__name__
            name = string(node.name)
            hierarchy = hierarchy + (name,)
            types[kind] += 1
            descriptor = {'type': kind, 'hierarchy': hierarchy, 'flags': int(node.flags)}
            local_cull = bool(int(node.flags) & 1)
            if local_cull:
                locally_culled_nodes.append(descriptor)
            if isinstance(node, NifFormat.NiBillboardNode):
                billboard_nodes.append({**descriptor, 'mode': int(node.billboard_mode)})
            if isinstance(node, NifFormat.NiLODNode):
                lod_nodes.append(descriptor)
            elif isinstance(node, NifFormat.NiSwitchNode):
                switch_nodes.append(descriptor)
            if isinstance(node, NifFormat.NiTriBasedGeom) and node.data:
                if culled_ancestors and not local_cull:
                    inherited_cull_geometry.append({**descriptor, 'culled_ancestor_hierarchies': culled_ancestors})
                if isinstance(node.data, NifFormat.NiTriStripsData):
                    strip_geometries += 1
                    strip_primitives += int(node.data.num_triangles)
                    strip_nondegenerate += len(node.data.get_triangles())
            next_culled = culled_ancestors + (hierarchy,) if local_cull else culled_ancestors
            for child in getattr(node, 'children', []):
                walk(child, hierarchy, next_culled)

        for scene_root in data.roots:
            walk(scene_root)
        output.update({
            'types': dict(sorted(types.items())),
            'billboard_nodes': billboard_nodes,
            'switch_nodes': switch_nodes,
            'lod_nodes': lod_nodes,
            'locally_culled_nodes': locally_culled_nodes,
            'unflagged_geometry_under_culled_ancestor': inherited_cull_geometry,
            'strip_geometry_count': strip_geometries,
            'strip_reported_primitive_count': strip_primitives,
            'strip_reader_nondegenerate_triangle_count': strip_nondegenerate,
        })
    except Exception as error:
        output['reader_error'] = f'{type(error).__name__}: {error}'
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reconstruction', type=Path)
    parser.add_argument('header_inventory', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--workers', type=int, default=4)
    args = parser.parse_args()
    inventory = json.loads(args.header_inventory.read_text(encoding='utf-8'))
    with ProcessPoolExecutor(max_workers=args.workers, initializer=initialize,
                             initargs=(str(args.reconstruction.resolve()),)) as workers:
        models = list(workers.map(scan, inventory['records'], chunksize=8))
    parsed = [model for model in models if 'reader_error' not in model]
    type_counts = Counter()
    for model in parsed:
        type_counts.update(model['types'])
    summary = {
        'input_models': len(models),
        'parsed_models': len(parsed),
        'reader_errors': len(models) - len(parsed),
        'node_type_counts': dict(sorted(type_counts.items())),
        'models_with_billboard_nodes': sum(bool(model['billboard_nodes']) for model in parsed),
        'billboard_nodes': sum(len(model['billboard_nodes']) for model in parsed),
        'models_with_switch_nodes': sum(bool(model['switch_nodes']) for model in parsed),
        'models_with_lod_nodes': sum(bool(model['lod_nodes']) for model in parsed),
        'models_with_unflagged_geometry_under_culled_ancestor': sum(bool(model['unflagged_geometry_under_culled_ancestor']) for model in parsed),
        'unflagged_geometry_under_culled_ancestor': sum(len(model['unflagged_geometry_under_culled_ancestor']) for model in parsed),
        'strip_geometry_count': sum(model['strip_geometry_count'] for model in parsed),
        'strip_reported_primitive_count': sum(model['strip_reported_primitive_count'] for model in parsed),
        'strip_reader_nondegenerate_triangle_count': sum(model['strip_reader_nondegenerate_triangle_count'] for model in parsed),
    }
    result = {
        'schema': 'aa-native-model-node-census-v1',
        'caution': 'Metadata interpreted by existing conversion reader / PyFFI 2.2.3, with its pre-existing legacy UV field shim. This is not AA.exe execution or proof of SDK equivalence.',
        'source_root': inventory['source_root'],
        'summary': summary,
        'models': models,
    }
    args.output.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(summary, indent=2))
