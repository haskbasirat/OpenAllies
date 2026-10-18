"""Convert the installed Axis & Allies Gamebryo meshes to browser-readable JSON.

Requires PyFFI 2.2.3 in convert_models_vendor (or the Python environment).
Input assets are read-only. The initial export is the model's authored pose;
keyframe animation and game-specific shader programs are not yet reconstructed.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import json
import logging
import math
from pathlib import Path
import struct
import sys
import time

sys.path.insert(0, str(Path(__file__).with_name('convert_models_vendor')))
# PyFFI predates Python 3.8, which removed time.clock.
if not hasattr(time, 'clock'):
    time.clock = time.perf_counter
from pyffi.formats.nif import NifFormat

# PyFFI 2.2.3 binds the repeated Num UV Sets XML field to the newer byte type,
# even for legacy 4.2.2.0 files whose on-disk field is a ushort. The supplied
# Waves models provide a direct fixture: four vertices, one UV set, two triangles.
_geometry_read = NifFormat.NiGeometryData.read
def _read_geometry_legacy(self, stream, data):
    if data.version > 0x04020200:
        return _geometry_read(self, stream, data)
    for attr in self._get_filtered_attribute_list(data):
        if attr.is_abstract:
            continue
        if attr.name == 'num_uv_sets':
            self.num_uv_sets = struct.unpack(data._byte_order + 'H', stream.read(2))[0] & 63
            continue
        attr_value = getattr(self, '_' + attr.name + '_value_')
        attr_value.arg = attr.arg if isinstance(attr.arg, (int, type(None))) else getattr(self, attr.arg)
        attr_value.read(stream, data)
for _nif_type_name in dir(NifFormat):
    _nif_type = getattr(NifFormat, _nif_type_name)
    if isinstance(_nif_type, type) and issubclass(_nif_type, NifFormat.NiGeometryData):
        _nif_type.read = _read_geometry_legacy

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / 'assets' / 'original' / 'Data'
OUTPUT = ROOT / 'assets' / 'converted' / 'models'
TEXTURE_ALIASES = {
    # Stale pluralization in original NIFs; matching atlas is beside each mesh.
    'savannahrocksbase.dds': 'savannahrockbase.dds',
    'temperatetreebase.dds': 'temperatetreesbase.dds',
}
logging.getLogger('pyffi').setLevel(logging.ERROR)


def string(value):
    return value.decode('utf-8', errors='replace') if isinstance(value, bytes) else str(value)


def round_float(value):
    value = float(value)
    if not math.isfinite(value):
        raise ValueError('Non-finite mesh coordinate')
    return round(value, 6)


def y_up(vector):
    return [round_float(vector.x), round_float(vector.z), round_float(-vector.y)]


def transformed_values(vectors, transform, translation=True):
    # Avoid allocating XML-backed Vector3 and Matrix33 objects for each vertex.
    m = transform.as_list()
    offset = m[3][:3] if translation else [0, 0, 0]
    result = []
    for vector in vectors:
        x, y, z = vector.x, vector.y, vector.z
        tx = x*m[0][0] + y*m[1][0] + z*m[2][0] + offset[0]
        ty = x*m[0][1] + y*m[1][1] + z*m[2][1] + offset[1]
        tz = x*m[0][2] + y*m[1][2] + z*m[2][2] + offset[2]
        result.extend((round_float(tx), round_float(tz), round_float(-ty)))
    return result


def model_summary(result):
    relative = Path(result['source'])
    meshes = result['meshes']
    return {'source': relative.as_posix(), 'url': '/assets/converted/models/' + relative.with_suffix('.json').as_posix(),
            'name': relative.stem, 'meshCount': len(meshes),
            'vertices': sum(len(m['positions']) // 3 for m in meshes),
            'triangles': sum(len(m['indices']) // 3 for m in meshes), 'bounds': result['bounds']}


class Converter:
    def __init__(self):
        self.files = {p.relative_to(ORIGINAL).as_posix().lower(): p
                      for p in ORIGINAL.rglob('*') if p.is_file()}
        self.textures_by_name = defaultdict(list)
        for rel, path in self.files.items():
            if path.suffix.lower() in ('.dds', '.tga', '.png', '.bmp', '.jpg'):
                self.textures_by_name[path.name.lower()].append(path)
        self.missing_textures = set()

    def resolve_texture(self, model, filename):
        raw = string(filename).replace('\\', '/').lstrip('/')
        if not raw:
            return None
        local = model.parent / raw
        try:
            rel = local.relative_to(ORIGINAL).as_posix().lower()
        except ValueError:
            rel = raw.lower()
        resolved = self.files.get(rel) or self.files.get(raw.lower())
        if not resolved:
            candidates = self.textures_by_name.get(raw.rsplit('/', 1)[-1].lower(), [])
            if not candidates:
                alias = TEXTURE_ALIASES.get(raw.rsplit('/', 1)[-1].lower())
                if alias:
                    candidates = self.textures_by_name.get(alias, [])
            # Prefer the longest common directory prefix; climate variants stay local.
            model_parts = model.relative_to(ORIGINAL).parts
            def rank(path):
                matching = 0
                for a, b in zip(model_parts, path.relative_to(ORIGINAL).parts):
                    if a.lower() != b.lower():
                        break
                    matching += 1
                return (-matching, len(path.parts), path.as_posix().lower())
            if candidates:
                resolved = min(candidates, key=rank)
        if not resolved:
            self.missing_textures.add(raw)
            return None
        if resolved.suffix.lower() in ('.png', '.jpg'):
            return '/assets/original/Data/' + resolved.relative_to(ORIGINAL).as_posix()
        return '/assets/converted/textures/' + resolved.relative_to(ORIGINAL).with_suffix('.png').as_posix()

    def material(self, model, properties):
        mat = {'color': [1, 1, 1], 'opacity': 1, 'doubleSided': False}
        uv_set = 0
        for p in properties:
            if isinstance(p, NifFormat.NiMaterialProperty):
                mat['name'] = string(p.name)
                mat['color'] = [round_float(p.diffuse_color.r), round_float(p.diffuse_color.g), round_float(p.diffuse_color.b)]
                mat['opacity'] = round_float(p.alpha)
                mat['emissive'] = [round_float(p.emissive_color.r), round_float(p.emissive_color.g), round_float(p.emissive_color.b)]
            elif isinstance(p, NifFormat.NiTexturingProperty):
                if p.has_base_texture:
                    # Retain authored descriptor words even when its source link
                    # is absent. These are metadata, not preview sampler defaults.
                    mat['baseTextureState'] = {
                        'clampMode': int(p.base_texture.clamp_mode),
                        'filterMode': int(p.base_texture.filter_mode),
                        'uvSet': int(p.base_texture.uv_set),
                    }
                if p.has_base_texture and p.base_texture.source:
                    mat['texture'] = self.resolve_texture(model, p.base_texture.source.file_name)
                    mat['sourceTexture'] = string(p.base_texture.source.file_name)
                    uv_set = p.base_texture.uv_set
            elif isinstance(p, NifFormat.NiAlphaProperty):
                mat['alphaBlend'] = bool(p.flags & 1)
                mat['alphaTest'] = round_float(p.threshold / 255) if p.flags & 512 else 0
            elif isinstance(p, NifFormat.NiStencilProperty):
                mat['doubleSided'] = int(p.draw_mode) == 3
        return mat, uv_set

    def convert(self, path, data=None, write=True):
        if data is None:
            data = NifFormat.Data()
            with path.open('rb') as stream:
                data.read(stream)
        meshes = []
        skipped = []
        seen = set()

        def walk(node, matrix=None, inherited=None, hierarchy=(), inherited_team_mask=False):
            if node is None or not isinstance(node, NifFormat.NiAVObject) or id(node) in seen:
                return
            seen.add(id(node))
            name = string(node.name)
            hierarchy = hierarchy + (name,)
            transform = node.get_transform()
            if matrix is not None:
                transform *= matrix
            properties = dict(inherited or {})
            for p in node.properties:
                if p:
                    properties[p.__class__.__name__] = p
            hidden = bool(node.flags & 1)
            extras = [string(x.name) for x in getattr(node, 'extra_data_list', []) if x]
            team_mask = inherited_team_mask or 'RepColor' in extras
            shadow = 'Sg' in extras or 'Sgv' in extras or ' sg' in name.lower()
            selection = name.lower().startswith(('selectionbox', 'selection box', 'collision'))
            if isinstance(node, NifFormat.NiTriBasedGeom) and node.data:
                if hidden or shadow or selection:
                    skipped.append({'name': name, 'reason': 'helper/shadow/hidden'})
                    return
                meshdata = node.data
                if node.is_skin():
                    vertices, normals = node.get_skin_deformation()
                else:
                    vertices, normals = meshdata.vertices, meshdata.normals
                positions = transformed_values(vertices, transform)
                _, rotation, _ = transform.get_scale_rotation_translation()
                normal_values = []
                if meshdata.has_normals:
                    normal_values = transformed_values(normals, rotation, translation=False)
                material, uv_set = self.material(path, properties.values())
                material['teamColorMask'] = team_mask
                uv_values = []
                if meshdata.uv_sets:
                    uv_set = min(uv_set, len(meshdata.uv_sets) - 1)
                    uv_values = [v for uv in meshdata.uv_sets[uv_set] for v in [round_float(uv.u), round_float(1 - uv.v)]]
                indices = [int(i) for triangle in meshdata.get_triangles() for i in triangle]
                if any(index < 0 or index >= len(vertices) for index in indices):
                    raise ValueError('Triangle index outside vertex buffer')
                if positions and indices:
                    meshes.append({'name': name, 'hierarchy': list(hierarchy), 'positions': positions,
                                   'normals': normal_values, 'uvs': uv_values, 'indices': indices,
                                   'material': material, 'skinnedPose': bool(node.is_skin())})
            children = list(getattr(node, 'children', []))
            # The nearest authored LOD is the faithful full-detail appearance.
            if isinstance(node, NifFormat.NiLODNode) and children:
                children = children[:1]
            for child in children:
                walk(child, transform, properties, hierarchy, team_mask)

        for root in data.roots:
            walk(root)
        if not meshes:
            raise ValueError('No visible triangle geometry')
        all_positions = [mesh['positions'] for mesh in meshes]
        minimum = [min(min(values[axis::3]) for values in all_positions) for axis in range(3)]
        maximum = [max(max(values[axis::3]) for values in all_positions) for axis in range(3)]
        relative = path.relative_to(ORIGINAL)
        bounds = {'min': minimum, 'max': maximum,
                  'size': [round_float(maximum[i] - minimum[i]) for i in range(3)]}
        result = {'format': 'aa-nif-mesh-v1', 'source': relative.as_posix(),
                  'nifVersion': '.'.join(str((data.version >> n) & 255) for n in (24, 16, 8, 0)),
                  'upAxis': 'Y', 'pose': 'authored static pose', 'bounds': bounds,
                  'meshes': meshes, 'skippedHelpers': skipped}
        if not write:
            return result
        target = OUTPUT / relative.with_suffix('.json')
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(result, separators=(',', ':')), encoding='utf-8')
        return model_summary(result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--match', help='Case-insensitive substring of original relative path')
    parser.add_argument('--limit', type=int)
    parser.add_argument('--force', action='store_true', help='Rebuild existing exports')
    args = parser.parse_args()
    converter = Converter()
    paths = [p for rel, p in converter.files.items() if p.suffix.lower() == '.nif'
             and (not args.match or args.match.lower() in rel)]
    def priority(p):
        rel = p.relative_to(ORIGINAL).as_posix().lower()
        category = 0 if rel.startswith('units/') else 1 if rel.startswith('buildings/') else 2
        preferred = 0 if any(s in p.stem.lower() for s in ('infantry', 'tankmedium', 'truck', 'hq')) else 1
        return category, preferred, rel
    paths.sort(key=priority)
    if args.limit:
        paths = paths[:args.limit]
    manifest_path = OUTPUT / 'manifest.json'
    previous = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    records = {m['source']: m for m in previous.get('models', [])}
    failures = []
    started = time.perf_counter()
    for i, path in enumerate(paths):
        source = path.relative_to(ORIGINAL).as_posix()
        target = OUTPUT / Path(source).with_suffix('.json')
        if not args.force and target.exists():
            if source in records:
                continue
            try:
                records[source] = model_summary(json.loads(target.read_text(encoding='utf-8')))
                continue
            except (ValueError, KeyError):
                pass
        try:
            record = converter.convert(path)
            records[source] = record
        except Exception as exc:
            failures.append({'source': source, 'error': str(exc), 'type': type(exc).__name__})
        if i % 50 == 0 or i + 1 == len(paths):
            print(f'{i + 1}/{len(paths)}: {len(records)} converted, {len(failures)} unsupported, {time.perf_counter() - started:.1f}s', flush=True)
    models = sorted(records.values(), key=lambda m: m['source'].lower())
    # Resume runs must still report texture gaps discovered in earlier batches.
    for model in models:
        exported = json.loads((OUTPUT / Path(model['source']).with_suffix('.json')).read_text(encoding='utf-8'))
        for mesh in exported['meshes']:
            material = mesh['material']
            if material.get('sourceTexture') and not material.get('texture'):
                converter.missing_textures.add(material['sourceTexture'])
    by_path = {}
    by_name = {}
    for model in models:
        by_path[model['source'].lower()] = model['url']
        by_path[str(Path(model['source']).with_suffix('')).replace('\\', '/').lower()] = model['url']
        by_name.setdefault(model['name'].lower(), model['url'])
    manifest = {'format': 'aa-model-manifest-v1', 'modelCount': len(models),
                'models': models, 'byPath': by_path, 'byName': by_name,
                'failures': failures, 'missingTextures': sorted(converter.missing_textures)}
    OUTPUT.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    print(f'Saved {manifest_path}', flush=True)


if __name__ == '__main__':
    main()
