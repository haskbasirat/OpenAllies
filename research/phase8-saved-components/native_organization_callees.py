"""Original GActor 4CED10, GActor 4CEBB0 (with Organization 57A330, member
iterator 4364C0 and Performer 57E010) and Body 561760, under Unicorn
(native_machine.py). Seams (recorded): Performer 57DCF0, Updatable 50B220
(has scheduled) and 50B1C0 (unschedule). Actors are synthetic objects with
the native component table at +60 (+74 Performer, +7C Organization, +80
Element, +A4 Experience); Organization members at +28/+2C, Element +10.
"""
from pathlib import Path
import sys, json, random
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_EAX


class World(Machine):
    def __init__(self, actors):
        super().__init__(b'')
        self.objects = {}
        self.labels = {}
        for name, a in actors.items():
            p = self.alloc(0x200)
            self.objects[name] = p
            self.labels[p] = name
        for name, a in actors.items():
            p = self.objects[name]
            if a.get('rank') is not None:
                e = self.alloc(0x20)
                self.w(e + 0xc, a['rank'])
                self.w(p + 0xa4, e)
            if 'element_org' in a:
                el = self.alloc(0x40)
                self.w(el + 0x10, self.objects[a['element_org']] if a['element_org'] else 0)
                self.w(p + 0x80, el)
            if a.get('performer'):
                perf = self.alloc(0x40)
                self.labels[perf] = name + '.Performer'
                self.w(p + 0x74, perf)
            if 'members' in a:
                org = self.alloc(0xdc)
                arr = self.alloc(4 * max(1, len(a['members'])))
                for i, m in enumerate(a['members']):
                    self.w(arr + 4 * i, self.objects[m] if m else 0)
                self.w(org + 0x28, arr)
                self.w(org + 0x2c, len(a['members']))
                self.w(p + 0x7c, org)
            if 'body' in a:
                body = self.alloc(0x40)
                self.labels[body] = name + '.Body'
                self.w(p + 0x60, body)
        self.scheduled = {name: a.get('body') for name, a in actors.items()}

        def performer(m):  # 57DCF0(rank) thiscall on Performer
            m.record('57DCF0', this=m.labels[m.u.reg_read(UC_X86_REG_ECX)], rank=m.arg(0))
            m.ret(4)
        self.seam(0x57dcf0, performer)

        def has(m):  # 50B220(event, data) thiscall on Body+8
            body = m.labels[m.u.reg_read(UC_X86_REG_ECX) - 8]
            r = bool(m.scheduled[body[:-5]])
            m.record('50B220', this=body, event=m.arg(0), data=m.arg(1), result=r)
            m.ret(8, int(r))
        self.seam(0x50b220, has)

        def remove(m):  # 50B1C0(event, data) thiscall on Body+8
            body = m.labels[m.u.reg_read(UC_X86_REG_ECX) - 8]
            m.record('50B1C0', this=body, event=m.arg(0), data=m.arg(1))
            m.ret(8, 1)
        self.seam(0x50b1c0, remove)


def cases():
    rng = random.Random(0x4cebb0)
    rows = []
    # 4CED10: own Experience, Element organization Experience, neither.
    for own in (None, 0, 0x51):
        for element in ('none', 'null_org', 'org_with', 'org_without'):
            actors = {'a': {'rank': own}, 'org_with': {'rank': 0x61}, 'org_without': {}}
            if element == 'null_org':
                actors['a']['element_org'] = None
            elif element.startswith('org'):
                actors['a']['element_org'] = element
            rows.append({'function': '4CED10', 'actor': 'a', 'actors': actors})
    # 4CEBB0: performer gate, organization recursion (nested), experience skips.
    for new in (0, 0x77):
        for performer in (False, True):
            actors = {
                'top': {'performer': performer, 'members': ['m0', None, 'm1', 'sub', 'm2']},
                'm0': {'performer': True},
                'm1': {'performer': True, 'rank': 5},
                'm2': {'performer': False},
                'sub': {'performer': True, 'members': ['s0', 's1'], 'rank': None},
                's0': {'performer': True},
                's1': {'performer': True, 'rank': 0},
            }
            rows.append({'function': '4CEBB0', 'actor': 'top', 'old': rng.getrandbits(32), 'new': new,
                         'actors': actors})
            rows.append({'function': '4CEBB0', 'actor': 'm0', 'old': 3, 'new': new, 'actors': actors})
    rows.append({'function': '4CEBB0', 'actor': 'e', 'old': 1, 'new': 2,
                 'actors': {'e': {'performer': True, 'members': []}}})
    # 561760: owner null / nonnull, scheduled or not.
    for owner in (0, 1):
        for scheduled in (False, True):
            rows.append({'function': '561760', 'actor': 'b', 'owner': owner,
                         'actors': {'b': {'body': scheduled}, 'own': {}}})
    return rows


def main():
    out = []
    for c in cases():
        m = World(c['actors'])
        a = m.objects[c['actor']]
        if c['function'] == '4CED10':
            r = m.call(0x4ced10, a, [])
            native = {'result': r}
        elif c['function'] == '4CEBB0':
            m.call(0x4cebb0, a, [c['old'], c['new']])
            native = {}
        else:
            body = m.r(a + 0x60)
            m.call(0x561760, body, [m.objects['own'] if c['owner'] else 0])
            native = {}
        assert m.outcome is None, m.outcome
        native['calls'] = m.calls
        out.append({'input': c, 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__,
           'visited_instructions': len(VISITED), 'cases': out}
    path = HERE / 'native-organization-callees.json'
    path.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
