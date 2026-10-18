"""Original AA.exe OrganizationLayoutData / FormationData producer oracle.

Every case executes ORIGINAL instructions under Unicorn:
  * the original TGI parser 5BD080 builds the node trees from the shipped
    files (or the synthetic case texts) after the template files are parsed;
  * the original 48D660 root dispatch runs every file through the GameData
    +834 handler map to the original DataManagers (+464 Property 47EFD0,
    +470 OrganizationLayout 481740, +474 Formation 481A50, +478 GroupFormation
    4840A0); 48D660's own catch funclets run through the C++ EH seam;
  * DataManager vf0 483A40 -> vf1C (4818A0 / 481BB0 / 484200 / 48B580) ->
    create (481960 / 481C70 / 4842C0 / 48B640) -> type LoadData:
    4C3610 KKC_OrganizationLayoutData::LoadData (4C2690 ctor) with
    4C32B0 LayoutFormation load, 4C1BE0 CheckFormationMatch, 4C3110 formation
    index lookup; 4C34A0 KKC_FormationData::LoadData (4C2540 ctor) with
    4763C0 icon path; GroupFormation 49A660 type; Property 47D8D0 type with
    4C48C0 LoadData.

Seams (recorded in order, never executed): 4C42D0 KKC_ModifierPropertyData::
LoadData (property contents are the Property producer's), 5A77E0 texture load
(icon resource), 4B9F40 string-table lookup (fixture table), log leaves
404EA0..404F90 (recorded with arguments), 5AB1D0 path formatter (definitions
harness leaf). 6F7810 allocations are filled with 0xA5.

Output: native-layout-data.json
  usage: python native_layout_data.py [--workers N]
"""
from pathlib import Path
import sys, json, struct, hashlib, multiprocessing, time
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from layout_machine import Machine, DATA, native_windows, P, V, H, signed
from unicorn.x86_const import *

TEMPLATES = ['Game/resource_list.tgi', 'Templates/template_fx.tgi', 'Templates/template_ability_effect.tgi',
             'Templates/template_modifier.tgi', 'Templates/template_property.tgi', 'Templates/template_terrain.tgi',
             'Templates/template_group_formation.tgi', 'Templates/template_formation.tgi',
             'Templates/template_organization_layout.tgi', 'Templates/template_organization_layout_aa.tgi']
SHIPPED = (['Game/group_formations.tgi']
           + ['Properties/properties.tgi', 'Properties/properties_actor_states.tgi',
              'Properties/properties_actor_states_aa.tgi', 'Properties/properties_buildings.tgi',
              'Properties/properties_handicaps.tgi', 'Properties/properties_intrinsic.tgi',
              'Properties/properties_organization_slots.tgi', 'Properties/properties_stance.tgi',
              'Properties/properties_technologies.tgi', 'Properties/properties_terrains.tgi']
           + ['Organizations/Formations/regiment_formations.tgi']
           + ['Organizations/Layouts/' + n for n in [
               'herd_ambient.tgi', 'regiment_hq.tgi', 'regiment_hq_mobile.tgi', 'regiment_infantry.tgi',
               'regiment_mechanized.tgi', 'regiment_single_unit.tgi', 'regiment_vehicle_heavy.tgi',
               'regiment_vehicle_light.tgi']])
CW = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}
HANDLERS = {'Property': 0x464, 'OrganizationLayout': 0x470, 'Formation': 0x474, 'GroupFormation': 0x478}
STRING_TABLE = {'regiment_infantry_name': 'Infantry Regiment', 'fixture_key': 'Fixture Text'}


def sha(b):
    return hashlib.sha256(b).hexdigest()


