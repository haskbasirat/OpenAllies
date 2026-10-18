"""523EA0 actual current/registry/membership/522CC0 rejection/removal prefix.

Runs both unchanged PE images at every x87 PC. Real52A000,529C30,521060,
522CC0,4D1080,51D560,51EFB0,4D0CD0,4F1430 and5E2250 execute when reached.
Allocator release is a recorded host seam.404F90 is either a recorded oracle
transport (not a native logging-policy proof), or an explicit failure cutoff
BEFORE its body; positive57A500 and4D4FF0 stop BEFORE the required child.
Authored object/registry/list fields are a bounded native laboratory, not an
actual saved-world or successful runtime backend. No game files are written.
"""
from pathlib import Path
import argparse, copy, json, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as n
from unicorn.x86_const import UC_X86_REG_EIP


class Probe(n.Machine):
    def __init__(self, image, precision, case):
        super().__init__(precision)
        if image is n.modified:
            old = (n.original.image_size + 0xfff) & ~0xfff
            new = (image.image_size + 0xfff) & ~0xfff
            if new > old:
                self.u.mem_map(image.base + old, new - old)
            for vs, r, size, off in image.sections:
                if size:
                    self.u.mem_write(image.base + r, image.b[off:off + size])
        self.case = copy.deepcopy(case)
        self.cut = None
        self.actors = {}
        self.names = {}
        self.nodes = {}
        self.reg = self.alloc(0x60010)
        self.w(0xb364c0, self.reg)
        self.owner = self.actor('owner', 0x10011)
        self.w(self.owner + 0x104, case.get('owner_flags104', 0))
        self.mind = self.alloc(0x400)
        self.w(self.mind, 0xa335a0)
        self.w(self.mind + 4, self.owner)
        self.engage = self.mind + 0x1cc
        self.w(self.engage + 4, self.mind)
        if case.get('current', True):
            stack = self.alloc(8)
            self.w(stack, self.engage)
            self.w(self.mind + 0x10, stack)
        for name, spec in case.get('actors', {}).items():
            actor = self.actor(name, spec['gtid'])
            if not spec.get('registered', True):
                self.w(self.reg + 0x20004 + 4 * (spec['gtid'] & 0xffff), 0)
            if spec.get('generation_mismatch'):
                self.u.mem_write(self.reg + 4 + 2 * (spec['gtid'] & 0xffff), struct.pack('<H', ((spec['gtid'] >> 16) + 1) & 0xffff))
            self.w(actor + 0xd4, spec.get('d4', 0))
            self.singly(actor + 0x118, spec.get('attackers', []), 'attacker:' + name)
        for name, spec in case.get('actors', {}).items():
            actor = self.actors[name]
            if spec.get('membership') is not None:
                membership = self.alloc(0x24)
                member = spec['membership']
                self.w(actor + 0x80, membership)
                self.w(membership + 0x10, self.actors.get(member['organization'], 0))
                self.wb(membership + 0x1c, member.get('attached', 1))
            if spec.get('mobile_target') is not None:
                mobile = self.alloc(0x40)
                self.w(actor + 0x94, mobile)
                self.w(mobile + 0x34, spec['mobile_target'])
        self.w(self.mind + 0xd0, case.get('forced_target', 0))
        self.doubly(self.engage + 0x10, case['targets'])
        self.seam(0x6f7430, self.free)
        self.seam(0x404f90, self.diagnostic)
        for address, label in [(0x57a500, 'required57A500'), (0x4d4ff0, 'required4D4FF0'), (0x4d3990, 'required4D3990'), (0x4d4900, 'required4D4900')]:
            self.seam(address, lambda m, label=label: self.stop(label))
        for address, label in [(0x522cc0, 'can_target522CC0'), (0x51d560, 'mobile51D560'), (0x51efb0, 'remove51EFB0'), (0x4d0cd0, 'unregister4D0CD0'), (0x524037, 'owner_zone_entry4')]:
            self.observe(address, lambda m, label=label: self.events.append({'call': label}))

    def actor(self, name, gtid):
        actor = self.alloc(0x200)
        self.w(actor, 0xa273f0)
        self.w(actor + 0x14, gtid)
        self.actors[name] = actor
        self.names[actor] = name
        self.u.mem_write(self.reg + 4 + 2 * (gtid & 0xffff), struct.pack('<H', gtid >> 16))
        self.w(self.reg + 0x20004 + 4 * (gtid & 0xffff), actor)
        return actor

    def singly(self, head, words, label):
        previous = 0
        for word in words:
            node = self.alloc(8)
            self.w(node, word)
            self.nodes[node] = label
            if previous:
                self.w(previous + 4, node)
            else:
                self.w(head, node)
            previous = node
        self.w(head + 4, previous)

    def doubly(self, head, rows):
        previous = 0
        for row in rows:
            node = self.alloc(16)
            self.w(node, row['gtid'])
            self.w(node + 4, previous)
            self.w(node + 12, row['priority'])
            self.nodes[node] = 'engage'
            if previous:
                self.w(previous + 8, node)
            else:
                self.w(head, node)
            previous = node
        self.w(head + 4, previous)

    def stop(self, label):
        self.cut = label
        self.events.append({'cutoff': label})
        self.u.emu_stop()

    def free(self, _):
        address = self.arg(1)
        self.events.append({'free': self.nodes[address], 'gtid': self.r(address)})
        self.ret(0)

    def diagnostic(self, _):
        # Call arguments are eagerly captured by actual51EFB0 before this entry.
        assert self.arg(3) == 0xf02
        actor = self.arg(8)
        self.events.append({'diagnostic': 'remove', 'owner': self.arg(6), 'target': self.arg(7), 'resolved': self.names.get(actor)})
        if self.case.get('diagnostic_required', False):
            self.stop('required404F90')
        else:
            self.ret(0)

    def list(self, head, stride):
        out = []
        node = self.r(head)
        previous = 0
        while node:
            if stride == 8:
                assert self.r(node + 4) == previous
                out.append({'gtid': self.r(node), 'priority': struct.unpack('<i', struct.pack('<I', self.r(node + 12)))[0]})
            else:
                out.append(self.r(node))
            previous = node
            node = self.r(node + stride)
        assert self.r(head + 4) == previous
        return out

    def run(self):
        try:
            self.call(0x523ea0, ecx=self.engage, count=20000)
        except AssertionError:
            if self.cut is None:
                raise
        assert not self.null_accesses, self.null_accesses
        return {'result': self.cut or 'returned', 'targets': self.list(self.engage + 0x10, 8), 'attackers': {name: self.list(actor + 0x118, 4) for name, actor in self.actors.items()}, 'events': self.events}


