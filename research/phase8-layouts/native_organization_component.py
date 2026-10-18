"""Original ActorData OrganizationComponent block (464BD0 466C4F..46707C)
and the deferred element link 460000 under Unicorn.

For every case the ORIGINAL 48D660 first loads the case's Property /
GroupFormation / Formation / OrganizationLayout files exactly as in
native_layout_data.py (same machine, same seams). Then:

* fragment: the original 464BD0 code from 466C4F to 46707C runs on a
  fixture ActorData object (0x600 bytes, 0xA5 fill; +8 IDS, +174 = 0,
  +314 = 0x20-byte block) with EDI = the actor root node from the original
  parser, EBX = 0 and a frame whose [esp+2C] holds an empty CString; it
  stops at 46707C or at the first throw. Template
  Templates/Components/template_organization.tgi is parsed with the others.
* links: the original 475230 walk is reproduced by calling each deferred
  reference's vf0 461750 -> 460000 natively, head first from GameData+824.
  4BDBA0 (Thing manager lookup, class 0x2B) is a seam returning a fixture
  element ActorData (+174 flags, +1D4 property list of the case's registered
  Property objects, +558 research list word, +17C/+180 list) or null.

Output: native-organization-component.json
"""
from pathlib import Path
import sys, json, struct, hashlib, multiprocessing, time
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_layout_data as NL
from layout_machine import DATA, native_windows, P, V, H, signed
from unicorn.x86_const import *

ORG_TEMPLATE = 'Templates/Components/template_organization.tgi'
TEMPLATES = NL.TEMPLATES + [ORG_TEMPLATE]
FRAGMENT, FRAGMENT_END = 0x466c4f, 0x46707c
BIT_ELEMENT, BIT_MOBILE, BIT_RESEARCH, BIT_HQ = 1 << 8, 1 << 19, 1 << 21, 1 << 30