class LayoutOracle(Machine):
    def setup(self, precision):
        self.u.reg_write(UC_X86_REG_FPCW, CW[precision])
        self.string_table = dict(STRING_TABLE)
        for t in TEMPLATES:
            self.parse_file(t)
        self.recorded = []
        self.lookups = []
        # GameData: keep the parser harness object (its +3BC markup registry)
        # but give it room for the +834 handler map.
        old = self.r(0xb36530)
        gd = self.alloc(0x1000)
        self.u.mem_write(gd, bytes(self.u.mem_read(old, 0x600)))
        self.w(0xb36530, gd)
        self.w(gd + 4, self.cstring(''))  # GameData+4 current data-file directory CString
        self.gd = gd
        self.build_managers()
        mp = gd + 0x834
        nb = 31
        buckets = self.alloc(4 * nb)
        self.w(mp, buckets)
        self.w(mp + 4, nb)
        for tag, off in HANDLERS.items():
            s = self.cstring(tag)
            bucket = self.thiscall(0x5043f0, mp, [s])['eax']
            e = self.alloc(12)
            self.w(e, s)
            self.w(e + 4, self.mgr[off])
            self.w(e + 8, self.r(buckets + 4 * bucket))
            self.w(buckets + 4 * bucket, e)
        self.watch_frames(0x48d684, 0xae0bc0)
        self.nodes = {}
        self.files = []

    # -- node identities ----------------------------------------------------
    def index_nodes(self, file_no, root):
        def walk(node, path):
            self.nodes[node] = {'file': file_no, 'path': path}
            for i, c in enumerate(self.children(node)):
                walk(c, path + [[i, self.wstr(self.r(c)), self.wstr(self.r(c + 4))]])
        for i, c in enumerate(self.children(root)):
            walk(c, [[i, self.wstr(self.r(c)), self.wstr(self.r(c + 4))]])

    def process(self, path, data):
        parsed = self.parse_file(path, data)
        file_no = len(self.files)
        self.index_nodes(file_no, parsed['root'])
        self.recorded.append({'event': 'file', 'file': file_no, 'path': path})
        self.eh_events = []
        self.last_throw = None
        before = len(self.recorded)
        r = self.thiscall(0x48d660, self.gd, [parsed['resource']])
        assert 'eax' in r, (path, r)
        caught = bool(self.eh_events)
        entry = {'path': path, 'returned': r['eax'] & 0xff, 'caught': caught,
                 'throw': self.last_throw if caught else None,
                 'gamedata_dir': self.wstr(self.r(self.gd + 4))}
        self.files.append(entry)
        return entry

    # -- dumps --------------------------------------------------------------
    def property_ref(self, p, embedded):
        if p == 0:
            return None
        if p in self.prop_ordinals:
            return {'registered': self.prop_ordinals[p]}
        if p in embedded:
            return embedded[p]
        return {'unknown': hex(p)}

    def localized(self, cstr_ptr):
        return self.wstr(cstr_ptr)

    def dump(self):
        props = self.manager_entries(self.mgr[0x464])
        self.prop_ordinals = {p: i for i, p in enumerate(props)}
        groups = self.manager_entries(self.mgr[0x478])
        group_ordinals = {p: i for i, p in enumerate(groups)}
        forms = self.manager_entries(self.mgr[0x474])
        form_ordinals = {p: i for i, p in enumerate(forms)}
        layouts = self.manager_entries(self.mgr[0x470])
        embedded = {}
        for i, f in enumerate(forms):
            if self.r(f + 0x24):
                embedded[self.r(f + 0x24)] = {'formation': i}
        for i, l in enumerate(layouts):
            if self.r(l + 0x50):
                embedded[self.r(l + 0x50)] = {'layout_broken': i}
        out = {'properties': [], 'group_formations': [], 'formations': [], 'layouts': []}
        for i, p in enumerate(props):
            assert self.r(p + 0xc) == i
            out['properties'].append({'ordinal': i, 'ids': self.wstr(self.r(p + 8)), 'vtable': hex(self.r(p)),
                                      'is_technology': self.u.mem_read(p + 0x5c, 1)[0]})
        for i, g in enumerate(groups):
            assert self.r(g + 0xc) == i
            out['group_formations'].append({'ordinal': i, 'ids': self.wstr(self.r(g + 8))})
        textures = {}
        for e in self.recorded:
            if e['event'] == 'texture':
                textures[int(e['handle'], 16)] = e['path']
        for i, f in enumerate(forms):
            assert self.r(f + 0xc) == i
            tex = self.r(f + 0x1c)
            grp = self.r(f + 0x20)
            out['formations'].append({
                'ordinal': i, 'ids': self.wstr(self.r(f + 8)), 'vtable': hex(self.r(f)),
                'name': self.localized(self.r(f + 0x10)), 'named': self.u.mem_read(f + 0x14, 1)[0],
                'organized': self.u.mem_read(f + 0x18, 1)[0], 'herding': self.u.mem_read(f + 0x19, 1)[0],
                'aggressive': self.u.mem_read(f + 0x1a, 1)[0],
                'icon': textures.get(tex) if tex else None, 'icon_word_zero': tex == 0,
                'group_formation': group_ordinals.get(grp) if grp else None,
                'group_formation_unknown': bool(grp) and grp not in group_ordinals,
                'property': self.property_ref(self.r(f + 0x24), embedded)})
        for i, l in enumerate(layouts):
            assert self.r(l + 0xc) == i
            entries = []
            base, n = self.r(l + 0x1c), self.r(l + 0x20)
            for k in range(n):
                e = base + 16 * k
                b = bytes(self.u.mem_read(e + 0xc, 3))
                entries.append({'name': self.localized(self.r(e)),
                                'property': self.property_ref(self.r(e + 4), embedded),
                                'number': signed(self.r(e + 8)), 'required': b[0], 'locked': b[1],
                                'morale_line': b[2]})
            gbase, gn = self.r(l + 0x28), self.r(l + 0x2c)
            groups_of = [signed(self.r(gbase + 4 * k)) for k in range(gn)]
            fbase, fn = self.r(l + 0x34), self.r(l + 0x38)
            formations = []
            for k in range(fn):
                lf = self.r(fbase + 4 * k)
                assert self.r(lf) == 0xa232a8 and self.r(lf + 8) == l
                fd = self.r(lf + 0xc)
                pbase, pn = self.r(lf + 0x10), self.r(lf + 0x14)
                positions = []
                for j in range(pn):
                    q = pbase + 12 * j
                    positions.append({'property': self.property_ref(self.r(q), embedded),
                                      'offset_bits': [self.r(q + 4), self.r(q + 8)]})
                formations.append({'formation': form_ordinals.get(fd) if fd else None,
                                   'formation_unknown': bool(fd) and fd not in form_ordinals,
                                   'positions': positions})
            out['layouts'].append({
                'ordinal': i, 'ids': self.wstr(self.r(l + 8)), 'vtable': hex(self.r(l)),
                'name': self.localized(self.r(l + 0x10)), 'named': self.u.mem_read(l + 0x14, 1)[0],
                'primary_slot': signed(self.r(l + 0x18)), 'entries': entries, 'slot_groups': groups_of,
                'formations': formations, 'default_formation': signed(self.r(l + 0x40)),
                'combat_formation': signed(self.r(l + 0x44)), 'retreat_formation': signed(self.r(l + 0x48)),
                'morale_count': signed(self.r(l + 0x4c)),
                'broken_property': self.property_ref(self.r(l + 0x50), embedded)})
        out['globals'] = {'max_slot_number_b3b6b0': signed(self.r(0xb3b6b0)),
                          'max_slot_groups_b3b6b4': signed(self.r(0xb3b6b4))}
        events = []
        for e in self.recorded:
            if e['event'] == 'property_load':
                obj = int(e['object'], 16)
                events.append({'event': 'property_load', 'property': self.property_ref(obj, embedded),
                               'node': self.nodes.get(int(e['node'], 16))})
            elif e['event'] == 'texture':
                events.append({'event': 'texture', 'path': e['path'], 'flag': e['flag']})
            else:
                events.append(e)
        out['events'] = events
        out['files'] = self.files
        return out


