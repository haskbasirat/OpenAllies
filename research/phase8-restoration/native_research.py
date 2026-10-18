"""Original Research component (slot 21): constructor 584210 then saved
Load 5849E0 on one object (Updatable 50BCD0, version 8589B0, ActorData
research list walk 45EE00 on +558, in-progress list head insert +10/+14).
Seams: 4DAAA0 reads the saved UTF-16 reference and returns the registered
Property definition (null for an unregistered name); 42A590 (the property
definition +54 FX action list fired at the actor position) is recorded, not
executed; 858A00 (data error) is recorded and stops."""
from pathlib import Path
import sys, json, struct
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_supply import Machine as Base, P, V, VISITED, W, B, H, NullAccess
from unicorn.x86_const import *

REGISTERED = ['tech_a', 'tech_b', 'tech_c', 'tech_d']


class Machine(Base):
    def setup(self, research_list):
        self.props = {}
        for name in REGISTERED:
            d = self.alloc(0x80)
            text = self.alloc(2 * len(name) + 2)
            self.u.mem_write(text, name.encode('utf-16-le') + b'\0\0')
            self.write(d + 8, text)
            self.props[name] = d
        self.fx = []
        self.error = None

    def hook(self, u, pc, size, arg):
        if pc == 0x4daaa0:
            start = self.r.at
            count = struct.unpack('<H', self.r.take(2))[0]
            name = self.r.take(count * 2).decode('utf-16-le')
            self.write(self.arg(1), self.props.get(name.lower(), 0))
            self.ret(value=self.arg(0))
            return
        if pc == 0x42a590:
            d = u.reg_read(UC_X86_REG_ECX) - 0x54
            name = next(k for k, v in self.props.items() if v == d)
            self.fx.append([name, self.arg(1)])
            self.ret(8)
            return
        if pc == 0x858a00:
            self.error = 'research property not found'
            u.emu_stop()
            return
        super().hook(u, pc, size, arg)

    def research(self, research_list):
        self.setup(research_list)
        owner = self.alloc(0x200)
        vt = self.alloc(0x100)
        self.write(vt + 0x24, 0x2e00050)  # name getter used only by the error path
        self.write(owner, vt)
        definition = self.alloc(0x800)
        self.write(owner + 4, definition)
        head = 0
        for name in reversed(research_list):
            item = self.alloc(0x40)
            self.write(item, self.props[name])
            node = self.alloc(8)
            self.write(node, item)
            self.write(node + 4, head)
            head = node
        self.write(definition + 0x558, head)
        obj = self.object
        for off in range(0, 0x40, 4):
            self.write(obj + off, 0xcdcdcdcd)
        self.call(0x584210, obj, [owner])
        ctor = [self.read(obj + 0x10), self.read(obj + 0x14)]
        null = None
        try:
            self.call(0x5849e0, obj, [self.world])
        except AssertionError:
            if self.error is None:
                raise
        except NullAccess as e:
            # 4DAAA0 null definition then [ebx+8]: a native access violation.
            null = e.args[0]
            self.error = 'native null dereference'
        items = []
        node = self.read(obj + 0x10)
        while node and node != 0xcdcdcdcd:
            item = self.read(node)
            prop = self.read(item)
            items.append(next(k for k, v in self.props.items() if v == prop))
            node = self.read(node + 4)
        tail = self.read(obj + 0x14)
        return {'consumed': self.r.at, 'error': self.error, 'after_ctor_10_14': ctor, 'items_head_first': items,
                'tail': None if tail == 0 else next(k for k, v in self.props.items() if v == self.read(self.read(tail))),
                'fx': [n for n, _ in self.fx]}


def reference(name):
    return struct.pack('<H', len(name)) + name.encode('utf-16-le')


def cases():
    rows = []
    for research_list in [['tech_a', 'tech_b', 'tech_c'], ['tech_c']]:
        for saved in [[], ['tech_a'], ['tech_c', 'tech_a'], ['TECH_B'], ['tech_a', 'tech_b', 'tech_c'], ['tech_d'], ['nope'], ['tech_a', 'tech_a']]:
            payload = W(1, 0) + W(0) + W(len(saved)) + b''.join(reference(n) for n in saved)
            rows.append({'research_list': research_list, 'saved': saved, 'payload': payload.hex()})
    rows.append({'research_list': ['tech_a'], 'saved': [], 'payload': (W(1, 0) + W(0) + W(0xffffffff)).hex()})
    return rows


def main():
    out = []
    for c in cases():
        m = Machine(bytes.fromhex(c['payload']) + b'FOLLOWING', 0x27f)
        native = m.research(c['research_list'])
        out.append({'input': c, 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-research.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
