"""Execute original full Mover order/ground-step routines with explicit world seams.

Original arithmetic/control flow, vector equality and facing setter execute.
Activation, spatial collision, target shape tests, scheduler/Mind notifications
and terrain height are synthetic fixture boundaries, never simulated gameplay.
"""
from pathlib import Path
import argparse, copy, hashlib, json, math, random, struct, sys

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
sys.path.insert(0, str(ROOT / 'research/phase3-terrain'))
from terrain_uv_oracle import Pe, ORIGINAL_SHA, ACCEPTED_SHA
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import *

OUT = Path(__file__).parent
M, A, CH, MIND, VT, W, TERRAIN, CACHE, VALUE, MAP = [0x2000000 + n * 0x1000 for n in range(1, 11)]
TARGET, REGISTRY, STOP = 0x2010000, 0x2100000, 0x2200000
STUB, FLOAT_RETURN, FLOAT_VALUE = 0x2230000, 0x2231000, 0x2232000
F32_FIELDS = {'destination': (0x10, 3), 'last_step_speed': (0x1c, 1), 'speed': (0x20, 1),
              'requested_speed': (0x24, 1), 'acceleration': (0x2c, 1), 'turn_rate': (0x34, 1),
              'desired_direction': (0x38, 3), 'target_range': (0x48, 1), 'speed_factor': (0x50, 1)}
BOOL_FIELDS = {'ignore_terrain_speed_limit': 0x4c, 'instant_turn': 0x54, 'turning': 0x55, 'falling': 0x56}