def run_case(args):
    try:
        return run_case_inner(args)
    except Exception as e:  # harness failure: reported, never a fixture row
        import traceback
        return args[0], {'harness_error': traceback.format_exc()}, {}


def run_case_inner(args):
    name, precision, files = args
    m = LayoutOracle()
    m.setup(precision)
    for path, data in files:
        m.process(path, data)
    out = m.dump()
    m.check_null()
    return name, out, dict(m.visited)


# ---------------------------------------------------------------- synthetic
PROPS = ''.join('[Property Template=SharedProperty]\r\n{\r\n\tIDS = %s\r\n\tname = "%s"\r\n\ticon = null\r\n}\r\n\r\n'
                % (i, i.title()) for i in ['captain', 'front', 'support', 'scout'])
GROUPS = ('[GroupFormation Template=GroupFormation]\r\n{\r\n\tids = g1\r\n\tname = "#fixture_key"\r\n'
          '\t[Position]\r\n\toffset = 0,6\r\n\t[Position]\r\n\toffset = -1.5,2\r\n}\r\n')


def formation(ids, organized='true', aggressive='true', herding=None, icon='Icons/a.png', group=None,
              prop=True, extra=''):
    s = '[Formation template=Formation]\r\n{\r\n\tIDS = %s\r\n\tname = "F %s"\r\n' % (ids, ids)
    if organized is not None:
        s += '\torganized = %s\r\n' % organized
    if aggressive is not None:
        s += '\taggressive = %s\r\n' % aggressive
    if herding is not None:
        s += '\therding = %s\r\n' % herding
    if icon is not None:
        s += '\ticon = %s\r\n' % icon
    if group is not None:
        s += '\tgroup_formation_ids = %s\r\n' % group
    s += extra
    if prop:
        s += ('\t[Property template=ModifierProperty]\r\n\t{\r\n\t\tname = "P %s"\r\n\t\ticon = /x.png\r\n'
              '\t\t[Modifier template=MultiplyModifier]\r\n\t\tattribute = SPEED_MAX\r\n\t\tamount = -.25\r\n\t}\r\n' % ids)
    return s + '}\r\n\r\n'