def cases():
    stale = {'gtid': 0x10022, 'priority': -3}
    live = {'gtid': 0x20033, 'priority': 5}
    org = {'gtid': 0x30044, 'priority': 17}
    out = []
    for current in [False, True]:
        out.append({'name': 'empty', 'current': current, 'targets': []})
        for required in [False, True]:
            out.append({'name': 'stale', 'current': current, 'targets': [stale], 'diagnostic_required': required})
            out.append({'name': 'generation_mismatch', 'current': current, 'targets': [stale], 'actors': {'stale': {'gtid': stale['gtid'], 'generation_mismatch': True}}, 'diagnostic_required': required})
            for gate in ['owner104', 'targetD4', 'mobile34']:
                c = {'name': gate, 'current': current, 'targets': [stale, live], 'actors': {'live': {'gtid': live['gtid'], 'attackers': [0x9988, 0x10011, 0x10011]}}, 'diagnostic_required': required}
                if gate == 'owner104':
                    c['owner_flags104'] = 4
                elif gate == 'targetD4':
                    c['actors']['live']['d4'] = 7
                else:
                    c['actors']['live']['mobile_target'] = 0x10011
                out.append(c)
        out.append({'name': 'positive_acceptance_cutoff', 'current': current, 'targets': [live], 'actors': {'live': {'gtid': live['gtid']}}, 'forced_target': live['gtid']})
        for required in [False, True]:
            out.append({'name': 'attached_duplicate_organization', 'current': current, 'owner_flags104': 4, 'targets': [live, org], 'actors': {'live': {'gtid': live['gtid'], 'membership': {'organization': 'organization'}}, 'organization': {'gtid': org['gtid']}}, 'diagnostic_required': required})
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--write', action='store_true')
    args = ap.parse_args()
    machines = []
    pins = {}
    for image, name in [(n.original, 'original'), (n.modified, 'v14')]:
        for pc in n.PRECISIONS:
            rows = []
            for case in cases():
                machine = Probe(image, pc, case)
                rows.append({'input': case, 'output': machine.run()})
                machine.verify_v14()
                pins.update(machine.visited)
            machines.append({'image': name, 'precision': pc, 'rows': rows})
    for machine in machines[1:]:
        assert machine['rows'] == machines[0]['rows']
    doc = {**n.header(__file__, __doc__), 'machines': machines, 'executed_instructions': n.instructions(pins)}
    path = HERE / 'native-engage-refresh-prefix.json'
    if args.write:
        path.write_text(json.dumps(doc, indent=1) + '\n')
    else:
        assert json.loads(path.read_text()) == doc
    print(json.dumps({'machines': 6, 'rows': sum(len(m['rows']) for m in machines), 'pins': len(pins), 'sha256': n.sha(path.read_bytes())}))


if __name__ == '__main__':
    main()
