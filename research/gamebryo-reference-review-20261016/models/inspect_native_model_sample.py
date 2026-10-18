"""Read a small original-asset sample with the existing conversion parser.

This is an original metadata-only research utility. It imports the workspace's
existing PyFFI-based conversion reader, never any Gamebryo SDK code. Reported
objects are that reader's interpretation, not original-executable proof.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True


def inspect(reconstruction: Path) -> dict:
    sys.path.insert(0, str(reconstruction / 'tools'))
    from convert_models import NifFormat, string

    root = reconstruction / 'assets' / 'original' / 'Data'
    selections = [
        'Doodads/Shared/Waves/Waves01.nif',
        'Buildings/Campaign/BeachMinePole/BeachMinePole.nif',
        'Bridges/LondonBridge/LondonBridge.nif',
        'Doodads/Audio/Speaker/Speaker.nif',
        'Doodads/Temperate/TemperateLakeLilyFish/TemperateLakeLilyFish01.nif',
    ]
    models = []
    for relative in selections:
        path = root / relative
        raw = path.read_bytes()
        data = NifFormat.Data()
        with path.open('rb') as source:
            data.read(source)
        nodes = []
        visited = set()

        def walk(node, parent=None, child_slot=None):
            if node is None or not isinstance(node, NifFormat.NiAVObject) or id(node) in visited:
                return
            visited.add(id(node))
            ordinal = len(nodes)
            record = {
                'ordinal': ordinal,
                'parent_ordinal': parent,
                'child_slot': child_slot,
                'type': type(node).__name__,
                'name': string(node.name),
                'flags': int(node.flags),
                'scale': float(node.scale),
            }
            children = list(getattr(node, 'children', []))
            if children:
                record['child_slots'] = len(children)
                record['nonnull_children'] = sum(child is not None for child in children)
            if isinstance(node, NifFormat.NiBillboardNode):
                record['billboard_mode'] = int(node.billboard_mode)
            geometry = getattr(node, 'data', None)
            if isinstance(node, NifFormat.NiTriBasedGeom) and geometry:
                record['geometry'] = {
                    'type': type(geometry).__name__,
                    'vertices': int(geometry.num_vertices),
                    'reported_triangles': int(geometry.num_triangles),
                    'reader_nondegenerate_triangles': len(geometry.get_triangles()),
                    'center': [float(geometry.center.x), float(geometry.center.y), float(geometry.center.z)],
                    'radius': float(geometry.radius),
                }
                if isinstance(geometry, NifFormat.NiTriStripsData):
                    record['geometry']['strip_lengths'] = [int(x) for x in geometry.strip_lengths]
            nodes.append(record)
            for index, child in enumerate(children):
                walk(child, ordinal, index)

        for index, scene_root in enumerate(data.roots):
            walk(scene_root, child_slot=index)
        models.append({
            'path': relative,
            'sha256': hashlib.sha256(raw).hexdigest(),
            'version': f'0x{data.version:08x}',
            'reader': 'existing convert_models.py / vendored PyFFI 2.2.3',
            'nodes': nodes,
        })
    return {
        'schema': 'aa-native-model-metadata-sample-v1',
        'caution': 'Read-only third-party parser interpretation; no AA.exe execution, behavior proof, or SDK implementation execution.',
        'source_root': str(root.resolve()),
        'models': models,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reconstruction', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result = inspect(args.reconstruction.resolve())
    args.output.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    for model in result['models']:
        print(json.dumps({
            'path': model['path'],
            'types': dict(Counter(node['type'] for node in model['nodes'])),
            'node_count': len(model['nodes']),
            'geometry_count': sum('geometry' in node for node in model['nodes']),
        }))