FORMS = (formation('f_org', group='g1') + formation('f_mob', organized='false', aggressive='false', herding='true',
                                                     icon='/abs/b.png', prop=False)
         + formation('f_org2', aggressive='false', icon='null', prop=False))


def slot(prop, number=None, primary=False, required=None, locked=None, morale=None, name='S'):
    s = '\t[Slot template=%s]\r\n\tproperty_IDS = %s\r\n' % ('PrimarySlot' if primary else 'LayoutSlot', prop)
    for k, v in [('number', number), ('required', required), ('locked', locked), ('morale_line', morale)]:
        if v is not None:
            s += '\t%s = %s\r\n' % (k, v)
    return s + '\tname = "%s"\r\n\r\n' % name


def lformation(ids, props, offsets=None):
    s = '\t[Formation]\r\n\t{\r\n\t\tformation_IDS = %s\r\n' % ids
    for i, p in enumerate(props):
        off = offsets[i] if offsets else '%d,%d' % (i, -i)
        s += '\t\t[Position template=FormationPosition]\r\n\t\tproperty_IDS = %s\r\n\t\toffset = %s\r\n' % (p, off)
    return s + '\t}\r\n'


def layout(ids, slots, formations, header='', template='AAOrganizationLayout', name=None):
    s = '[OrganizationLayout template=%s]\r\n{\r\n\tIDS = %s\r\n\tname = "%s"\r\n' % (template, ids, name or ids)
    return s + header + ''.join(slots) + ''.join(formations) + '}\r\n\r\n'


BASE_SLOTS = [slot('front', number=2, locked='true', name='Front'), slot('captain', primary=True, name='Cap')]
BASE_FORM = lformation('f_org', ['front', 'front', 'captain'], ['-1,+5.25', '+1.25,+5', '0,0'])


