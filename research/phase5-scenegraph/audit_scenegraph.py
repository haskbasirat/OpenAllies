"""Read-only source scenegraph audit for every model referenced by Crete.

Reports authored hidden/LOD/controller geometry separately from exported meshes.
This does not interpret visibility controllers or invent missing geometry.
"""
from __future__ import annotations
import collections
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools'))
from convert_models import NifFormat, ORIGINAL, string
from convert_models_animations import read_nif
sys.path.insert(0, str(ROOT / 'research/texture-phase2'))
from export_model_materials import extras, tag_evidence, next_context
CONVERTED = GAME / 'reconstruction/assets/converted/models'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def audit(relative):
    original = ORIGINAL / relative
    converted = CONVERTED / str(Path(relative).with_suffix('.json'))
    data = read_nif(original)
    exported = json.loads(converted.read_text())
    meshes = {tuple(m['hierarchy']): m for m in exported['meshes']}
    nodes, geometries, switches = [], [], []
    seen = set()
    def walk(node, hierarchy=(), hidden_ancestor=False, lod=(), context=None):
        if not isinstance(node, NifFormat.NiAVObject) or id(node) in seen:
            return
        seen.add(id(node))
        path = hierarchy + (string(node.name),)
        flags = int(node.flags)
        own = tag_evidence(extras(node), list(path))
        state = next_context(isinstance(node, NifFormat.NiGeometry), context or {}, own)
        controllers, controller_seen = [], set()
        controller = node.controller
        while controller is not None and id(controller) not in controller_seen:
            controller_seen.add(id(controller))
            controllers.append(type(controller).__name__)
            controller = controller.next_controller
        row = {'hierarchy': list(path), 'type': type(node).__name__, 'flags': flags,
               'hidden_ancestor': hidden_ancestor, 'controllers': controllers, 'lod_branches': list(lod),
               'native_tag_evidence': state}
        if isinstance(node, NifFormat.NiSwitchNode):
            switches.append({**row, 'active_index': int(node.active_index)})
        if isinstance(node, NifFormat.NiTriBasedGeom) and node.data:
            row.update(vertices=len(node.data.vertices), triangles=len(list(node.data.get_triangles())),
                       exported=path in meshes, skin=node.is_skin())
            if path in meshes:
                m=meshes[path]
                assert row['vertices'] == len(m['positions'])//3
                assert row['triangles'] == len(m['indices'])//3
            geometries.append(row)
        nodes.append(row)
        for index, child in enumerate(getattr(node, 'children', [])):
            walk(child, path, hidden_ancestor or bool(flags & 1), lod + (index,) if isinstance(node, NifFormat.NiLODNode) else lod, state)
    for node in data.roots:
        walk(node)
    assert sum(g['exported'] for g in geometries) == len(exported['meshes'])
    return {'source_sha256':digest(original), 'converted_sha256':digest(converted),
            'node_count':len(nodes), 'geometry':geometries, 'switch_nodes':switches,
            'exported_mesh_count':len(exported['meshes']), 'skipped':exported['skippedHelpers']}

def main():
    path = GAME/'reconstruction/game/data/map-actors/Campaign/axis01_crete.json'
    placement = json.loads(path.read_text())
    models = sorted({m['path'] for t in placement['types'].values() for m in t['models']})
    rows={m:audit(m) for m in models}
    placements = collections.Counter(a['thingId'] for a in placement['actors'])
    speaker_ids={key:placements[key] for key,t in placement['types'].items() if any(m['path'].lower()=='doodads/audio/speaker/speaker.nif' for m in t['models'])}
    result={'schema':1,'map':placement['id'],'map_sha256':digest(path),'model_count':len(rows),'models':rows,
            'speaker_editor_marker_instances':sum(speaker_ids.values()),'speaker_thing_counts':speaker_ids,
            'scope':'Read-only authored NIF static mesh inventory audit. No claim of native animation/LOD/controller runtime parity.'}
    target=Path(__file__).with_name('crete-scenegraph-audit.json')
    target.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'models':len(rows),'exported_meshes':sum(r['exported_mesh_count'] for r in rows.values()),
        'native_geometry':sum(len(r['geometry']) for r in rows.values()),'speaker_markers':sum(speaker_ids.values()),'report':str(target),'sha256':digest(target)}))

if __name__=='__main__':main()