def f32(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


class OrgOracle(NL.LayoutOracle):
    def setup(self, precision):
        super().setup(precision)
        self.parse_file(ORG_TEMPLATE)
        self.elements = {}
        self.thing_lookups = []

    def hook(self, u, pc, size, user):
        if pc == 0x4bdba0:
            # thiscall Thing-manager lookup (ids, class 0x2B) ret 8: fixture.
            ids = self.raw_wstr(self.arg())
            cls = self.arg(1)
            obj = self.elements.get(ids.lower(), 0)
            self.thing_lookups.append({'ids': ids, 'class': hex(cls), 'found': bool(obj)})
            self.recorded.append({'event': 'thing_lookup', 'ids': ids, 'class': hex(cls), 'found': bool(obj)})
            self.ret(obj, pop=8)
            return
        super().hook(u, pc, size, user)

    def property_object(self, ids):
        for p in self.manager_entries(self.mgr[0x464]):
            if self.wstr(self.r(p + 8)).lower() == ids.lower():
                return p
        raise KeyError(ids)

    def make_list(self, values):
        head = 0
        for v in reversed(values):
            n = self.alloc(8)
            self.w(n, v)
            self.w(n + 4, head)
            head = n
        return head

    def read_list(self, head):
        out = []
        while head:
            out.append(self.r(head))
            head = self.r(head + 4)
        return out

    def make_element(self, name, spec):
        obj = self.alloc(0x600)
        self.u.mem_write(obj, b'\xa5' * 0x600)
        self.w(obj + 8, self.cstring(name))
        self.w(obj + 0x174, spec.get('flags', 0))
        self.w(obj + 0x1d4, self.make_list([self.property_object(p) for p in spec.get('properties', [])]))
        self.w(obj + 0x558, 0x1234 if spec.get('research_items') else 0)
        items = [0x5170000 + 0x10 * i for i in spec.get('list_17c', [])]
        head = self.make_list(items)
        self.w(obj + 0x17c, head)
        tail = head
        while tail and self.r(tail + 4):
            tail = self.r(tail + 4)
        self.w(obj + 0x180, tail)
        self.elements[spec.get('lookup', name).lower()] = obj
        return obj

    def run_fragment(self, actor_node, owner_ids, owner_flags, owner_17c):
        actor = self.alloc(0x600)
        self.u.mem_write(actor, b'\xa5' * 0x600)
        self.w(actor + 8, self.cstring(owner_ids))
        self.w(actor + 0x174, owner_flags)
        rampage = self.alloc(0x20)
        self.u.mem_write(rampage, b'\xa5' * 0x20)
        self.w(actor + 0x314, rampage)
        items = [0x5170000 + 0x10 * i for i in owner_17c]
        self.w(actor + 0x17c, self.make_list(items))
        self.w(actor + 0x180, 0)
        frame = self.alloc(0x400)
        self.u.mem_write(frame, b'\xa5' * 0x400)
        esp = frame + 0x100
        self.w(esp + 0x2c, self.cstring(''))
        self.thrown = None
        self.messages = []
        self.last_throw = None
        u = self.u
        u.reg_write(UC_X86_REG_ESP, esp)
        u.reg_write(UC_X86_REG_EBP, actor)
        u.reg_write(UC_X86_REG_EDI, actor_node)
        u.reg_write(UC_X86_REG_EBX, 0)
        u.reg_write(UC_X86_REG_ESI, 0x22334455)
        u.reg_write(UC_X86_REG_ECX, 0x33445566)
        u.emu_start(FRAGMENT, FRAGMENT_END, count=20_000_000)
        self.check_null()
        if self.thrown is None:
            assert u.reg_read(UC_X86_REG_EIP) == FRAGMENT_END, hex(u.reg_read(UC_X86_REG_EIP))
            assert u.reg_read(UC_X86_REG_ESP) == esp
        return actor

    def run_links(self):
        refs = []
        p = self.r(self.gd + 0x824)
        while p:
            refs.append(p)
            p = self.r(p + 8)
        out = []
        for ref in refs:
            out.append({'ids': self.wstr(self.r(ref + 4)), 'slot': self.r(ref + 0x10), 'vtable': hex(self.r(ref))})
        for ref in refs:
            self.last_throw = None
            r = self.thiscall(0x461750, ref, [])
            if 'thrown' in r:
                return out, self.last_throw or {'cxx': [e for e in self.recorded if e['event'] == 'cxx_throw'][-1:]}
        return out, None

    def dump_actor(self, actor, element_names):
        layouts = self.manager_entries(self.mgr[0x470])
        b = lambda off: self.u.mem_read(actor + off, 1)[0]
        rampage = self.r(actor + 0x314)
        out = {'flags_174': hex(self.r(actor + 0x174)), 'supply_used': b(0x2f4),
               'shortfall_damage_rate': self.r(actor + 0x2f8), 'company_group_priority': signed(self.r(actor + 0x320)),
               'morale_breakpoint': self.r(actor + 0x308), 'control_range': self.r(actor + 0x2fc),
               'operational_range': self.r(actor + 0x300), 'operational_range_squared': self.r(actor + 0x304)}
        lay = self.r(actor + 0x2e8)
        out['layout'] = layouts.index(lay) if lay in layouts else ('null' if lay == 0 else hex(lay))
        out.update({'entrenchment_time': self.r(actor + 0x2ec), 'rampages': b(0x310),
                    'rampage_wait_chance': self.r(rampage), 'rampage_points_to_try': signed(self.r(rampage + 4)),
                    'rampage_direction': [self.r(rampage + 8), self.r(rampage + 0xc)],
                    'rampage_distance': [self.r(rampage + 0x10), self.r(rampage + 0x14)],
                    'rampage_max_detached_ratio': self.r(rampage + 0x18), 'rampage_aggressive': self.u.mem_read(rampage + 0x1c, 1)[0],
                    'recruit_time': self.r(actor + 0x318), 'recruit_full': b(0x31c), 'in_supply_detached': b(0x31d)})
        lo = self.r(actor + 0x2f0)
        if lo and lo != 0xa5a5a5a5:
            assert self.r(lo) == 0xa232b8
            params = [self.r(self.r(lo + 0xc) + 4 * i) for i in range(self.r(lo + 0x10))]
            names = {v: k for k, v in element_names.items()}
            out['default_layout'] = {'layout': layouts.index(self.r(lo + 8)), 'count_10': self.r(lo + 0x10),
                                     'capacity_14': self.r(lo + 0x14),
                                     'parameters': [None if v == 0 else names.get(v, hex(v)) for v in params]}
        else:
            out['default_layout'] = None
        return out


ACTOR_TEMPLATE_FIELDS = ['supply_used', 'company_group_priority', 'shortfall_damage_rate', 'layout_IDS',
                         'entrenchment_time', 'control_range', 'operational_range', 'morale_breakpoint', 'rampages',
                         'rampage_wait_chance', 'rampage_points_to_try', 'rampage_direction', 'rampage_distance',
                         'rampage_max_detached_ratio', 'rampage_aggressive', 'recruit_time', 'recruit_full',
                         'in_supply_detached']

L2 = NL.layout('L2', [NL.slot('front', number=2, locked='true', required='false', name='F'),
                      NL.slot('support', number=1, required='false', name='S'),
                      NL.slot('captain', primary=True, name='C')],
               [NL.lformation('f_org', ['front', 'front', 'support', 'captain'])])
L3 = NL.layout('L3', [NL.slot('front', number=1, required='false', name='F'),
                      NL.slot('support', number=3, required='false', name='S'),
                      NL.slot('captain', primary=True, name='C')],
               [NL.lformation('f_org', ['front', 'support', 'support', 'support', 'captain'])])


def actor_text(fields, slots, ids='org_actor'):
    s = '[OrgActor]\r\n{\r\n\tIDS = %s\r\n\t[OrganizationComponent template=OrganizationComponent]\r\n\t{\r\n' % ids
    for k, v in fields.items():
        s += '\t\t%s = %s\r\n' % (k, v)
    for e in slots:
        s += '\t\t[Slot]\r\n\t\telement_IDS = %s\r\n' % e
    return s + '\t}\r\n}\r\n'


def cases():
    c = {}
    base = NL.base_files(NL.layout('L1', NL.BASE_SLOTS, [NL.BASE_FORM]), L2, L3)
    ok_elements = {'e_front': {'flags': BIT_ELEMENT, 'properties': ['front']},
                   'e_cap': {'flags': BIT_ELEMENT, 'properties': ['captain', 'scout']},
                   'e_support': {'flags': BIT_ELEMENT, 'properties': ['support']}}

    def add(name, fields, slots, elements=None, owner_flags=0, owner_17c=(), precisions=('Double53',)):
        for pc in precisions:
            c[name if len(precisions) == 1 else '%s_%s' % (name, pc)] = {
                'precision': pc, 'files': base, 'actor': actor_text(fields, slots),
                'elements': elements if elements is not None else ok_elements,
                'owner_flags': owner_flags, 'owner_17c': list(owner_17c)}

    add('template_defaults', {'layout_IDS': 'L1', 'entrenchment_time': 30}, ['e_front', 'e_cap'],
        precisions=('Single24', 'Double53', 'Extended64'))
    add('aa_ambient_values', {'layout_IDS': 'l1', 'entrenchment_time': 99999, 'morale_breakpoint': 8, 'rampages': 'true',
                              'control_range': 18, 'operational_range': 34, 'recruit_time': 30, 'recruit_full': 'false',
                              'in_supply_detached': 'false', 'rampage_wait_chance': '75%', 'rampage_direction': '-45,45',
                              'rampage_aggressive': 'false', 'rampage_distance': '.5, 4',
                              'rampage_max_detached_ratio': '25%', 'rampage_points_to_try': 50},
        ['e_front', 'e_cap'], precisions=('Single24', 'Double53', 'Extended64'))
    for i, (op, d) in enumerate([('10.1', '33.3,-12.7'), ('3.3', '0.1,179.9'), ('0.00000000000000000000001', '0.00000000000000000000000000000000000001,-0.00000000000000000000000000000000000001'),
                                 ('0.3', '7,11'), ('1.1', '-90,90')]):
        add('x87_rounding_%d' % i, {'layout_IDS': 'L1', 'entrenchment_time': 1, 'operational_range': op,
                                    'rampage_direction': d}, ['e_front', 'e_cap'],
            precisions=('Single24', 'Double53', 'Extended64'))
    # PC24 double rounding of subnormal binary32 products (found by an exact
    # rational search): x87 rounds to 24 bits with the extended exponent,
    # then fstp rounds again onto the subnormal grid.
    def dec(x):
        text = format(x, '.70f').rstrip('0')
        assert float(text) == x
        return text
    add('x87_pc24_square', {'layout_IDS': 'L1', 'entrenchment_time': 1,
                            'operational_range': dec(4.9021553608551e-20)}, ['e_front', 'e_cap'],
        precisions=('Single24', 'Double53', 'Extended64'))
    add('x87_pc24_direction', {'layout_IDS': 'L1', 'entrenchment_time': 1,
                               'rampage_direction': '%s,-%s' % (dec(1.630947068190191e-37), dec(3.3619916882840324e-37))},
        ['e_front', 'e_cap'], precisions=('Single24', 'Double53', 'Extended64'))
    add('morale_negative', {'layout_IDS': 'L1', 'entrenchment_time': 1, 'morale_breakpoint': -1}, ['e_front', 'e_cap'])
    add('morale_zero', {'layout_IDS': 'L1', 'entrenchment_time': 1, 'morale_breakpoint': 0}, ['e_front', 'e_cap'])
    add('control_zero', {'layout_IDS': 'L1', 'entrenchment_time': 1, 'control_range': 0}, ['e_front', 'e_cap'])
    add('control_negative', {'layout_IDS': 'L1', 'entrenchment_time': 1, 'control_range': -2}, ['e_front', 'e_cap'])
    add('operational_zero', {'layout_IDS': 'L1', 'entrenchment_time': 1, 'operational_range': 0}, ['e_front', 'e_cap'])
    add('operational_negative', {'layout_IDS': 'L1', 'entrenchment_time': 1, 'operational_range': -0.5}, ['e_front'])
    add('unknown_layout', {'layout_IDS': 'nolayout', 'entrenchment_time': 1}, ['e_front'])
    add('empty_layout', {'layout_IDS': 'null', 'entrenchment_time': 1}, ['e_front'])
    add('too_many_slots', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap', 'e_front'])
    add('locked_not_required', {'layout_IDS': 'L2', 'entrenchment_time': 1}, ['e_front', 'e_support', 'e_cap'])
    add('required_empty_element', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['null', 'e_cap'])
    add('remaining_required', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front'])
    add('optional_empty', {'layout_IDS': 'L3', 'entrenchment_time': 1}, ['null', 'e_support', 'e_cap'])
    add('optional_missing_tail', {'layout_IDS': 'L3', 'entrenchment_time': 1}, ['e_front'],
        elements={**ok_elements})
    # Links (460000).
    add('link_unknown_element', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['nobody', 'e_cap'])
    add('link_not_element', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap'],
        elements={**ok_elements, 'e_front': {'flags': 0, 'properties': ['front']}})
    add('link_lacks_property', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap'],
        elements={**ok_elements, 'e_cap': {'flags': BIT_ELEMENT, 'properties': ['front']}})
    add('link_packable_multi', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap'],
        elements={**ok_elements, 'e_front': {'flags': BIT_ELEMENT | BIT_MOBILE, 'properties': ['front']}})
    add('link_packable_single', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap'],
        elements={**ok_elements, 'e_cap': {'flags': BIT_ELEMENT | BIT_MOBILE, 'properties': ['captain']}})
    add('link_hq_research_items', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap'],
        elements={**ok_elements, 'e_cap': {'flags': BIT_ELEMENT | BIT_RESEARCH, 'properties': ['captain'],
                                           'research_items': True}}, owner_flags=BIT_HQ)
    add('link_hq_research_no_items', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap'],
        elements={**ok_elements, 'e_cap': {'flags': BIT_ELEMENT | BIT_RESEARCH, 'properties': ['captain']}},
        owner_flags=BIT_HQ)
    add('link_research_items_not_hq', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap'],
        elements={**ok_elements, 'e_cap': {'flags': BIT_ELEMENT | BIT_RESEARCH, 'properties': ['captain'],
                                           'research_items': True}})
    add('link_two_failures_walk_order', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['nobody', 'nobody2'])
    add('link_case_insensitive_lookup', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['E_FRONT', 'e_cap'])
    add('link_17c_merge', {'layout_IDS': 'L1', 'entrenchment_time': 1}, ['e_front', 'e_cap'],
        elements={**ok_elements, 'e_cap': {'flags': BIT_ELEMENT, 'properties': ['captain'], 'list_17c': [2]}},
        owner_17c=[1, 2, 3])
    return c


def run_case(args):
    try:
        return run_case_inner(args)
    except Exception:
        import traceback
        return args[0], {'harness_error': traceback.format_exc()}, {}


def run_case_inner(args):
    name, case = args
    m = OrgOracle()
    m.setup(case['precision'])
    for path, data in case['files']:
        m.process(path, data)
    parsed = m.parse_file('Fixture/actor.tgi', case['actor'].encode())
    actor_node = m.children(parsed['root'])[0]
    names = {}
    for ename, spec in case['elements'].items():
        names[ename] = m.make_element(ename, spec)
    m.recorded = []
    actor = m.run_fragment(actor_node, 'org_actor', case['owner_flags'], case['owner_17c'])
    fragment_throw = m.last_throw if m.thrown is not None else None
    cxx = [e for e in m.recorded if e['event'] == 'cxx_throw']
    result = {'fragment_thrown': m.thrown is not None, 'fragment_throw': fragment_throw,
              'fragment_cxx': cxx[-1] if (m.thrown is not None and fragment_throw is None and cxx) else None}
    result['refs'] = []
    result['link_throw'] = None
    if m.thrown is None:
        refs, link_throw = m.run_links()
        result['refs'] = refs
        result['link_throw'] = link_throw
    result['actor'] = m.dump_actor(actor, names)
    result['thing_lookups'] = m.thing_lookups
    result['element_17c'] = {n: [hex(v) for v in m.read_list(m.r(o + 0x17c))] for n, o in names.items()}
    result['element_17c_tail'] = {n: hex(m.r(m.r(o + 0x180))) if m.r(o + 0x180) else None for n, o in names.items()}
    result['layout_dump'] = m.dump()['layouts']
    return name, result, dict(m.visited)


def main():
    workers = 6
    if '--workers' in sys.argv:
        workers = int(sys.argv[sys.argv.index('--workers') + 1])
    cs = cases()
    if '--only' in sys.argv:
        keep = sys.argv[sys.argv.index('--only') + 1].split(',')
        cs = {k: v for k, v in cs.items() if k in keep}
    t0 = time.time()
    with multiprocessing.Pool(workers) as pool:
        results = pool.map(run_case, list(cs.items()))
    visited = {}
    rows = []
    for name, out, vis in results:
        if 'harness_error' in out:
            print('HARNESS ERROR', name, out['harness_error'], flush=True)
            raise SystemExit(1)
        visited.update(vis)
        case = cs[name]
        rows.append({'name': name, 'precision': case['precision'],
                     'files': [{'path': p, 'text': d.decode('latin-1')} for p, d in case['files']],
                     'actor': case['actor'], 'elements': case['elements'], 'owner_flags': case['owner_flags'],
                     'owner_17c': case['owner_17c'], 'result': out})
        print(name, case['precision'], 'fragment', out['fragment_throw'] or out['fragment_cxx'],
              'link', out['link_throw'], flush=True)
    doc = {'schema': 1, 'passed': True, 'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b),
           'templates': [{'path': t, 'sha256': hashlib.sha256((DATA / t).read_bytes()).hexdigest(),
                          'text': (DATA / t).read_bytes().decode('latin-1')} for t in TEMPLATES],
           'cases': rows, 'native_windows': native_windows(visited),
           'seams': ['Case files loaded by the original 48D660 as in native-layout-data.json (same seams).',
                     'Fragment 466C4F..46707C of the original 464BD0 on a fixture ActorData (0xA5 fill; +8 IDS, '
                     '+174 owner flags, +314 rampage block, +17C list) with EDI = the parsed actor root, EBX = 0.',
                     '475230 walk reproduced head first over GameData+824 calling the original vf0 461750 -> 460000; '
                     'a throw stops the walk (natively it escapes the data load).',
                     '4BDBA0 is a seam: fixture element ActorData objects by case-insensitive IDS (+174 flags, '
                     '+1D4 list of the case\'s registered Property objects, +558 word, +17C/+180 list).']}
    out = HERE / ('native-organization-component.json' if '--only' not in sys.argv else 'scratch-org.json')
    out.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(rows), 'seconds': round(time.time() - t0),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