def base_files(*layout_texts, forms=FORMS, props=PROPS):
    return [('Fixture/props.tgi', props.encode()), ('Fixture/groups.tgi', GROUPS.encode()),
            ('Fixture/Formations/forms.tgi', forms.encode())] + [
        ('Fixture/Layouts/l%d.tgi' % i, t.encode()) for i, t in enumerate(layout_texts)]


def synthetic_cases():
    c = {}
    c['basic'] = base_files(layout('L1', BASE_SLOTS, [BASE_FORM]))
    c['two_layouts_one_file'] = [x for x in base_files()] + [(
        'Fixture/Layouts/both.tgi', (layout('A1', BASE_SLOTS, [BASE_FORM]) + layout('A2', BASE_SLOTS, [BASE_FORM])).encode())]
    c['default_combat_retreat'] = base_files(layout('L1', BASE_SLOTS, [
        BASE_FORM, lformation('f_mob', ['front', 'front', 'captain']), lformation('f_org2', ['front', 'front', 'captain'])],
        header='\tdefault_formation_ids = f_org2\r\n\tcombat_formation_IDS = F_ORG\r\n\tretreat_formation_IDS = f_mob\r\n'))
    c['retreat_last_organized'] = base_files(layout('L1', BASE_SLOTS, [
        BASE_FORM, lformation('f_org2', ['front', 'front', 'captain']), lformation('f_mob', ['front', 'front', 'captain'])]))
    c['retreat_none_organized'] = base_files(layout('L1', BASE_SLOTS, [lformation('f_mob', ['front', 'front', 'captain'])]))
    c['unknown_slot_property'] = base_files(layout('L1', [slot('front'), slot('bogus'), slot('captain', primary=True)],
                                                   [BASE_FORM]))
    c['empty_slot_property'] = base_files(layout('L1', [slot('front'), slot('null')], [BASE_FORM]))
    c['two_primary'] = base_files(layout('L1', [slot('captain', primary=True), slot('front', number=1),
                                               slot('captain', primary=True)], [BASE_FORM]))
    c['primary_zero_number_first'] = base_files(layout('L1', [
        slot('captain', primary=True, number=0), slot('front', number=2), slot('captain', primary=True)],
        [lformation('f_org', ['front', 'front', 'captain'])]))
    c['negative_and_zero_numbers'] = base_files(layout('L1', [
        slot('front', number=-1), slot('front', number=0, morale='false'), slot('support', number=3, required='false'),
        slot('captain', primary=True)], [lformation('f_org', ['support', 'support', 'support', 'captain'])]))
    c['morale_lines'] = base_files(layout('L1', [
        slot('front', number=3, morale='false'), slot('support', number=2, required='false'),
        slot('captain', primary=True, morale='false')],
        [lformation('f_org', ['front'] * 3 + ['support'] * 2 + ['captain'])]))
    c['unknown_formation'] = base_files(layout('L1', BASE_SLOTS, [lformation('nope', ['front', 'front', 'captain'])]))
    c['empty_formation_ids'] = base_files(layout('L1', BASE_SLOTS, [lformation('null', ['front', 'front', 'captain'])]))
    c['unknown_position_property'] = base_files(layout('L1', BASE_SLOTS, [lformation('f_org', ['front', 'bogus', 'captain'])]))
    c['formation_size_mismatch'] = base_files(layout('L1', BASE_SLOTS, [lformation('f_org', ['front', 'captain'])]))
    c['formation_property_mismatch'] = base_files(layout('L1', BASE_SLOTS, [lformation('f_org', ['front', 'support', 'captain'])]))
    c['formation_order_mismatch'] = base_files(layout('L1', BASE_SLOTS, [lformation('f_org', ['captain', 'front', 'front'])]))
    c['no_formation'] = base_files(layout('L1', BASE_SLOTS, []))
    c['default_not_used'] = base_files(layout('L1', BASE_SLOTS, [BASE_FORM], header='\tdefault_formation_ids = f_mob\r\n'))
    c['default_unknown'] = base_files(layout('L1', BASE_SLOTS, [BASE_FORM], header='\tdefault_formation_ids = zzz\r\n'))
    c['combat_unknown'] = base_files(layout('L1', BASE_SLOTS, [BASE_FORM], header='\tcombat_formation_IDS = zzz\r\n'))
    c['retreat_not_used'] = base_files(layout('L1', BASE_SLOTS, [BASE_FORM], header='\tretreat_formation_IDS = f_org2\r\n'))
    c['duplicate_layout_ids'] = base_files(layout('L1', BASE_SLOTS, [BASE_FORM]), layout('l1', BASE_SLOTS, [BASE_FORM]))
    c['throw_abandons_rest_of_file'] = base_files() + [(
        'Fixture/Layouts/both.tgi', (layout('B1', BASE_SLOTS, [lformation('nope', ['front'])])
                                     + layout('B2', BASE_SLOTS, [BASE_FORM])).encode())]
    c['many_slots_and_positions'] = base_files(layout('L1', [slot('front', number=9, locked='true'),
                                                            slot('support', number=4, required='false'),
                                                            slot('captain', primary=True)],
                                                      [lformation('f_org', ['front'] * 9 + ['support'] * 4 + ['captain']),
                                                       lformation('f_org2', ['front'] * 9 + ['support'] * 4 + ['captain'])]))
    c['case_insensitive_refs'] = base_files(layout('L1', [slot('FRONT', number=2), slot('Captain', primary=True)],
                                                   [lformation('F_Org', ['Front', 'front', 'CAPTAIN'])]))
    c['localized_names'] = base_files(layout('L1', [slot('front', number=2, name='#fixture_key'),
                                                    slot('captain', primary=True, name='#missing_key')],
                                             [BASE_FORM], name='#regiment_infantry_name'))
    # Formation-side cases.
    c['formation_unknown_group'] = base_files(forms=FORMS + formation('f_bad', group='nogroup'))
    c['formation_duplicate_ids'] = base_files(forms=FORMS + formation('F_ORG', prop=False))
    c['formation_icon_forms'] = base_files(forms=(formation('i1', icon='a/b/c.png', prop=False)
                                                  + formation('i2', icon='/lead/x.png', prop=False)
                                                  + formation('i3', icon='Icons\\back.png', prop=False)
                                                  + formation('i4', icon='/', prop=False)))
    # A value starting with '//' stays a value natively (5BDF80), not a comment.
    c['formation_icon_double_slash'] = base_files(forms=formation('i5', icon='//lead/x.png', prop=False))
    c['formation_flags'] = base_files(forms=(formation('h1', organized='false', aggressive='true', herding='true', prop=False)
                                             + formation('h2', organized='true', aggressive='false', herding='false', prop=False)))
    # Property manager mode 1: a repeated IDS re-loads the first definition
    # (its stored name is empty, so 47D960 inside the seamed 4C42D0 would not
    # throw natively either).
    dup = '[Property Template=SharedProperty]\r\n{\r\n\tIDS = %s\r\n\tname = ""\r\n\ticon = null\r\n}\r\n'
    c['duplicate_property_ids_reload'] = base_files(layout('L1', BASE_SLOTS, [BASE_FORM]),
                                                    props=PROPS + dup % 'extra' + dup % 'EXTRA')
    c['no_broken_property_template'] = base_files(
        '[Template BareLayout]\r\n{\r\n\tstring IDS\r\n\tlocalized name\r\n\tstring default_formation_ids = NULL\r\n'
        '\tstring combat_formation_IDS = NULL\r\n\tstring retreat_formation_IDS = NULL\r\n'
        '\t[list Slot template=LayoutSlot]\r\n\t[list Formation template=LayoutFormation]\r\n}\r\n\r\n'
        + layout('L1', BASE_SLOTS, [BASE_FORM], template='BareLayout'))
    c['layout_before_formations'] = [('Fixture/props.tgi', PROPS.encode()),
                                     ('Fixture/Layouts/l0.tgi', layout('L1', BASE_SLOTS, [BASE_FORM]).encode()),
                                     ('Fixture/groups.tgi', GROUPS.encode()),
                                     ('Fixture/Formations/forms.tgi', FORMS.encode())]
    return c


