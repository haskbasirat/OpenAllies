"""Original KKC_GWorld::CreateGThing 0x4F9C10 under Unicorn: its own control
flow, with every callee seamed and recorded in call order.

Seamed callees (each returns the case's configured value):
  4154D0 instance-name map Find on world+F0 (entry+4 = GTID), IDMGR tables
  (B364C0: generation at +4+slot*2, object at +20004+slot*4), 404ED0 debug
  log, 6F7810 allocation, the nine class constructors (4EA5E0 Object,
  4D6BB0 Actor, 4E4DC0 Kingdom, 4E66D0 class 5, 4DB610 Area, 4DBEE0 Camera,
  4DE350 Group, 4DCB30 DynamicDoodad, 4DC450 class 10), 4D64F0 actor state,
  4CF480 component factory (stores actor+60+slot*4), 432E90 state test,
  855D30 assertion dialog (latch B3CC42), 57C450 Organization layout,
  object virtuals E4 (InitCreate), E8 (InitExisting), EC (PostCreate), B0,
  params virtuals 0 (GACD) and 4 (GKCD), 4F9510 world registration, 50C220
  Updatable registration on world+130, 42A590 definition+BC action list,
  4F5490 destroy.
A call on a null object is recorded as outcome `null_dereference`.
"""
from pathlib import Path
import sys, json, struct, hashlib, itertools
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_supply import Machine as Base, P, V, VISITED, NullAccess
from unicorn.x86_const import *

H = lambda b: hashlib.sha256(b).hexdigest()
CTORS = {0x4ea5e0: 2, 0x4d6bb0: 3, 0x4e4dc0: 4, 0x4e66d0: 5, 0x4db610: 6,
         0x4dbee0: 7, 0x4de350: 8, 0x4dcb30: 9, 0x4dc450: 10}
SIZES = {2: 0x60, 3: 0x180, 4: 0x378, 5: 0x6c, 6: 0x8c, 7: 0x68, 8: 0x1c0, 9: 0x68, 10: 0x68}


class Stop(Exception):
    pass


