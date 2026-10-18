"""Round-2 Unicorn oracles: GameSpeed, admin-clock estimate and SetFrozen.

Executed original code (unchanged AA.exe bytes):
  0x76F370  KKC_Sim::SetGameSpeed clamp (min 0x76F38C.., max 0x76B3A0), the
            speed-change test and the 0x76B230 conversion
  0x76B230  +0x2C = min(f32(pow(2.0, speed)), +0x24) / AdminTicksPerSecond
  0x96D890  MSVC CRT _CIpow, both paths: SSE2 0x977A10 (flag 0xB4B340 = 1,
            set by ___sse2_available_init 0x9773D4 on SSE2 CPUs except AMD
            family 0xF) and the x87 path 0x96D8ED (flag 0)
  0x74B180  TGC_AdminNode current admin clock estimate (with 0x76AF90)
  0x76DD90  KKC_Sim::SetFrozen (0x74B180 on node 0xB36558, 0x76DCE0)
Seams: the speed-changed UI message branch 0x76F474..0x76F511 (recorded and
skipped), sync-log writers, diagnostics/assertion logger, the admin real
clock (native stub returning a scripted binary32).
"""
from pathlib import Path
import hashlib, json, struct, sys, random

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_tick_driver as D
from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_FPCW

bits, fl, digest = D.bits, D.fl, D.digest
RANGES = {'set_game_speed_76F370': (0x76F370, 0x76F525), 'speed_to_rate_76B230': (0x76B230, 0x76B28C),
          'max_speed_76B3A0': (0x76B3A0, 0x76B3F1), 'cipow_96D890': (0x96D890, 0x96D8E4),
          'admin_clock_estimate_74B180': (0x74B180, 0x74B21C), 'set_frozen_76DD90': (0x76DD90, 0x76DE70)}


class M(D.Machine):
    def __init__(self, pc, sse2):
        super().__init__(pc)
        self.allow_freeze_calls = True
        self.w32(0xB4B340, sse2)
        self.svar = {}
        for off in (0x60, 0x64, 0x68, 0x6C, 0x20):
            self.svar[off] = self.var(self.gvars, off)
        self.speed_ret = self.alloc(16)
        self.tramp_pow = self.code(b'\xdd\x05' + struct.pack('<I', 0xA01B60) + b'\xd9\x05'
                                   + struct.pack('<I', self.speed_ret) + b'\xe8' + b'\0' * 4
                                   + b'\xdb\x3d' + struct.pack('<I', self.res80) + b'\xc3')
        rel = 0x96D890 - (self.tramp_pow + 12 + 5)
        self.u.mem_write(self.tramp_pow + 13, struct.pack('<i', rel))
        self.tramp_est = self.code(b'\x6a\x01\xe8' + b'\0' * 4 + b'\xdb\x3d' + struct.pack('<I', self.res80) + b'\xc3')
        rel = 0x74B180 - (self.tramp_est + 2 + 5)
        self.u.mem_write(self.tramp_est + 3, struct.pack('<i', rel))

    def hook(self, u, pc, size, _):
        if pc == 0x76F474:
            self.events.append({'event': 'speed_changed_message'})
            self.u.reg_write(UC_X86_REG_EIP, 0x76F511)
            return
        super().hook(u, pc, size, _)


def speed_inputs():
    return [-6.0, -4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, -0.0,
            0.5, -0.5, 1.5, 0.25, 0.58496250072, 1.0 / 3.0, -1.7, 2.000001]


def pow_inputs(seed=0x5EEDC):
    rng = random.Random(seed)
    out = [bits(y) for y in speed_inputs()]
    out += [bits(rng.uniform(-4.0, 4.0)) for _ in range(400)]
    out += [bits(rng.uniform(-8.0, 8.0)) for _ in range(100)]
    return out


def pow_rows():
    # MXCSR 0x1F80 and masked x87 exceptions select the SSE2 routine when the
    # CRT flag is set (0x96D899..0x96D8C6), as on Windows SSE2 CPUs.
    rows = []
    for sse2 in (1, 0):
        for pc in (24, 53, 64):
            m = M(pc, sse2)
            for y in pow_inputs():
                m.w32(m.speed_ret, y)
                m.call(m.tramp_pow, 0)
                # The CRT routine must hand back the caller's control word.
                assert m.u.reg_read(UC_X86_REG_FPCW) & 0x1F3F == D.PC[pc] & 0x1F3F, (sse2, pc, y)
                rows.append({'sse2_path': sse2, 'precision': pc, 'exponent': y, 'result_f80': m.f80()})
    return rows