def main():
    workers = 4
    if '--workers' in sys.argv:
        workers = int(sys.argv[sys.argv.index('--workers') + 1])
    shipped_files = [(p, (DATA / p).read_bytes()) for p in SHIPPED]
    jobs = [('shipped', pc, shipped_files) for pc in CW]
    cases = synthetic_cases()
    jobs += [(name, 'Double53', files) for name, files in cases.items()]
    if '--only' in sys.argv:
        keep = sys.argv[sys.argv.index('--only') + 1].split(',')
        jobs = [j for j in jobs if j[0] in keep and (j[0] != 'shipped' or j[1] == 'Double53')]
    t0 = time.time()
    with multiprocessing.Pool(workers) as pool:
        results = pool.map(run_case, jobs)
    visited = {}
    shipped, synth = [], []
    for (name, pc, files), (_, out, vis) in zip(jobs, results):
        visited.update(vis)
        if 'harness_error' in out:
            print('HARNESS ERROR', name, out['harness_error'], flush=True)
            raise SystemExit(1)
        if name == 'shipped':
            shipped.append({'precision': pc, 'files': [{'path': p, 'sha256': sha(d)} for p, d in files],
                            'result': out})
        else:
            synth.append({'name': name, 'precision': pc,
                          'files': [{'path': p, 'text': d.decode('latin-1')} for p, d in files], 'result': out})
        print(name, pc, 'files', [(f['path'], f['returned'], f['caught']) for f in out['files']][-3:], flush=True)
    doc = {
        'schema': 1, 'passed': True, 'producer_sha256': sha(Path(__file__).read_bytes()),
        'machine_sha256': sha((HERE / 'layout_machine.py').read_bytes()),
        'original_sha256': H(P.b), 'v14_sha256': H(V.b),
        'templates': [{'path': t, 'sha256': sha((DATA / t).read_bytes()),
                       'text': (DATA / t).read_bytes().decode('latin-1')} for t in TEMPLATES],
        'string_table': STRING_TABLE,
        'shipped': shipped, 'cases': synth,
        'native_windows': native_windows(visited),
        'seams': [
            'Template files parsed first by the original parser in index order (only the closure the layout/'
            'formation/property templates need): ' + ', '.join(TEMPLATES) + '.',
            'Each file runs through the original 48D660 (GameData+834 handler map with the four original '
            'DataManagers built by their constructors; arguments as in 48A8E0). 48D660 catch funclets run '
            'through the C++ EH seam (inner-frame destructors skipped: reference counts only).',
            '4C42D0 KKC_ModifierPropertyData::LoadData is a recorded seam (object, node): property contents are '
            'not part of this producer. It is reached from the Property type LoadData 4C48C0 (whose is_technology '
            'read runs natively), from 4C3610 (BrokenProperty) and from 4C34A0 (Formation Property).',
            '5A77E0 texture-resource load is a recorded seam returning a fixture handle (icon is presentation).',
            '4B9F40 string-table lookup is a fixture table (string_table); unknown keys return the static empty '
            'default B43B48 as natively.',
            'Leveled log calls 404EA0..404F90 are recorded leaves with their printf arguments.',
            '6F7810 allocations are filled with 0xA5; free 6F7430 is a no-op leaf.',
            'Precision: shipped data at PC24/53/64 (FPCW); synthetic cases at PC53.',
        ],
    }
    out = HERE / ('native-layout-data.json' if '--only' not in sys.argv else 'scratch-layout-data.json')
    out.write_text(json.dumps(doc, indent=1))
    print(json.dumps({'cases': len(synth), 'shipped': len(shipped), 'seconds': round(time.time() - t0),
                      'sha256': sha(out.read_bytes())}))


if __name__ == '__main__':
    main()
