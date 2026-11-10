"""Original City candidate test 5640A0, City registration 564300 and the actor
maximum value 4D01E0(0) with 578C00(0) under Unicorn.

5640A0, 564300, 4D01E0, 578C00 (zero argument), 4549C0, 4CF1B0, 517F30 and
the node push 4D0900 execute unchanged. Seams: the object's virtual +108
(owner, recorded) answers from the fixture; the allocator 6F7810 returns
fresh fixture memory; the Updatable registration 50AC60 (on City +8, argument
0) is recorded. The City +10 block list and +18 candidate list hold GTID
words; the final +18 list and the GTID of its +1C tail node are recorded.

Usage: native_city_admission.py (AA.exe and the accepted v14 copy are located
by the shared harness relative to this repository).
"""
import copy
import random
import struct
from harness import Machine, PRECISIONS, f32, report
from unicorn.x86_const import UC_X86_REG_ECX

TINY = struct.unpack('<f', struct.pack('<I', 0x30800000))[0]  # 2^-30


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.c = copy.deepcopy(c)
        self.city = self.alloc(0x40)
        self.city_actor = self.alloc(0x100)
        self.w(self.city + 4, self.city_actor)
        self.w(self.city + 0xc, c['updatable_word'])
        self.w(self.city + 0x10, self.gtid_list(c['blocks'])[0])
        head, tail = self.gtid_list(c['candidates'])
        self.w(self.city + 0x18, head)
        self.w(self.city + 0x1c, tail)
        self.kingdom = self.alloc(0x200)
        self.w(self.kingdom + 0x190, c['kingdom_190'])
        self.object = 0
        o = c['object']
        if o == 'city':
            self.object = self.city_actor
        elif o is not None:
            self.object = self.make_object(o)
        self.hooks.update({0x6f7810: self.allocate, 0x50ac60: self.register_update})

    def gtid_list(self, gtids):
        head = tail = 0
        for g in reversed(gtids):
            node = self.alloc(8)
            self.w(node, g)
            self.w(node + 4, head)
            head = node
            if not tail:
                tail = node
        return head, tail

    def make_object(self, o):
        a = self.alloc(0x200)
        owners = list(o['owner'])

        def v108(m):
            owner = owners.pop(0) if len(owners) > 1 else owners[0]
            self.events.append({'Owner': owner})
            self.ret(0, self.kingdom if owner else 0)
        self.w(a, self.vtable({0x108: v108}))
        self.w(a + 8, o['flags8'])
        self.w(a + 0x14, o['gtid'])
        self.w(a + 0x60, o['body'])
        definition = self.alloc(0x300)
        self.wb(definition + 0x290, o['bytes'][0])
        self.wb(definition + 0x291, o['bytes'][1])
        self.w(a + 4, definition)
        if o['membership'] is not None:
            comp = self.alloc(0x40)
            self.w(comp + 0x10, o['membership'])
            self.w(a + 0x80, comp)
        for offset, field, key in ((0x7c, 0x6c, 'organization'), (0x98, 0x34, 'component98'),
                                   (0x74, 0x60, 'performer'), (0x88, 0x14, 'denizen')):
            if o.get(key) is not None:
                comp = self.alloc(0x80)
                self.w(comp + field, o[key])
                self.w(a + offset, comp)
        return a

    def allocate(self, m):
        size = self.arg()
        ptr = self.alloc(size + 16)
        self.u.mem_write(ptr, b'\xcd' * (size + 16))
        self.ret(0, ptr)

    def register_update(self, m):
        assert self.reg(UC_X86_REG_ECX) == self.city + 8
        self.events.append({'Register': self.arg()})
        self.ret(4)

    def list_words(self, head):
        out = []
        while head:
            out.append(self.r(head))
            head = self.r(head + 4)
        return out

    def run(self):
        if self.c['function'] == 'admits':
            result = self.call(0x5640a0, self.city, (self.object,)) & 0xff
        else:
            self.call(0x564300, self.city, (self.object,))
            result = None
        tail = self.r(self.city + 0x1c)
        return {'result': result, 'events': self.events,
                'candidates': self.list_words(self.r(self.city + 0x18)),
                'tail': self.r(tail) if tail else None}


def cases():
    rnd = random.Random(0x5640a0)
    rows = []
    values = [None, None, f32(0.0), f32(2.0), f32(-2.0), f32(0.5), f32(TINY), f32(-1.0), f32(1.0)]
    for i in range(600):
        obj = rnd.choice([None] + ['city'] + [0] * 28)
        if obj == 0:
            obj = {'flags8': rnd.choice([0, 0, 0, 1, 0x1000, 0x1001, 0xffffeffe]),
                   'owner': rnd.choice([[1], [1], [1], [0]]),
                   'gtid': rnd.choice([0x00110011, 0x00220022, 0x00330033]),
                   'body': rnd.choice([0, 0, 0x6000]), 'bytes': rnd.choice([[0, 0], [0, 0], [1, 0], [0, 1]]),
                   'membership': rnd.choice([None, None, None, 0, 0x7000]),
                   'organization': rnd.choice(values), 'component98': rnd.choice(values),
                   'performer': rnd.choice(values), 'denizen': rnd.choice(values)}
        gtids = [0x00110011, 0x00220022, 0x00330033, 0x00440044]
        c = {'name': 'city_admission%d' % i, 'function': rnd.choice(['admits', 'register']), 'object': obj,
             'kingdom_190': rnd.choice([0, 0, 0, 1]), 'updatable_word': rnd.choice([0, 0, 0x5000]),
             'blocks': rnd.sample(gtids, rnd.choice([0, 0, 1, 2])),
             'candidates': rnd.sample(gtids, rnd.choice([0, 1, 2, 3]))}
        rows.append(c)
    # Precision: 1.0 + 2^-30 - 1.0 is 2^-30 above single precision and 0 at it.
    for function in ('admits', 'register'):
        rows.append({'name': 'city_admission_precision_' + function, 'function': function,
                     'object': {'flags8': 0, 'owner': [1], 'gtid': 0x00550055, 'body': 0, 'bytes': [0, 0],
                                'membership': None, 'organization': f32(1.0), 'component98': f32(TINY),
                                'performer': f32(-1.0), 'denizen': None},
                     'kingdom_190': 0, 'updatable_word': 0, 'blocks': [], 'candidates': [0x00110011]})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        for precision in PRECISIONS:
            m = Run(c, precision)
            try:
                result = m.run()
            except Exception as exc:
                raise RuntimeError(f'{c["name"]} {precision}') from exc
            visited.update(m.visited)
            out.append({'input': {**c, 'precision': precision}, 'output': result})
    report('native-city-admission-fixtures.json', __doc__ + '\nEvery case at every precision control.',
           out, visited)


if __name__ == '__main__':
    main()
