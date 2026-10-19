"""Oracle: KKC_TellSelectedCommandOrder::Process 0x61F210 (GameOrders.cpp).

Runs, in both pinned images at PC24/PC53/PC64, the ORIGINAL 0x61F210 and
natively inside it 0x74B3B0 (Admin+284), 0x765E00 (Player+28 when [B36534]
GWorld is not NULL), 0x766380 (Player+8 name copy, flag 0) and 0x40B000
(59ABA0(B39690+50) * 0.001f), plus the CString release paths. Seams (each
recorded with its arguments):
  0x59ABA0         timer milliseconds (ecx must be B396E0 = B39690+50)
  0x4DA570         GuessCommandData(cmd, actor, 0) -> case definition
  0x4ED360         selection delivery (ecx = Player+50, arg = &order+4)
  0x404F90         diagnostic log: channel/file/line/function/format and
                   the varargs decoded per format (doubles as raw bits)
  0x401B00         CString(L"?") (9F7228), 0x4012C0 CStringData clone
  actor vt+B4 / vt+108 (raw EAX; AL tested), every other slot traps.
Inputs: Admin [B36558] +284 player (+8 name, +28 kingdom, +50 selection:
+4 head node {actor, next}), [B36534] GWorld, Kingdom+188, command +0C
flags / +08 definition (+8 IDS CString), Actor +14 GTID and +4 ThingData
(+8 IDS CString), [B39B1C] view time binary32.

  python native_describe_61f210.py --write   (refuses to overwrite)
  python native_describe_61f210.py           (replay, byte-identical)
"""
from pathlib import Path
import itertools, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import recruit_lab as lab  # noqa: E402
nm = lab.nm

OUT = HERE / 'native-describe-61f210.json'
FUNCTIONS = {'61F210': (0x61f210, 0x61f521), '74B3B0': (0x74b3b0, 0x74b3b7), '765E00': (0x765e00, 0x765e10),
             '766380': (0x766380, 0x7663d7), '40B000': (0x40b000, 0x40b021)}
EMPTY_FORMAT = 0xA5B900
SELECTED_FORMAT = 0xA5B7C8


class Machine(lab.Lab):
    def __init__(self, precision, image):
        super().__init__(precision, image)
        self.events = []
        self.seam(0x59ABA0, self._timer)
        self.seam(0x4DA570, self._guess)
        self.seam(0x4ED360, self._deliver)
        self.seam(0x404F90, self._log)
        self.seam(0x401B00, self._cstring)
        self.seam(0x4012C0, self._clone)
        self.trap = self.stub(self._trap)
        self.vt_b4 = self.stub(self._b4)
        self.vt_108 = self.stub(self._k108)
        self.admin = self.alloc(0x300)
        self.w(0xB36558, self.admin)
        self.mark = self.heap

    # seams --------------------------------------------------------------
    def _trap(self, m):
        raise AssertionError('unexpected virtual call')

    def _timer(self, m):
        m.events.append(['timer', hex(m.ecx())])
        m.ret(m.case['milliseconds'])

    def _guess(self, m):
        m.events.append(['guess', m.names.get(m.arg(1), hex(m.arg(1))), m.arg(2)])
        g = m.case['guess']
        m.ret(m.defs[g] if g is not None else 0, pop=8)

    def _deliver(self, m):
        assert m.ecx() == m.selection and m.arg(1) == m.order + 4
        m.events.append(['deliver'])
        m.ret(0, pop=4)

    def _cstring(self, m):
        this, src = m.ecx(), m.arg(1)
        m.events.append(['cstring', m.text(src)])
        m.w(this, m.cstring(m.text(src)))
        m.ret(this, pop=4)

    def _clone(self, m):
        h = m.arg(1)
        m.w(h + 12, m.r(h + 12) + 1)
        m.ret(h)

    def _b4(self, m):
        m.events.append(['b4'])
        m.ret(m.case['actor']['b4'])

    def _k108(self, m):
        m.events.append(['108'])
        k = m.case['actor']['kingdom']
        m.ret(m.kingdoms[k] if k is not None else 0)

    def _log(self, m):
        sp = m.u.reg_read(nm.UC_X86_REG_ESP)
        a = lambda i: m.r(sp + 4 * i)
        fmt = a(5)
        record = {'channel': hex(a(1)), 'file': m.text(a(2)), 'line': a(3), 'function': m.text(a(4)),
                  'format': m.text(fmt)}
        q = lambda off: m.r(sp + off) | (m.r(sp + off + 4) << 32)
        s = lambda v: v - (1 << 32) * (v >> 31)
        if fmt == EMPTY_FORMAT:
            record['args'] = {'time_f64': q(0x18), 'player': m.text(a(8)), 'player_kingdom': s(a(9)),
                              'order': m.text(a(10))}
        elif fmt == SELECTED_FORMAT:
            record['args'] = {'time_f64': q(0x18), 'view_f64': q(0x20), 'player': m.text(a(10)),
                              'player_kingdom': s(a(11)), 'actor': s(a(12)), 'actor_ids': m.text(a(13)),
                              'actor_kingdom': s(a(14)), 'order': m.text(a(15))}
        else:
            raise AssertionError(hex(fmt))
        m.events.append(['log', record])
        m.ret(0)

    # case -----------------------------------------------------------------
    def run(self, case):
        self.heap = self.mark
        self.case, self.events, self.names = case, [], {}
        self.kingdoms = {}
        for k, index in case['kingdoms'].items():
            p = self.alloc(0x200)
            self.w(p + 0x188, index)
            self.kingdoms[k] = p
        self.defs = {}
        for name in case['definitions']:
            d = self.alloc(0x48)
            self.w(d + 8, self.cstring(name))
            self.defs[name] = d
        player = self.alloc(0x60)
        self.w(self.admin + 0x284, player)
        self.w(player + 8, self.cstring(case['player']))
        self.w(player + 0x28, self.kingdoms[case['player_kingdom']] if case['player_kingdom'] is not None else 0)
        self.selection = self.alloc(0x40)
        self.w(player + 0x50, self.selection)
        self.w(0xB36534, self.alloc(0x10) if case['world'] else 0)
        self.w(0xB39B1C, case['view_bits'])
        if case['actor'] is not None:
            node = self.alloc(8)
            self.w(self.selection + 4, node)
            if case['actor']['present']:
                actor = self.alloc(0x200)
                vt = self.array([self.trap] * 0x80)
                self.w(vt + 0xB4, self.vt_b4)
                self.w(vt + 0x108, self.vt_108)
                self.w(actor, vt)
                self.w(actor + 0x14, case['actor']['gtid'])
                thing = self.alloc(0x40)
                self.w(thing + 8, self.cstring(case['actor']['ids']))
                self.w(actor + 4, thing)
                self.names[actor] = 'actor'
                self.w(node, actor)
        command = self.alloc(0x40)
        self.w(command + 0x0C, case['flags'])
        self.w(command + 8, self.defs[case['definition']] if case['definition'] is not None else 0)
        self.order = self.alloc(0x10)
        self.w(self.order + 4, command)
        self.u.reg_write(nm.UC_X86_REG_FPSW, 0)
        self.call(0x61F210, ecx=self.order, pop=0)
        assert (self.u.reg_read(nm.UC_X86_REG_FPSW) >> 11) & 7 == 0
        return {'events': self.events}