class Machine:
    def __init__(self, pe, case):
        self.c = case
        self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        pe.load(self.u)
        self.u.mem_map(0, 0x1000)
        self.u.mem_map(0x2000000, 0x400000)
        self.u.mem_map(0x3000000, 0x10000)
        self.u.reg_write(UC_X86_REG_FPCW, {'Single24': 0x007f, 'Double53': 0x027f, 'Extended64': 0x037f}[case['context']['precision']])
        self.events = []
        self.turning_trace = []
        self.u.mem_write(FLOAT_RETURN, b'\xd9\x05' + struct.pack('<I', FLOAT_VALUE) + b'\xc2\x08\x00')
        self.put(M, VT); self.put(M + 4, A); self.put(A + 0x68, CH if case['actor']['has_character'] else 0)
        self.put(A + 0x70, MIND if case['actor']['has_mind'] else 0); self.put(MIND, VT)
        for offset, index in [(0x38, 0), (0x12c, 1), (0x130, 2), (0x134, 3), (0x154, 4)]:
            self.put(VT + offset, STUB + index * 0x10)
        self.put(0xb36534, W); self.put(W + 0x30, MAP)
        self.floats(W + 0xe0, 125.0); self.floats(W + 0xe8, case['context']['world_delta'])
        self.put(0xb364c0, REGISTRY); self.put(A + 0xf8, TERRAIN)
        self.floats(TERRAIN + 0x38, case['actor']['terrain_speed_limit'])
        self.put(0xb3d694, 1); self.put(0xb3d690, VALUE + 4)
        self.floats(VALUE + 4, case['context']['terrain_minimum_move_speed'])
        self.put(0xb39ab0, CACHE); self.put(CACHE + 0x54, VALUE)
        self.floats(VALUE, case['precision_radius'])
        if case.get('turning_fixture'):
            # Tables come from actual native CRT constructors/FSIN/FCOS, not
            # Python trig generation. Real lookup/asin/rotation code executes.
            doc = json.loads((ROOT / 'research/phase4-trig/native-trig-vectors.json').read_text())
            assert doc['original_sha256'] == ORIGINAL_SHA and doc['tables_identical_across_precision_modes']
            for name, obj, pointer, global_slot in [('cos', 0xb39a48, 0x2240000, 0xb39b08),
                                                    ('sin', 0xb39a60, 0x2250000, 0xb39b10)]:
                table = doc['tables'][name]; assert len(table) == 4096
                self.put(pointer, *table); self.put(obj + 4, 4096, pointer, 0x40c90fdb)
                self.u.mem_write(obj + 0x10, b'\x01'); self.put(global_slot, obj)
        for name, (offset, count) in F32_FIELDS.items():
            values = case['mover'][name]
            self.floats(M + offset, *(values if count > 1 else [values]))
        for name, offset in BOOL_FIELDS.items(): self.u.mem_write(M + offset, bytes([case['mover'][name]]))
        self.put(M + 0x44, case['mover']['target_id'])
        self.put(A + 8, case['actor']['flags'])
        self.floats(A + 0x20, *case['actor']['position']); self.floats(A + 0x2c, *case['actor']['facing'])
        self.u.mem_write(CH + 0x24, bytes([case['actor']['character_stopped']]))
        if case['target_valid']:
            handle = case['mover']['target_id']; slot = handle & 0xffff
            self.u.mem_write(REGISTRY + 4 + slot * 2, struct.pack('<H', handle >> 16))
            self.put(REGISTRY + 0x20004 + slot * 4, TARGET)
            self.floats(TARGET + 0x20, *case['target_position'], 0)
        self.u.hook_add(UC_HOOK_CODE, self.hook)

    def put(self, address, *values): self.u.mem_write(address, struct.pack('<' + 'I' * len(values), *values))
    def floats(self, address, *values): self.u.mem_write(address, struct.pack('<' + 'f' * len(values), *values))
    def ints(self, address, count=1): return list(struct.unpack('<' + 'I' * count, self.u.mem_read(address, 4 * count)))
    def f(self, address, count=1): return list(struct.unpack('<' + 'f' * count, self.u.mem_read(address, 4 * count)))
    def args(self, count=1): return self.ints(self.u.reg_read(UC_X86_REG_ESP) + 4, count)
    def ret(self, pop=0, value=1):
        esp = self.u.reg_read(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EIP, self.ints(esp)[0])
        self.u.reg_write(UC_X86_REG_ESP, esp + 4 + pop)
        self.u.reg_write(UC_X86_REG_EAX, value)
    def event(self, name, bits=None): self.events.append({'name': name, 'bits': bits or []})

    def hook(self, uc, address, size, _):
        if address == STOP: uc.emu_stop(); return
        if address in (0x445400, 0x96d780):
            self.turning_trace.append({'entry': hex(address), 'precision_flag': bool(uc.reg_read(UC_X86_REG_FPSW) & 0x20)})
        ecx = uc.reg_read(UC_X86_REG_ECX)
        if address == STUB:
            self.event('Activate'); self.ret(value=int(self.c['activation'])); return
        if address in (STUB + 0x10, STUB + 0x20, STUB + 0x30):
            self.event({STUB + 0x10: 'DestinationReached', STUB + 0x20: 'TargetInRange', STUB + 0x30: 'TurnCompleted'}[address])
            self.ret(value=int(self.c['notify_result'])); return
        if address == STUB + 0x40:
            args = self.args(3); self.event('Blocked', args[1:3]); self.ret(12); return
        if address in (0x50ac00, 0x50b250, 0x4db090):
            self.event({0x50ac00: 'ScheduleImmediate', 0x50b250: 'CancelScheduled', 0x4db090: 'DeactivateMover'}[address])
            self.ret(4 if address == 0x50ac00 else 0); return
        if address == 0x4ea0f0:
            dest = self.args()[0]; self.u.mem_write(dest, bytes(24)); self.ret(4, dest); return
        if address == 0x46f100: self.ret(12, int(self.c['shape_in_range'])); return
        if address == 0x4d0720:
            self.event('SpeedChanged'); self.ret(value=0); return
        if address == 0x554350:
            self.u.mem_write(ecx, bytes(0x58)); self.ret(8, ecx); return
        if address == 0x553ba0:
            result, request = self.args(2)
            self.event('CollisionQuery', self.ints(request + 0x4c, 2))
            self.u.mem_write(result, bytes(24)); self.put(result, int(self.c['collision']))
            self.ret(8, result); return
        if address == 0x4daf20: self.event('CharacterStop')
        if address == 0x4daf40: self.event('CharacterResume')
        if address == 0x4eebe0: self.event('SetFacing', self.ints(self.args()[0], 2))
        if address == 0x4ee9f0: self.ret(8); return
        if address == 0x4eeb40: self.event('MoveOnGround', self.args(2))
        if address == 0x4f0e50:
            position, flags = self.args(2)
            # Deliberately nonconstant terrain seam: catches which XY the
            # original helper queries before committing the new coordinates.
            x, y = self.f(position, 2)
            self.floats(FLOAT_VALUE, x * 0.125 + y * 0.25)
            uc.reg_write(UC_X86_REG_EIP, FLOAT_RETURN); return
        if address == 0x4ee5c0:
            self.u.mem_write(ecx + 0x20, bytes(self.u.mem_read(self.args()[0], 12)))
            self.ret(4); return

    def run(self):
        op = self.c['operation']
        if op == 'begin':
            r = self.c['order']; values = r['direction'] + r['destination'] + [r['requested_speed'], r['speed_factor']]
            arguments = list(struct.unpack('<6I', struct.pack('<6f', *values))); entry = 0x5773c0
        elif op == 'stop': arguments = [int(self.c['keep_scheduled']), int(self.c['reset_speed'])]; entry = 0x5767c0
        else: arguments = list(struct.unpack('<I', struct.pack('<f', self.c['context']['acceleration_delta']))); entry = 0x5774a0
        stack = 0x3008000
        self.put(stack, STOP, *arguments); self.u.reg_write(UC_X86_REG_ESP, stack); self.u.reg_write(UC_X86_REG_ECX, M)
        preserved = [(UC_X86_REG_EBX, 0x11223344), (UC_X86_REG_ESI, 0x33445566), (UC_X86_REG_EDI, 0x55667788), (UC_X86_REG_EBP, 0x778899aa)]
        for register, value in preserved: self.u.reg_write(register, value)
        self.u.emu_start(entry, STOP, count=100000)
        assert self.u.reg_read(UC_X86_REG_EIP) == STOP
        assert self.u.reg_read(UC_X86_REG_ESP) == stack + 4 * (1 + len(arguments))
        assert all(self.u.reg_read(reg) == value for reg, value in preserved)
        state = {name: self.ints(M + offset, count) for name, (offset, count) in F32_FIELDS.items()}
        state.update({name: bool(self.u.mem_read(M + offset, 1)[0]) for name, offset in BOOL_FIELDS.items()})
        state['target_id'] = self.ints(M + 0x44)[0]
        output = {'mover_bits': state, 'position_bits': self.ints(A + 0x20, 3), 'facing_bits': self.ints(A + 0x2c, 3),
                'character_stopped': bool(self.u.mem_read(CH + 0x24, 1)[0]),
                'native_success': bool(self.u.reg_read(UC_X86_REG_EAX) & 255), 'events': self.events,
                'abi_preserved': True}
        if self.turning_trace: output['turning_trace'] = self.turning_trace
        return output