def game_speed_rows(seed=0x5EED9):
    rng = random.Random(seed)
    rows = []
    svars = [(-2.0, 2.0, -4.0, 4.0), (-2.0, 2.0, -4.0, 4.0), (-3.0, 7.0, -6.0, 9.0)]
    for sse2 in (1, 0):
        for pc in (24, 64):
            for i, speed in enumerate(speed_inputs() + [10.0, -10.0]):
                m = M(pc, sse2)
                mn, mx, fmn, fmx = svars[i % len(svars)]
                for off, v in ((0x60, mn), (0x64, mx), (0x68, fmn), (0x6C, fmx)):
                    m.w32(m.svar[off], bits(v))
                admin_tps = [16.0, 16.0, 10.0, 15.0][i % 4]
                m.w32(m.svar[0x20], bits(admin_tps))
                mode = [0, 0, 4, 2, 4][i % 5]
                f4 = int(i % 7 == 3)
                m.w32(m.session + 0xE4, mode)
                m.w8(m.session + 0xF4, f4)
                cutscene = int(i % 6 == 5)
                world = int(i % 11 != 10)
                m.w8(m.world + 0x275, cutscene)
                if not world:
                    m.w32(0xB36534, 0)
                old = bits(rng.choice([0.0, 1.0, -2.0, speed, fl(bits(speed)) + 5e-5]))
                max_speed = bits(rng.choice([64.0, 64.0, 2.5, 8.0]))
                m.set_sim({'speed': old, 'max_speed': max_speed, 'game_per_admin_tick': 0})
                m.events = []
                m.call(0x76F370, m.sim, (bits(speed),))
                rows.append({'sse2_path': sse2, 'precision': pc, 'argument': bits(speed),
                             'svars': {'game_speed_min': bits(mn), 'game_speed_max': bits(mx),
                                       'film_min': bits(fmn), 'film_max': bits(fmx),
                                       'admin_ticks_per_second': bits(admin_tps)},
                             'session_mode': mode, 'session_f4': f4, 'world_present': world,
                             'cutscene': cutscene, 'old_speed': old, 'max_speed': max_speed,
                             'events': m.events, 'speed': m.r32(m.sim + 0x20),
                             'game_per_admin_tick': m.r32(m.sim + 0x2C)})
    return rows


def estimate_rows(seed=0x5EEDA):
    rng = random.Random(seed)
    rows = []
    def rnd():
        return bits(rng.choice([0.0, 0.0625, 1.0, 3.5, 7.1, 100.25]) + rng.choice([0.0, rng.uniform(-1, 1)]))
    for pc in (24, 53, 64):
        for i in range(60):
            m = M(pc, 1)
            s = {'begin_game': rnd(), 'begin_admin': rnd(), 'end_game': rnd(), 'end_admin': rnd(),
                 'paused': int(i % 9 == 4), 'frozen': int(i % 8 == 3), 'game_per_admin_tick': bits(1 / 16)}
            if i % 5 == 0:
                s['end_game'] = s['begin_game']
            m.set_sim(s)
            world_end = rnd()
            m.set_world(3, 0, world_end, bits(1 / 16))
            admin = rnd()
            real = rnd()
            m.w32(m.admin + 0x28C, admin)
            m.w32(m.cell_real, real)
            m.call(m.tramp_est, m.admin)
            rows.append({'precision': pc, 'sim': s, 'world_end': world_end, 'admin_time': admin,
                         'real_admin_clock': real, 'result_f80': m.f80()})
    return rows


def set_frozen_rows(seed=0x5EEDB):
    rng = random.Random(seed)
    rows = []
    for pc in (24, 64):
        for i in range(24):
            m = M(pc, 1)
            frozen_before = i % 3 != 2
            arg = int(i % 4 == 1)
            s = {'begin_game': bits(rng.uniform(0, 9)), 'begin_admin': bits(rng.uniform(0, 9)),
                 'end_game': bits(rng.uniform(0, 9)), 'end_admin': bits(rng.uniform(0, 9)),
                 'paused': 0, 'frozen': int(frozen_before), 'max_speed': bits(2.0),
                 'game_per_admin_tick': bits(1 / 16)}
            m.set_sim(s)
            wbegin = bits(rng.choice([0.0, 2.3125, rng.uniform(0, 50)]))
            m.set_world(9, wbegin, bits(fl(wbegin) + 0.0625), bits(1 / 16))
            admin = bits(rng.uniform(0, 60))
            real = bits(rng.uniform(0, 60))
            m.w32(m.admin + 0x28C, admin)
            m.w32(m.cell_real, real)
            m.events = []
            m.call(0x76DD90, m.sim, (arg,))
            rows.append({'precision': pc, 'sim': s, 'world_begin': wbegin, 'world_end': m.r32(m.world + 0xE4),
                         'admin_time': admin, 'real_admin_clock': real, 'argument': arg,
                         'events': [e for e in m.events if e['event'] != 'diagnostic'],
                         'after': m.sim_state()})
    return rows


def main():
    ranges = {}
    for name, (a, b) in RANGES.items():
        data = D.NATIVE.range(a, b - a)
        assert data == D.ACCEPTED.range(a, b - a), name
        ranges[name] = {'start': hex(a), 'end_exclusive': hex(b), 'sha256': digest(data), 'unchanged_in_v14': True}
    pows = pow_rows()
    speeds = game_speed_rows()
    est = estimate_rows()
    frozen = set_frozen_rows()
    # Path comparison for non-integral exponents (documentation only).
    div = []
    by = {}
    for r in pows:
        by.setdefault((r['precision'], r['exponent']), {})[r['sse2_path']] = r['result_f80']
    for (pc, e), v in sorted(by.items()):
        if v[0] != v[1]:
            div.append({'precision': pc, 'exponent': e, 'sse2': v[1], 'x87': v[0]})
    report = {'schema': 1, 'passed': True, 'source_sha256': digest(D.NATIVE.b),
              'accepted_sha256': digest(D.ACCEPTED.b), 'producer_sha256': digest(Path(__file__).read_bytes()),
              'method': __doc__, 'executed_ranges': ranges, 'pow_rows': pows, 'pow_path_divergence': div,
              'game_speed_rows': speeds, 'admin_clock_estimate_rows': est, 'set_frozen_rows': frozen}
    target = HERE / 'native-tick-round2.json'
    target.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'pow_rows': len(pows), 'pow_divergent': len(div), 'game_speed_rows': len(speeds),
                      'estimate_rows': len(est), 'set_frozen_rows': len(frozen),
                      'sha256': digest(target.read_bytes())}))


if __name__ == '__main__':
    main()
