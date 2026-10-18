"""Read-only original x86 renderer-facing matrix audit; no game process is run."""
from pathlib import Path
import hashlib
import json
import math
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-rng'))
from original_rng import Machine, original, modified
from unicorn.x86_const import UC_X86_REG_ESP
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sha = lambda b: hashlib.sha256(b).hexdigest()
RANGES = {
    'default_facing_registration': (0x402913, 0x402975),
    'view_copies_actor_facing': (0x45b530, 0x45b5d0),
    'render_model_facing_dispatch': (0x715bc0, 0x715bf2),
    'facing_to_node_matrix': (0x8715f0, 0x87165e),
    'matrix_from_direction': (0x70d270, 0x70d340),
    'normalized_cross_product': (0x436310, 0x4363be),
}


def main():
    for start, end in RANGES.values():
        assert original.range(start, end-start) == modified.range(start, end-start)
    data = json.loads((ROOT.parent / 'reconstruction/game/data/map-actors/Campaign/axis01_crete.json').read_text())
    selections = [{'kind': 'cardinal', 'instance_id': None, 'heading': v}
                  for v in [[0., -1., 0.], [1., 0., 0.], [0., 1., 0.], [-1., 0., 0.]]]
    selected_types = ['dirt_road_straight', 'dirt_road_bend', 'dirt_road_curve', 'dirt_road_end',
                      'gbr_corps', 'gbr_depot_oil', 'gbr_brigade_infantry',
                      'regiment_gbr_infantry', 'regiment_ger_hq_airborne_unpacked',
                      'med_villageblock12x12_01', 'street_lights', 'chaparral_trees']
    for kind in selected_types:
        seen = set()
        for actor in data['actors']:
            v = actor['orientation']
            if actor['thingId'] != kind or None in v or tuple(v) in seen or v[2] != 0:
                continue
            seen.add(tuple(v))
            selections.append({'kind': kind, 'instance_id': actor['instanceId'], 'heading': v})
            if len(seen) == 8:
                break
    cases = []
    for label, cw in [('Single24', 0x007f), ('Double53', 0x027f), ('Extended64', 0x037f)]:
        machine = Machine(cw)
        for selection in selections:
            machine.reset(0)
            machine.u.mem_write(machine.STATE, bytes(0x100))
            machine.u.mem_write(machine.STACK+4, struct.pack('<I3f', machine.STATE, *selection['heading']))
            machine.u.emu_start(0x8715f0, machine.STOP, count=2000)
            assert machine.u.reg_read(UC_X86_REG_ESP) == machine.STACK+4
            raw = bytes(machine.u.mem_read(machine.STATE+0x38, 36))
            matrix = struct.unpack('<9f', raw)
            x, y, _ = selection['heading']
            yaw = math.atan2(x, -y)
            # Ni's row-major native XYZ rotation and the equivalent render-Y yaw.
            expected = [math.cos(yaw), -math.sin(yaw), 0., math.sin(yaw), math.cos(yaw), 0., 0., 0., 1.]
            error = max(abs(a-b) for a,b in zip(matrix, expected))
            assert error < 0.00001, (selection, matrix, expected, error)
            cases.append({**selection, 'precision': label, 'native_matrix_bits': list(struct.unpack('<9I',raw)),
                          'render_yaw_radians': yaw, 'max_planar_matrix_error': error})
    roads = [a for a in data['actors'] if a['thingId'] == 'dirt_road_straight'][:3]
    report = {'schema': 1, 'source_sha256': sha(original.b), 'modified_sha256': sha(modified.b),
              'producer_sha256': sha(Path(__file__).read_bytes()),
              'method': 'Execute unchanged 0x8715f0 -> 0x70d270 -> 0x436310 with no hooks; explicit x87 precision modes.',
              'ranges': {name: {'start': hex(a), 'end_exclusive': hex(b), 'sha256': sha(original.range(a,b-a))}
                         for name,(a,b) in RANGES.items()},
              'default_forward': [0,-1,0], 'render_yaw': 'atan2(native_heading_x, -native_heading_y)',
              'road_example': [{'instance_id': a['instanceId'], 'xy': [a['x'], a['y']],
                                'saved_heading': a['orientation'], 'old_yaw': a['yaw'],
                                'corrected_yaw': math.atan2(a['orientation'][0], -a['orientation'][1])} for a in roads],
              'cases': cases,
              'limits': ['Planar map placement orientation only; pitch/roll and full live orientation interpolation remain unimplemented.',
                         'Yaw uses a normalized planar direction; the native direction column retains small serialized normalization error.',
                         'No claim of native frame image parity; this verifies original matrix orientation and source placement inputs.']}
    HERE.mkdir(exist_ok=True)
    (HERE/'native-placement-basis.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    cs = Cs(CS_ARCH_X86,CS_MODE_32)
    lines = []
    for name,(a,b) in RANGES.items():
        lines.append(name)
        lines.extend(f'{i.address:08x} {i.mnemonic} {i.op_str}' for i in cs.disasm(original.range(a,b-a),a))
    (HERE/'native-placement-basis-assembly.txt').write_text('\n'.join(lines),encoding='utf-8')
    print(json.dumps({'cases':len(cases),'max_error':max(c['max_planar_matrix_error'] for c in cases),
                      'report_sha256':sha((HERE/'native-placement-basis.json').read_bytes())}))


if __name__ == '__main__':
    main()