def cases(include_turning=False):
    base = {'name': 'straight', 'operation': 'step',
            'mover': {'destination': [10., 0., 0.], 'last_step_speed': 2., 'speed': 2., 'requested_speed': 3.,
                      'acceleration': 0., 'turn_rate': 0., 'desired_direction': [1., 0., 0.], 'target_id': 0,
                      'target_range': 1., 'ignore_terrain_speed_limit': False, 'speed_factor': 1.,
                      'instant_turn': False, 'turning': True, 'falling': False},
            'actor': {'position': [0., 0., 0.], 'facing': [1., 0., 0.], 'flags': 0,
                      'has_mind': True, 'has_character': True, 'character_stopped': False, 'terrain_speed_limit': 0.},
            'context': {'acceleration_delta': 0.0625, 'world_delta': 0.0625, 'terrain_minimum_move_speed': 0.5, 'precision': 'Extended64'},
            'precision_radius': 0.1, 'activation': True, 'collision': False, 'target_valid': False,
            'target_position': [10., 0.], 'shape_in_range': False, 'notify_result': True,
            'order': {'direction': [1., 0.], 'destination': [8., 3.], 'requested_speed': 4., 'speed_factor': 0.75},
            'keep_scheduled': False, 'reset_speed': True}
    rows = []
    def add(name, **parts):
        row = copy.deepcopy(base); row['name'] = name
        for key, value in parts.items():
            if isinstance(value, dict): row[key].update(value)
            else: row[key] = value
        rows.append(row)
    add('straight')
    add('already_at_destination', mover={'destination': [0., 0., 0.]})
    add('already_at_destination_no_mind', mover={'destination': [0., 0., 0.]}, actor={'has_mind': False})
    add('collision', collision=True)
    add('collision_no_components', collision=True, actor={'has_mind': False, 'has_character': False})
    add('bypass_collision', collision=True, actor={'flags': 1 << 18})
    add('resume_character', actor={'character_stopped': True})
    add('accelerate', mover={'acceleration': 2., 'speed': 0.})
    add('accelerate_clamp', mover={'acceleration': 2., 'speed': 2.95})
    add('decelerate_is_immediate', mover={'acceleration': 0.1, 'speed': 8.})
    add('terrain_cap', actor={'terrain_speed_limit': 1.})
    add('terrain_floor', actor={'terrain_speed_limit': 0.25})
    add('ignore_terrain_cap', actor={'terrain_speed_limit': 0.25}, mover={'ignore_terrain_speed_limit': True})
    add('speed_factor', mover={'speed_factor': 0.25})
    add('cross_destination', mover={'destination': [0.05, 0., 0.]})
    add('cross_destination_no_mind', mover={'destination': [0.05, 0., 0.]}, actor={'has_mind': False})
    add('target_before_move', mover={'target_id': 0x10007}, target_valid=True, shape_in_range=True, notify_result=False)
    add('target_after_move', mover={'target_id': 0x10007}, target_valid=True, target_position=[0.5, 0.])
    add('target_still_far', mover={'target_id': 0x10007}, target_valid=True)
    add('distinct_acceleration_and_world_delta', mover={'acceleration': 2., 'speed': 0.}, context={'acceleration_delta': 0.25, 'world_delta': 0.125})
    add('height_at_old_position', actor={'position': [4., 2., 0.]}, mover={'destination': [20., 2., 0.]})
    add('destination_behind_direction', mover={'destination': [-1., 0., 0.]})
    add('crossing_equality', mover={'destination': [0.1875, 0., 0.]})
    add('zero_speed_direct_routine', mover={'speed': 0., 'requested_speed': 0.})
    add('signed_zero_speed', mover={'speed': -0., 'requested_speed': -0., 'speed_factor': 1.})
    add('negative_factor_raw_domain', mover={'speed_factor': -0.25})
    add('negative_requested_speed_raw_domain', mover={'requested_speed': -2.})
    add('native_epsilon_near', mover={'destination': [0.009, 0., 0.]})
    add('native_epsilon_far', mover={'destination': [0.011, 0., 0.]})
    add('begin_far', operation='begin')
    add('begin_near_retains_requests', operation='begin', order={'destination': [0.01, 0.]})
    add('begin_radius_equality_is_far', operation='begin', order={'destination': [0.125, 0.]}, precision_radius=0.125)
    add('begin_falling_rejects_before_activation', operation='begin', mover={'falling': True})
    add('begin_activation_rejection', operation='begin', activation=False)
    for keep in (False, True):
        for reset in (False, True): add(f'stop_{keep}_{reset}', operation='stop', keep_scheduled=keep, reset_speed=reset)
    rng = random.Random(145773)
    for i in range(24):
        add(f'fractional_{i}', mover={'speed': rng.random() * 7, 'requested_speed': rng.random() * 7,
                                     'acceleration': rng.random() * 4, 'speed_factor': rng.random() * 1.3},
            context={'world_delta': rng.random() * .2, 'acceleration_delta': rng.random() * .2},
            actor={'position': [rng.random() * 2, 0., 0.]})
    if include_turning:
        for index, angle in enumerate((0.1, -0.1, 0.5, -0.5, 1.5, -1.5, 3., -3.)):
            direction = [math.cos(angle), math.sin(angle), 0.]
            for mode, rate, instant in [('gradual', 1., False), ('snap', 80., False), ('instant', 1., True), ('zero_rate', 0., False)]:
                add(f'turn_{index}_{mode}', turning_fixture=True,
                    mover={'desired_direction': direction, 'turn_rate': rate, 'instant_turn': instant})
        add('turn_axis_locked', turning_fixture=True, actor={'flags': 1 << 17},
            mover={'desired_direction': [0., 1., 0.], 'turn_rate': 1.})
        add('turn_collision', turning_fixture=True, collision=True,
            mover={'desired_direction': [0., -1., 0.], 'turn_rate': 1.})
        add('turn_cross_without_equal_direction', turning_fixture=True,
            mover={'destination': [0.05, 0., 0.], 'desired_direction': [0., 1., 0.], 'turn_rate': 1.})
        add('turn_snap_no_mind', turning_fixture=True, actor={'has_mind': False},
            mover={'desired_direction': [0., 1., 0.], 'turn_rate': 80.})
        add('turn_prior_inexact_distance', turning_fixture=True,
            actor={'position': [0.1234567, 0.2345678, 0.], 'facing': [0.3, 0.7, 0.]},
            mover={'destination': [9.876543, 7.654321, 0.], 'desired_direction': [0.9, -0.1, 0.], 'turn_rate': 1.25})
    return [{**copy.deepcopy(row), 'name': row['name'] + '_' + pc,
             'context': {**row['context'], 'precision': pc}} for pc in ('Single24', 'Double53', 'Extended64') for row in rows]


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--include-turning', action='store_true'); args = parser.parse_args()
    pe = Pe(GAME / 'AA.exe', ORIGINAL_SHA)
    accepted = Pe(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', ACCEPTED_SHA)
    ranges = [(0x5767c0, 0x5767e6), (0x577330, 0x577372), (0x5773c0, 0x577495),
              (0x5774a0, 0x577a6c), (0x4515f0, 0x451626), (0x4eeb40, 0x4eec45)]
    for a, z in ranges: assert pe.at(a, z-a) == accepted.at(a, z-a)
    rows = []
    for case in cases(args.include_turning):
        try: result = Machine(pe, case).run()
        except Exception:
            print('FAILED', case['name'], flush=True); raise
        rows.append({'input': case, 'output': result})
    report = {'schema': 1, 'passed': True, 'original_sha256': ORIGINAL_SHA,
              'accepted_v14_sha256': ACCEPTED_SHA, 'case_count': len(rows), 'fixtures': rows,
              'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'seams': ['component activation', 'target-shape range', 'collision world query', 'scheduler and Mind callbacks',
                        'synthetic terrain height and spatial commit'],
              'limits': ['No complete pathfinding or world implementation is implied.'],
              'includes_native_turning': args.include_turning}
    if args.include_turning:
        report['trig_fixture_sha256'] = hashlib.sha256((ROOT / 'research/phase4-trig/native-trig-vectors.json').read_bytes()).hexdigest()
    path = OUT / ('native-movement-turning-fixtures.json' if args.include_turning else 'native-movement-fixtures.json')
    path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': True, 'cases': len(rows), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__': main()