class Machine(Base):
    def setup(self, c):
        self.case = c
        self.calls = []
        self.stub = self.alloc(0x1000)
        # stub addresses for virtuals: object E4/E8/EC/B0, params 0/4
        names = ['obj_e4', 'obj_e8', 'obj_ec', 'obj_b0', 'params_0', 'params_4']
        self.stubs = {self.stub + 16 * i: n for i, n in enumerate(names)}
        by_name = {n: a for a, n in self.stubs.items()}
        self.obj_vt = self.alloc(0x200)
        for name, off in (('obj_e4', 0xe4), ('obj_e8', 0xe8), ('obj_ec', 0xec), ('obj_b0', 0xb0)):
            self.write(self.obj_vt + off, by_name[name])
        params_vt = self.alloc(0x20)
        self.write(params_vt, by_name['params_0'])
        self.write(params_vt + 4, by_name['params_4'])
        # IDMGR
        self.idmgr = self.alloc(0x60000)
        self.write(0xb364c0, self.idmgr)
        self.u.mem_write(0xb3cc42, bytes([1 if c['latch'] else 0]))
        # definition: +8 IDS, +18 category (+1C class), +174 flags, +BC list
        self.definition = self.alloc(0x400)
        self.write(self.definition + 8, self.string('def_ids'.encode('utf-16-le')))
        category = self.alloc(0x40)
        self.write(category + 0x1c, c['class'])
        self.write(self.definition + 0x18, category)
        self.write(self.definition + 0x174, c['definition_flags'])
        # params
        self.params = self.alloc(0x80)
        self.write(self.params, params_vt)
        self.write(self.params + 4, self.definition)
        self.write(self.params + 0x1c, self.string(c['name'].encode('utf-16-le')))
        self.write(self.params + 0x2c, c['layout'])
        self.write(self.params + 0x38, c['mode'])
        self.u.mem_write(self.params + 0x50, bytes([c['byte_50']]))
        # world (this): +F0 name map, +130 updatable owner
        self.gworld = self.alloc(0x400)
        self.write(self.gworld + 0x130, 0x7130)
        self.name_entry = self.alloc(8)
        if c['name_gtid'] is not None:
            gtid = c['name_gtid']
            self.write(self.name_entry + 4, gtid)
            slot = gtid & 0xffff
            gen = (gtid >> 16) if c['name_generation_matches'] else ((gtid >> 16) + 1) & 0xffff
            self.u.mem_write(self.idmgr + 4 + slot * 2, struct.pack('<H', gen))
            self.write(self.idmgr + 0x20004 + slot * 4, 0x5555 if c['name_object_live'] else 0)
        self.objects = []

    def record(self, name, *args):
        self.calls.append([name] + [a & 0xffffffff for a in args])

    def hook(self, u, pc, size, arg):
        c = self.case
        ecx = u.reg_read(UC_X86_REG_ECX)
        if pc in self.stubs:
            name = self.stubs[pc]
            if name.startswith('obj_') and ecx == 0:
                raise Stop('null_dereference')
            if name == 'params_0':
                self.record('params_vf0')
                self.ret(value=int(c['actor_params']))
            elif name == 'params_4':
                self.record('params_vf4')
                self.ret(value=int(c['kingdom_params']))
            elif name == 'obj_e4':
                self.record('init_create', self.objects.index(ecx))
                self.ret(4, value=int(c['init_create']))
            elif name == 'obj_e8':
                self.record('init_existing', self.objects.index(ecx))
                self.ret(4, value=int(c['init_existing']))
            elif name == 'obj_ec':
                self.record('post_create', self.objects.index(ecx), self.arg(1))
                self.ret(8)
            elif name == 'obj_b0':
                self.record('fires_creation_fx', self.objects.index(ecx))
                self.ret(value=int(c['fires_creation_fx']))
            return
        if pc == 0x4154d0:
            assert ecx == self.gworld + 0xf0
            self.record('instance_name_lookup')
            self.ret(4, value=self.name_entry if c['name_gtid'] is not None else 0)
            return
        if pc == 0x404ed0:
            fmt = ''.join(map(chr, self.wtext(self.arg(4))))
            self.calls.append(['log', self.arg(2), fmt])
            self.ret()
            return
        if pc == 0x6f7810:
            size = self.arg()
            self.record('allocate', size)
            if c['allocation_fails']:
                self.ret(value=0)
            else:
                self.ret(value=self.alloc(max(size, 0x200)))
            return
        if pc in CTORS:
            self.record('construct', CTORS[pc])
            obj = ecx
            self.write(obj, self.obj_vt)
            self.write(obj + 4, self.definition)
            self.objects.append(obj)
            self.ret(4, value=obj)
            return
        if pc in (0x4d64f0, 0x4cf480, 0x432e90, 0x4f9510, 0x50c220, 0x4f5490, 0x42a590, 0x57c450):
            if pc in (0x4d64f0, 0x4cf480, 0x432e90) and ecx == 0:
                raise Stop('null_dereference')
        if pc == 0x4d64f0:
            self.record('set_state', self.objects.index(ecx), self.arg(), self.arg(1))
            self.ret(8)
            return
        if pc == 0x4cf480:
            slot = self.arg()
            self.record('create_component', self.objects.index(ecx), slot)
            self.write(ecx + 0x60 + 4 * slot, 0x7000 + slot)
            self.ret(4)
            return
        if pc == 0x432e90:
            self.record('state_active', self.objects.index(ecx), self.arg())
            self.ret(4, value=int(c['construction_state']))
            return
        if pc == 0x855d30:
            self.record('assert_dialog')
            self.ret(value=int(c['dialog_result']))
            return
        if pc == 0x57c450:
            self.record('set_organization_layout', ecx, self.arg())
            self.ret(4)
            return
        if pc == 0x4f9510:
            assert ecx == self.gworld
            self.record('register', self.objects.index(self.arg()))
            self.ret(4)
            return
        if pc == 0x50c220:
            self.record('register_updatable', ecx, self.objects.index(self.arg()))
            self.ret(4)
            return
        if pc == 0x42a590:
            assert ecx == self.definition + 0xbc
            self.record('creation_fx', self.objects.index(self.arg()), self.arg(1))
            self.ret(8)
            return
        if pc == 0x4f5490:
            assert ecx == self.gworld
            self.record('destroy', self.objects.index(self.arg()))
            self.ret(4)
            return
        super().hook(u, pc, size, arg)

    def run_case(self, c):
        self.setup(c)
        try:
            self.call(0x4f9c10, self.gworld, [self.params])
            ret = self.u.reg_read(UC_X86_REG_EAX)
            outcome = 'object' if ret else 'null'
            index = self.objects.index(ret) if ret else None
        except Stop as e:
            outcome, index = str(e), None
        except NullAccess:
            outcome, index = 'null_dereference', None
        return {'outcome': outcome, 'object': index, 'calls': self.calls,
                'latch': self.u.mem_read(0xb3cc42, 1)[0]}


def cases():
    base = dict(cls=3, name='', name_gtid=None, name_generation_matches=True, name_object_live=True,
                actor_params=True, kingdom_params=True, layout=0, mode=0, byte_50=0,
                definition_flags=0, latch=False, construction_state=False, dialog_result=False,
                allocation_fails=False, init_create=True, init_existing=True, fires_creation_fx=True)
    rows = []

    def add(**kw):
        c = dict(base)
        c.update(kw)
        c['class'] = c.pop('cls')
        rows.append(c)

    for cls in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0xffffffff]:
        add(cls=cls)
        add(cls=cls, allocation_fails=True)
        add(cls=cls, actor_params=False, kingdom_params=False)
    for gtid, gen, live in [(0x10005, True, True), (0x10005, False, True), (0x10005, True, False)]:
        add(name='unit_a', name_gtid=gtid, name_generation_matches=gen, name_object_live=live)
    add(name='unit_b')
    for mode, b50 in itertools.product([0, 1, 2, 3], [0, 1]):
        add(mode=mode, byte_50=b50)
    for layout, flags, latch, state, dialog in itertools.product(
            [0, 0x4440], [0, 0x80, 0x180], [False, True], [False, True], [False, True]):
        add(layout=layout, definition_flags=flags, latch=latch, construction_state=state,
            dialog_result=dialog, mode=3, byte_50=1)
    for e4, e8, fx in itertools.product([False, True], repeat=3):
        for cls in (2, 3, 9):
            add(cls=cls, init_create=e4, init_existing=e8, fires_creation_fx=fx)
    return rows


def main():
    out = []
    for c in cases():
        m = Machine(b'', 0x27f)
        native = m.run_case(c)
        out.append({'input': c, 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-create-gthing.json'
    path.write_text(json.dumps(doc, indent=0))
    outcomes = {}
    for row in out:
        outcomes[row['native']['outcome']] = outcomes.get(row['native']['outcome'], 0) + 1
    print(json.dumps({'cases': len(out), 'outcomes': outcomes, 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
