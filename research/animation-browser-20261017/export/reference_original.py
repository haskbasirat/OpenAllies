"""Independent raw-NIF -> PyFFI hierarchy/skin reference for rich preview.

Curve equations use the separately executed AA primitive proof (Hermite uses
left.backward/right.forward WITHOUT a time-span factor). Source keys are read
again from NIF, never from cached curves. Geometry/skin math is PyFFI, not Rust
glam. This is differential inspector evidence, not whole-Gamebryo parity.
"""
from __future__ import annotations
import argparse
import bisect
import json
import math
from pathlib import Path
import struct

import export_browser as export

ORDERS = ['XYZ', 'XZY', 'YZX', 'YXZ', 'ZXY', 'ZYX', 'XYX', 'YZY', 'ZXZ']


def scalar_value(value):
    if isinstance(value, (int, float)):
        return float(value)
    if hasattr(value, 'w'):
        return [float(value.x), float(value.y), float(value.z), float(value.w)]
    return [float(value.x), float(value.y), float(value.z)]


def blend(a, b, c, d, coefficients):
    if isinstance(a, list):
        return [sum(co * value for co, value in zip(coefficients, values))
                for values in zip(a, b, c, d)]
    return sum(co * value for co, value in zip(coefficients, (a, b, c, d)))


def raw_sample(group, kind, time, quaternion=False):
    keys = list(group)
    if not keys:
        return None
    next_key = bisect.bisect_right([float(k.time) for k in keys], time)
    if next_key == 0:
        return scalar_value(keys[0].value)
    if next_key == len(keys):
        return scalar_value(keys[-1].value)
    left, right = keys[next_key - 1], keys[next_key]
    u = (time - float(left.time)) / (float(right.time) - float(left.time))
    a, b = scalar_value(left.value), scalar_value(right.value)
    if quaternion:
        if kind != 1:
            raise ValueError('reference requires separately proved nonlinear quaternion implementation')
        dot = sum(v * w for v, w in zip(a, b))
        if dot < 0:
            b, dot = [-v for v in b], -dot
        dot = max(-1., min(1., dot))
        if dot < .9995:
            angle = math.acos(dot)
            denominator = math.sin(angle)
            value = [(math.sin((1-u)*angle)*v + math.sin(u*angle)*w)/denominator for v, w in zip(a, b)]
        else:
            value = [v + (w-v)*u for v, w in zip(a, b)]
        length = math.sqrt(sum(v*v for v in value))
        return [v/length for v in value]
    if kind == 1:
        if isinstance(a, list):
            return [v + (w-v)*u for v, w in zip(a, b)]
        return a + (b-a)*u
    if kind == 2:
        outgoing = scalar_value(left.backward)
        incoming = scalar_value(right.forward)
        return blend(a, b, outgoing, incoming,
                     (2*u*u*u-3*u*u+1, -2*u*u*u+3*u*u, u*u*u-2*u*u+u, u*u*u-u*u))
    raise ValueError('reference requires separately proved key type ' + str(kind))


def axis_matrix(axis, angle):
    s, c = math.sin(angle), math.cos(angle)
    rows = {'X': [[1,0,0],[0,c,s],[0,-s,c]],
            'Y': [[c,0,-s],[0,1,0],[s,0,c]],
            'Z': [[c,s,0],[-s,c,0],[0,0,1]]}[axis]
    from convert_models_animations import matrix
    return matrix(rows)


def apply_raw(data, nodes, objects, time):
    for node, obj in zip(nodes, objects):
        obj.translation.x, obj.translation.y, obj.translation.z = node['translation']
        obj.scale = node['scale']
        for i, row in enumerate(node['rotation']):
            for j, value in enumerate(row):
                setattr(obj.rotation, f'm_{i+1}{j+1}', value)
    targets = {id(obj): obj for obj in objects}
    for controller in data.blocks:
        if not isinstance(controller, export.NifFormat.NiKeyframeController) or id(controller.target) not in targets:
            continue
        obj, keydata = controller.target, controller.data
        if not isinstance(keydata, export.NifFormat.NiKeyframeData):
            continue
        t = min(float(controller.stop_time), max(float(controller.start_time), time))
        value = raw_sample(keydata.translations.keys, int(keydata.translations.interpolation), t)
        if value is not None:
            obj.translation.x, obj.translation.y, obj.translation.z = value
        scale = raw_sample(keydata.scales.keys, int(keydata.scales.interpolation), t)
        if scale is not None:
            obj.scale = scale
        if int(keydata.rotation_type) == 4:
            order = struct.unpack('<I', struct.pack('<f', float(keydata.unknown_float)))[0]
            values = [raw_sample(axis.keys, int(axis.interpolation), t) for axis in keydata.xyz_rotations]
            # Executed native 862FE0 stores zero for an empty axis count;
            # it does not retain that axis from the bind rotation.
            values = [0.0 if value is None else value for value in values]
            rotations = [axis_matrix(axis, angle) for axis, angle in zip(ORDERS[order], values)]
            obj.rotation = rotations[0] * rotations[1] * rotations[2]
        else:
            q = raw_sample(keydata.quaternion_keys, int(keydata.rotation_type), t, True)
            if q is not None:
                from convert_models_animations import quaternion_matrix
                obj.rotation = quaternion_matrix(q)


