"""Original structure enable/disable and command availability in isolated x86.

Executed unchanged: 565400 enable, 564DC0 disable, 565140 command recompute,
565090 upgrade availability, 585F00 settlement readiness, 4D14C0/4D0820 state
group, 517FA0 command bit set/clear (96D710 64-bit shift), 584E10 settlement
refresh, 584DE0 settlement queue clearing, 529C30.
Seams (recorded): 4D64F0 actor state (test double sets/clears the state
bit), actor virtuals +0x34/+0x100/+0x158/+0x15C, 4CEDA0 settlement center,
4CF110 modifier token, 4D1350 refresh, 589CC0 queue clearing. The
AllowSellUnderSiege option (4B2E60) is pre-initialised in its static cache.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, pin  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'enable_565400': (0x565400, 0x565442), 'disable_564DC0': (0x564DC0, 0x564E10),
          'recompute_565140': (0x565140, 0x5653F1), 'upgrade_available_565090': (0x565090, 0x565132),
          'settlement_ready_585F00': (0x585F00, 0x585F56), 'state_group_4D14C0': (0x4D14C0, 0x4D1503),
          'lowest_state_4D0820': (0x4D0820, 0x4D0855), 'command_bit_517FA0': (0x517FA0, 0x518004),
          'settlement_refresh_584E10': (0x584E10, 0x584E6A), 'settlement_clear_584DE0': (0x584DE0, 0x584E0C)}
COMP, ACTOR, AVT, AD = 0x2000000, 0x2000100, 0x2000800, 0x2001000
SCOMP, SACTOR, SAD, LIMITS, CENTER, TOKEN, OPTION = 0x2002000, 0x2002100, 0x2002200, 0x2002A00, 0x2003000, 0x2003800, 0x2003900
STRUCTS, SCOMPS, MEMBERS = 0x2010000, 0x2020000, 0x2030000
STUB_STATE, STUB_V100, STUB_IN, STUB_SETTLEMENT = 0x3000100, 0x3000110, 0x3000120, 0x3000130
GROUP_MASK = (1 << 0x1B) | (1 << 0x1C) | (1 << 0x1D) | (1 << 0x1E)


def S(i): return STRUCTS + 0x200 * i
def SC(i): return SCOMPS + 0x40 * i
def M(i): return MEMBERS + 0x40 * i


class StateMachine(Machine):
    def __init__(self, cw, op):
        super().__init__(cw)
        self.op = op
        self.trace = []
        self.put(COMP + 4, ACTOR); self.put(ACTOR, AVT); self.put(ACTOR + 4, AD)
        self.put(AVT + 0x34, STUB_STATE); self.put(AVT + 0x100, STUB_V100)
        self.put(AVT + 0x158, STUB_IN); self.put(AVT + 0x15C, STUB_SETTLEMENT)
        self.put(ACTOR + 0x100, op['states'] & 0xFFFFFFFF); self.put(ACTOR + 0x104, op['states'] >> 32)
        self.put(ACTOR + 0xF0, op['mask'] & 0xFFFFFFFF); self.put(ACTOR + 0xF4, op['mask'] >> 32)
        self.put(ACTOR + 0x90, 0x2004000 if op['c90'] else 0)
        self.put8(AD + 0x4E4, int(op['can_sell'])); self.put8(AD + 0x4E5, int(op['can_disable']))
        node = self.alloc(8) if op['upgrades'] else 0
        if node:
            self.put(node, 0x2F00000)
        self.put(AD + 0x4C0, node)
        self.put(0xB3AC40 + 0xD * 8, GROUP_MASK & 0xFFFFFFFF); self.put(0xB3AC44 + 0xD * 8, GROUP_MASK >> 32)
        self.put(0xB3D660, 1); self.put(0xB3D65C, OPTION); self.put8(OPTION, int(op['allow_sell_under_siege']))
        self.put(CENTER + 0x100, (1 << 30) if op['center_disabled'] else 0)
        self.put(TOKEN, op['token'] if op['token'] is not None else 0)
        s = op['settlement']
        self.put(SACTOR, AVT); self.put(SACTOR + 4, SAD); self.put(SACTOR + 0x98, SCOMP); self.put(SAD + 0x464, LIMITS)
        self.put(SCOMP + 4, SACTOR); self.put(SCOMP + 0x10, 1); self.put(LIMITS + 12, s['limit'])
        self.put(SCOMP + 0x14, CENTER if s['center'] else 0)
        data = self.alloc(4 * max(1, len(s['structures'])))
        for i, constructing in enumerate(s['structures']):
            self.put(data + 4 * i, S(i)); self.put(S(i) + 0x94, SC(i)); self.put(SC(i) + 4, S(i))
            self.put(S(i) + 0x100, (1 << 21) if constructing else 0)
        self.put(SCOMP + 0x18, data); self.put(SCOMP + 0x1C, s['count'])
        mdata = self.alloc(4 * max(1, len(s['members'])))
        self.put_words(mdata, [M(i) if present else 0 for i, present in enumerate(s['members'])])
        self.put(SCOMP + 0x24, mdata); self.put(SCOMP + 0x28, len(s['members']))
        self.seam(0x4D64F0, self._state_write)
        self.seam(STUB_STATE, self._state_active)
        self.seam(STUB_V100, self._flag('virtual_100'))
        self.seam(STUB_IN, self._flag('in_settlement'))
        self.seam(STUB_SETTLEMENT, lambda m, esp: (SACTOR, 0))
        self.seam(0x4CEDA0, self._center)
        self.seam(0x4CF110, self._token)
        self.seam(0x4D1350, self._refresh)
        self.seam(0x589CC0, self._clear)

    def name(self, p):
        if p == ACTOR: return 'actor'
        if p == SACTOR: return 'settlement'
        if p == CENTER: return 'center'
        if STRUCTS <= p < SCOMPS: return 's%d' % ((p - STRUCTS) // 0x200)
        if MEMBERS <= p < MEMBERS + 0x10000: return 'm%d' % ((p - MEMBERS) // 0x40)
        raise AssertionError(hex(p))

    def _state_write(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ACTOR
        state, enabled = self.read(esp + 4), bool(self.read(esp + 8) & 0xFF)
        self.trace.append({'kind': 'actor_state', 'state': state, 'enabled': enabled})
        flags = self.read(ACTOR + 0x100) | self.read(ACTOR + 0x104) << 32
        flags = flags | (1 << state) if enabled else flags & ~(1 << state)
        self.put(ACTOR + 0x100, flags & 0xFFFFFFFF); self.put(ACTOR + 0x104, flags >> 32)
        return None, 8

    def _state_active(self, m, esp):
        state = self.read(esp + 4)
        self.trace.append({'kind': 'state_active', 'state': state})
        return int(state == 0x13 and self.op['out_of_supply']), 4

    def _flag(self, name):
        def handler(m, esp):
            assert self.uc.reg_read(UC_X86_REG_ECX) == ACTOR
            self.trace.append({'kind': name})
            return int(self.op[name]), 0
        return handler

    def _center(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ACTOR
        self.trace.append({'kind': 'settlement_center'})
        return (CENTER if self.op['center'] else 0), 0

    def _token(self, m, esp):
        self.trace.append({'kind': 'token'})
        return (TOKEN if self.op['token'] is not None else 0), 0

    def _refresh(self, m, esp):
        self.trace.append({'kind': 'refresh', 'actor': self.name(self.uc.reg_read(UC_X86_REG_ECX))})
        return None, 0

    def _clear(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.trace.append({'kind': 'clear_queue', 'structure': 's%d' % ((this - SCOMPS) // 0x40)})
        return None, 0

    def run(self):
        entry = {'enable': 0x565400, 'disable': 0x564DC0, 'recompute': 0x565140}[self.op['kind']]
        self.call(entry, [], COMP)
        return {'states': self.read(ACTOR + 0x100) | self.read(ACTOR + 0x104) << 32,
                'mask': self.read(ACTOR + 0xF0) | self.read(ACTOR + 0xF4) << 32, 'events': self.trace}


R = random.Random(0x565140)


def make_op(i):
    mode = R.choice([0x1B, 0x1B, 0x1D, 0x1E, None, 0x1C])
    states = (1 << mode) if mode is not None else 0
    states |= (1 << 21) if R.random() < 0.2 else 0
    states |= (1 << 20) if R.random() < 0.3 else 0
    states |= (1 << 36) if R.random() < 0.2 else 0
    if R.random() < 0.15:
        states |= 1 << 0x1E
    n = R.randint(0, 4)
    limit = R.choice([n, n, n + 1, max(0, n - 1)])
    return {'name': f'random-{i}', 'kind': R.choice(['enable', 'disable', 'recompute', 'recompute']),
            'states': states, 'mask': R.getrandbits(64), 'c90': R.random() < 0.3,
            'can_sell': R.random() < 0.7, 'can_disable': R.random() < 0.7, 'upgrades': R.random() < 0.7,
            'allow_sell_under_siege': R.random() < 0.5, 'center': R.random() < 0.5,
            'center_disabled': R.random() < 0.3, 'token': R.choice([None, 0, 0x800, 0x801, 0x7FF]),
            'virtual_100': R.random() < 0.4, 'in_settlement': R.random() < 0.6, 'out_of_supply': R.random() < 0.3,
            'settlement': {'limit': limit, 'count': n, 'center': R.random() < 0.6,
                           'structures': [R.random() < 0.2 for _ in range(n)],
                           'members': [R.random() < 0.8 for _ in range(R.randint(0, 3))]}}


def main():
    ops = [make_op(i) for i in range(300)]
    fixtures, executions = [], 0
    for op in ops:
        for mode, cw in MODES:
            m = StateMachine(cw, op)
            fixtures.append({'name': op['name'], 'precision': mode, 'op': op, 'expected': m.run()})
            executions += 1
    doc = {'schema': 1, 'group_mask': GROUP_MASK, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-structure-state-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