def cases():
    rng = random.Random(0x61F210)
    out = []
    ms_values = [0, 1, 999, 1000, 1049, 12345, 0x7fffffff, 0x80000000, 0x80000001, 0xffffffff, 0x12345678,
                 0xdeadbeef, 3600_000, 86_399_999]
    views = [0, 0x3f800000, 0x80000000, 0x7f800000, 0x7fc00000, 0x00000001, 0x447a0000, 0x4b189680]
    names = ['move', 'recruit', 'kill', 'pack']
    # Structure grid.
    for selection, present, b4, flags, definition, guess, world, pk, ak in itertools.product(
            ['empty', 'actor'], [True, False], [1, 0x100], [0x40, 0x42, 0x5c, 0x1c, 0],
            [None, 'recruit'], [None, 'move'], [True, False], [None, 'p'], [None, 'a']):
        if selection == 'empty' and (not present or b4 != 1 or guess or ak):
            continue
        if not (flags & 0x40) and definition is not None:
            continue
        out.append({'actor': None if selection == 'empty' else {'present': present, 'b4': b4, 'kingdom': ak,
                                                              'gtid': 0x0003002a, 'ids': 'rus_corps'},
                    'flags': flags, 'definition': definition, 'guess': guess, 'world': world,
                    'player_kingdom': pk, 'kingdoms': {'p': 3, 'a': 0x7fffffff}, 'definitions': names,
                    'player': 'Local human', 'milliseconds': 160, 'view_bits': 0})
    # Time / view arithmetic at every precision.
    for ms in ms_values + [rng.getrandbits(32) for _ in range(40)]:
        for selection in ['empty', 'actor']:
            out.append({'actor': None if selection == 'empty' else {'present': True, 'b4': 1, 'kingdom': 'a',
                                                                  'gtid': rng.getrandbits(32), 'ids': 'regiment_rus_hq_infantry'},
                        'flags': 0x42, 'definition': 'recruit', 'guess': None, 'world': True, 'player_kingdom': 'p',
                        'kingdoms': {'p': rng.choice([0, 1, 7, -1 & 0xffffffff]), 'a': rng.getrandbits(32)},
                        'definitions': names, 'player': rng.choice(['', 'P', 'Local human', 'x' * 40]),
                        'milliseconds': ms, 'view_bits': rng.choice(views + [rng.getrandbits(31)])})
    return out


def main():
    write = '--write' in sys.argv
    rows, visited = [], {}
    for label, image in lab.IMAGES.items():
        for pc in nm.PRECISIONS:
            m = Machine(pc, image)
            for case in cases():
                rows.append({'image': label, 'precision': pc, 'input': case, 'output': m.run(case)})
            m.verify_v14()
            visited.update(m.visited)
    doc = nm.header(__file__, 'unicorn original 61F210 + 74B3B0/765E00/766380/40B000')
    doc['scope'] = __doc__
    doc['coverage'] = lab.coverage(FUNCTIONS, visited)
    doc['rows'] = rows
    doc['instructions'] = nm.instructions(visited)
    lab.write_or_replay(OUT, doc, write)
    print({k: f"{v['executed']}/{v['instructions']}" for k, v in doc['coverage'].items()}, len(rows), 'rows')


if __name__ == '__main__':
    main()