def reference_pose(nodes, meshes, objects):
    globals_ = []
    for node, obj in zip(nodes, objects):
        matrix = obj.get_transform()
        if node['parent'] is not None:
            matrix *= globals_[node['parent']]
        globals_.append(matrix)
    results = []
    for mesh in meshes:
        obj = mesh['_object']
        vertices, normals = obj.get_skin_deformation() if obj.is_skin() else (obj.data.vertices, obj.data.normals)
        matrix = globals_[mesh['node']]
        _, rotation, _ = matrix.get_scale_rotation_translation()
        positions, directions = [], []
        for vertex in vertices:
            point = vertex * matrix
            positions.append([float(point.x), float(point.z), float(-point.y)])
        for normal in normals:
            direction = normal * rotation
            length = math.sqrt(direction.x**2 + direction.y**2 + direction.z**2)
            directions.append([float(direction.x/length), float(direction.z/length), float(-direction.y/length)])
        results.append({'name': mesh['name'], 'positions': positions, 'normals': directions})
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=Path, default=export.HERE / 'hq-sample-v2')
    parser.add_argument('--output', type=Path, default=export.HERE / 'references')
    parser.add_argument('--match', default='CorpsDeath', help='source regular expression')
    args = parser.parse_args()
    args.cache = args.cache.resolve()
    args.output = args.output.resolve()
    import re
    manifest = json.loads((args.cache / 'animation-browser-manifest.json').read_text())
    document = json.loads(export.MATERIALS.read_text())
    converter = export.Converter()
    converter.audited_materials = document['models']
    records = []
    for rig in manifest['rigs']:
        if not re.search(args.match, rig['source'], re.I):
            continue
        data, nodes, meshes, objects, _, _ = export.geometry.collect_model(export.ORIGINAL / rig['source'], converter)
        clip = export.make_clip(rig['source'], data, nodes, objects, meshes)
        times = [fraction * clip['duration'] for fraction in (0,.25,.5,.75,1)]
        first = next(c for c in data.blocks if isinstance(c, export.NifFormat.NiKeyframeController))
        axis = list(first.data.xyz_rotations[0].keys)
        if len(axis) > 2:
            middle = float(axis[len(axis)//2].time)
            times += [max(0, middle-1e-5), middle, min(clip['duration'], middle+1e-5)]
        poses = []
        for time in sorted(set(times)):
            apply_raw(data, nodes, objects, time)
            poses.append({'clip': 0, 'time': time, 'meshes': reference_pose(nodes, meshes, objects)})
        relative = Path(rig['source']).with_suffix('.reference.json')
        path = args.output / relative
        export.write_new(path, {'source': rig['source'], 'source_sha256': rig['source_sha256'], 'poses': poses})
        records.append({'source': rig['source'], 'rig': str((args.cache / rig['file']).relative_to(export.HERE)).replace('\\','/'),
                        'reference': str(path.relative_to(export.HERE)).replace('\\','/'), 'reference_sha256': export.sha(path),
                        'poses': len(poses)})
        print(rig['source'], len(poses), flush=True)
    export.write_new(args.output / 'geometry-references.json', {'format': 'aa-animation-browser-geometry-reference-v1',
                     'generator_sha256': export.sha(Path(__file__)), 'records': records,
                     'scope': 'Raw original curve keys + separately native-proved Hermite/Euler primitives; independent PyFFI hierarchy/skin; no whole Gamebryo/native gameplay parity.'})


if __name__ == '__main__':
    main()
