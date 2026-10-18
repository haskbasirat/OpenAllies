"""Original KKC_UAIStateUnpack::Think 5372B0 (element state uai_unpack at
Mind+188) executing unchanged on an emulated element Mind with 529E60,
529C30 and 52EBF0 (native_uai_harness.py), at Single24, Double53 and
Extended64.

Seams (recorded in order): the move state Think, 575C80 on the actor pack
component (+AC; records the +18/+1C point it receives; a null component is a
native null `this`), 52EC20 on the unpack state (placement, not ported;
writes the scripted point and, in some cases, a scripted move state status
so the Think's reread of Mind+88 is exercised), 4515F0 (binary32 point equality, computed
exactly), 52DF60 on the move state (movement-owned; seven arguments), Mind
virtual +1B0. The error message uses refcounted fake CStrings: 401B00
(wide literal), actor virtual +24 (name), 4B9F40 (display string on
[B36544]), 4012C0 (CString data clone), actor virtual +108 (owner) and
40C230 (message post) are seamed; the Think's own releases call the fake
string manager's virtual +4 (not recorded).
"""
from pathlib import Path
import random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, from_f32, produce

ENTRY, END = 0x5372b0, 0x53752a
POSITION, POSITION_END = 0x52ebf0, 0x52ec1e


class UnpackMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.mind_virtual(0x1b0, 'mind_virtual_1b0')
        a = self.actor
        d = self.r(a + 4)
        self.w(d + 0x538, c['offset'][0]); self.w(d + 0x53c, c['offset'][1])
        self.pack = 0
        if c['pack']:
            self.pack = self.obj('pack'); self.w(a + 0xac, self.pack)
        if c['organization'] is not None:
            ms = self.obj('membership'); self.w(a + 0x80, ms)
            org = self.obj('organization'); self.w(ms + 0x10, org)
            comp = self.alloc(0x100); self.w(org + 0x7c, comp)
            self.w(comp + 0x44, a if c['organization']['leader'] == 'actor' else self.obj('leader'))
        s = self.states['unpack']
        for k, v in enumerate(c['position'] + c['facing']):
            self.w(s + 0x10 + 4 * k, v)
        self.w(self.states['move'] + 0x14, 0x11111111); self.w(self.states['move'] + 0x18, 0x22222222)
        # Fake CString manager (virtual +4 frees, not recorded).
        mgr = self.alloc(0x10); mvt = self.alloc(0x10); self.w(mgr, mvt)
        self.w(mvt + 4, self.stub(lambda m: self.ret(0, 4)))
        self.string_mgr = mgr; self.strings = {}
        self.w(0xb36544, 0x0bad5440)
        self.vslot(a, 0x24, self.v_name); self.vslot(a, 0x108, self.v_owner)
        for addr, fn in [(0x575c80, self.s_unpack_at), (0x52ec20, self.s_place), (0x4515f0, self.s_equal),
                         (0x52df60, self.s_move), (0x401b00, self.s_literal), (0x4b9f40, self.s_display),
                         (0x4012c0, self.s_clone), (0x40c230, self.s_post)]:
            self.seam(addr, fn)

    def cstring(self, text, refs=1):
        hdr = self.alloc(0x20)
        self.w(hdr, self.string_mgr); self.w(hdr + 0xc, refs)
        self.strings[hdr + 0x10] = text
        return hdr + 0x10

    def s_literal(self, m):
        raw = bytes(self.u.mem_read(self.arg(1), 128))
        self.w(self.ecx(), self.cstring(raw.decode('utf-16le').split('\x00')[0])); self.ret(self.ecx(), 4)

    def v_name(self, m):
        assert self.ecx() == self.actor
        dst = self.arg(1); self.w(dst, self.cstring('actor name'))
        self.ev('actor_name'); self.ret(dst, 4)

    def s_display(self, m):
        assert self.ecx() == 0x0bad5440 and self.arg(2) == 0xb43b48
        key = self.strings[self.r(self.arg(1))]
        entry = self.alloc(8); self.w(entry, self.cstring('display:' + key, refs=5))
        self.ev('display_string', key=key); self.ret(entry, 8)

    def s_clone(self, m):
        hdr = self.arg(1)
        self.w(hdr + 0xc, self.r(hdr + 0xc) + 1); self.ret(hdr)

    def v_owner(self, m):
        assert self.ecx() == self.actor
        r = self.c['owner']; self.ev('actor_owner', result=r); self.ret(r)

    def s_post(self, m):
        assert self.arg(1) == 0xb399b0 and self.arg(3) == self.actor and self.arg(5) == 0
        fmt = self.arg(6)
        self.ev('post_message', kind=self.arg(2), owner=self.arg(4), format=self.strings[fmt],
                argument=self.strings[self.arg(7)])
        self.w(fmt - 4, self.r(fmt - 4) - 1)
        self.ret(0)

    def s_unpack_at(self, m):
        if self.ecx() == 0:
            return self.fault(0x575c80)
        assert self.ecx() == self.pack
        p = self.arg(1)
        r = self.c['unpack_result']
        self.ev('unpack_at', point=[self.r(p), self.r(p + 4)], result=r); self.ret(int(r), 4)

    def s_place(self, m):
        s = self.states['unpack']
        assert self.ecx() == s and self.arg(1) == self.actor and self.arg(3) == s + 0x18
        p = self.arg(2)
        before = [self.r(p), self.r(p + 4)]
        r = self.c['place']
        self.ev('place', point=before, facing=[self.r(s + 0x18), self.r(s + 0x1c)], result=r)
        if r is not None:
            self.w(p, r[0]); self.w(p + 4, r[1])
        if self.c['place_move_status'] is not None:
            self.w(self.states['move'] + 0xc, self.c['place_move_status'])
        self.ret(int(r is not None), 12)

    def s_equal(self, m):
        a, b = self.ecx(), self.arg(1)
        va = [from_f32(self.r(a)), from_f32(self.r(a + 4))]
        vb = [from_f32(self.r(b)), from_f32(self.r(b + 4))]
        self.ret(int(va[0] == vb[0] and va[1] == vb[1]), 4)

    def s_move(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_activate', args=[self.arg(i) for i in range(1, 8)])
        self.ret(1, 0x1c)

    def run(self):
        r = self.run_think(ENTRY, 'unpack')
        if 'fault' not in r:
            s = self.states['unpack']
            r['position'] = [self.r(s + 0x10), self.r(s + 0x14)]
            r['move_destination'] = [self.r(self.states['move'] + 0x14), self.r(self.states['move'] + 0x18)]
        return r


def cases():
    rnd = random.Random(0x5372b0)
    rows = []
    for i in range(540):
        pos = [f32(rnd.uniform(-300, 300)), f32(rnd.uniform(-300, 300))]
        place = rnd.choice([None, pos, [f32(rnd.uniform(-300, 300)), f32(rnd.uniform(-300, 300))],
                            [pos[0], f32(rnd.uniform(-300, 300))], [f32(0.0), f32(-0.0)]])
        if place == [f32(0.0), f32(-0.0)]:
            pos = [f32(-0.0), f32(0.0)]
        c = {'name': 'unpack%d' % i, 'precision': list(PRECISIONS)[i % 3],
             'pack': rnd.random() < 0.92, 'organization': rnd.choice([None, {'leader': 'actor'}, {'leader': 'other'},
                                                                       {'leader': 'other'}, {'leader': 'other'}]),
             'position': pos, 'facing': [f32(rnd.choice([0.0, 1.0])), f32(rnd.choice([-1.0, 0.5]))],
             'offset': [f32(rnd.choice([0.0, 1.0 / 3.0, -2.5, 1e-3, 7.0])), f32(rnd.choice([0.0, 0.1, -4.75, 3e3]))],
             'place': place, 'place_move_status': rnd.choice([None, None, None, 2, 1]), 'unpack_result': rnd.random() < 0.5, 'owner': rnd.choice([3, 9]),
             'null_point': [f32(rnd.choice([0.0, -1.0])), f32(rnd.choice([0.0, 3.0e38]))],
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['unpack'], ['unpack', 'sleep'], ['move', 'unpack']])}
        c['status']['unpack'] = rnd.choice([1, 2, 2])
        c['status']['move'] = rnd.choice([0, 1, 2, 2, 0])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-unpack.json', cases(), UnpackMachine,
            {'5372B0': (ENTRY, END), '52EBF0': (POSITION, POSITION_END)})


if __name__ == '__main__':
    main()
