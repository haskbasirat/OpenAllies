"""Phase8 round-2 oracle corpus: the 546AF0 element types no campaign mission uses.

Run:  PYTHONPATH=<reconstruction/tools/python-packages> python native_extended.py
Writes native-extended.json. Same seam policy as harness.py, plus the seams of
these leaves (EXTRA_SEAMS) and recorded stores into watched engine fields
(hero entry +18, the fidget game-variable cell), which appear in the call
sequence as {"call": "Store", "this": object, "args": [offset, value]}.
"""
from pathlib import Path
import copy, itertools, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402
from unicorn import UC_HOOK_MEM_WRITE  # noqa: E402
from unicorn.x86_const import *  # noqa: E402,F403

P = h.param
bits = h.core.bits
A1, A2, K1, K2 = 0x10003, 0x10004, 0x10002, 0x10005

EXTRA_SEAMS = {
    0x4ed820: ('ViewAdjustSunDimmer', 8, 'ww', False),   # World+4C view
    0x79f5d0: ('SaiChangeProfile', 8, 'oo', False),      # [B36528]
    0x747630: ('PlayMusic', 4, 'o', False),              # [B36548]
    0x50d230: ('UiVfxOnActor', 8, 'oo', False),          # World+130
    0x40fca0: ('VarPermission', 8, 'wx', False),         # B39690 (kind, B43B40)
    0x76f370: ('SetGameSpeed', 4, 'w', False),           # [B36554]
    0x579750: ('OrganizationModifyMorale', 8, 'ww', True),
    0x4d12c0: ('Actor4D12C0', 0, '', True),
    0x4f6760: ('World4F6760', 8, 'ow', False),           # World
    0x4ee5c0: ('ActorTeleport', 4, 'v', True),
    0x4eeb40: ('MemberTeleport', 8, 'ww', True),
    0x4f03c0: ('World4F03C0', 0, '', False),
    0x76ac60: ('Players76AC60', 0, '', False),           # [B36550]
    0x765e00: ('Player765E00', 0, '', True),
    0x416370: ('AudioLookup', 4, 't', False),            # GameData+584
    0x4e1130: ('HeroEntryInit', 4, 'o', True),           # this = new 0x24 block
    0x85d840: ('Random', 0, 'o', False),                 # cdecl (World+D4 generator)
}
h.SEAMS.update(EXTRA_SEAMS)
h.VIRTUALS[('actor', 0x24)] = ('DisplayName', 4, 'x')
HERO_ENTRY = 0x4e2a60
HERO_LIST = {0x4e4c30: ('KingdomAddHero', 4, 1), 0x4e2cb0: ('KingdomRemoveHero', 8, 2)}
FOG_QUERY = 0x554dd0
LOG_410C40 = 0x410c40
WORLD_RNG = 0x4f02b0
HERO_AT = 0x4e13f0


class Machine(h.Machine):
    def __init__(self, c):
        super().__init__(c)
        # Manager+40/+44 string array (54C240).
        strings = c.get('strings', [])
        arr = self.alloc(4 * max(1, len(strings)))
        for i, s in enumerate(strings):
            self.w(arr + 4 * i, self.string(s))
        self.w(h.MANAGER + 0x40, arr)
        self.w(h.MANAGER + 0x44, len(strings))
        # [[B39AB0]+24] fidget variable cell with its +28 flag word.
        cell = self.alloc(0x40)
        self.w(cell + 0x28, c.get('fidget_flags', 0))
        self.u.mem_write(cell, bytes([c.get('fidget_value', 0x5a)]))
        self.w(h.GAMEVARS + 0x24, cell)
        self.names_by_ptr[cell] = 'fidget_var'
        # GameData+C ActorData bank (random heroes): +24 array, +28 count.
        if c.get('actor_data_bank'):
            self.w(h.GAMEDATA + 0xc, self.objects[c['actor_data_bank']])
        # GameData+584 audio bank (its +20 name is read by the 410C40 error log).
        bank = self.alloc(0x40)
        self.w(bank + 0x20, self.string('AudioBank'))
        self.w(h.GAMEDATA + 0x584, bank)
        self.names_by_ptr[bank] = 'audio_bank'
        self.watch(cell, 0, 1)
        for name, offset in c.get('watch', []):
            if name in self.objects:
                self.watch(self.objects[name], offset, 4)

    def element_struct(self, e):
        """The element object exactly as the phase7 core run() builds it."""
        q = self.alloc(20)
        definition = self.alloc(0x60)
        self.w(definition + 12, e['opcode'])
        self.call(0x4c6e40, definition)
        self.w(q, definition)
        self.w(q + 4, len(e['parameters']))
        self.w(q + 12, self.triggerptr[0])
        params = self.alloc(0x34 * len(e['parameters']))
        self.w(q + 8, params)
        for i, prm in enumerate(e['parameters']):
            self.parameter(prm, params + i * 0x34)
        return q

    def w(self, a, v):
        # While a prelude ran, keep the register strings it left: the base
        # run() would reset them before the main element's 546AF0 call.
        if getattr(self, 'preserve_registers', False) and h.CONTEXT <= a < h.CONTEXT + 8 * 0x24 \
                and (a - h.CONTEXT) % 0x24 in (4, 8):
            return
        super().w(a, v)

    def run(self):
        prelude = self.c.get('prelude', [])
        if prelude:
            # Manager+2C register file: fresh registers own empty strings
            # (54DE30), then every prelude element converts into them.
            for i in range(8):
                self.w(h.CONTEXT + i * 0x24 + 4, self.string(''))
                self.w(h.CONTEXT + i * 0x24 + 8, self.string(''))
            for e in prelude:
                self.call(0x546af0, self.element_struct(e), [h.CONTEXT])
            self.preserve_registers = True
        # A genuine native null dereference (page-0 guard, not an unmapped heap
        # address) is recorded as a fault with the seam calls made before it.
        try:
            return super().run()
        except Exception as e:
            cause = e
            while cause is not None and 'native null-page access' not in str(cause):
                cause = cause.__cause__ or cause.__context__
            if cause is None:
                raise
            return {'fault': str(cause), 'calls': self.calls}

    def special(self, name, this, args):
        if name == 'FindPlacement' and isinstance(self.returns.get('FindPlacement'), list):
            spec = self.returns['FindPlacement'].pop(0) if self.returns['FindPlacement'] else {'ok': 0}
            for i, w in enumerate(spec.get('pos', [])):
                self.w(self.arg(0) + i * 4, w)
            for i, w in enumerate(spec.get('facing', [])):
                self.w(self.arg(1) + i * 4, w)
            return spec.get('ok', 0)
        if name == 'GameVarString' and 'by_arg' in self.returns.get('GameVarString', {}):
            out = self.arg(0)
            self.w(out, self.string(self.returns['GameVarString']['by_arg'].get(args[0], '')))
            return out
        if name == 'HeroEntryInit':
            self.names_by_ptr[this] = 'new_entry'
            return this
        if name == 'AudioLookup':
            return self.value(self.returns.get('AudioLookup', {}).get('by_arg', {}).get(args[0], 0))
        return super().special(name, this, args)

    def watch(self, base, offset, size):
        def store(u, access, address, sz, value, _):
            self.calls.append({'call': 'Store', 'this': self.describe(base),
                               'args': [hex(address - base), value & ((1 << (8 * sz)) - 1)]})
        self.u.hook_add(UC_HOOK_MEM_WRITE, store, begin=base + offset, end=base + offset + size - 1)

    def hook(self, u, a, n, x):
        if a == 0x546af0:
            self.preserve_registers = False
        if a == WORLD_RNG:
            rng = h.WORLD + 0xd4
            self.names_by_ptr[rng] = 'world_rng'
            self.record('WorldRng', 0, [], False)
            self.ret(0, rng)
            return
        if a == HERO_AT:
            kingdom = u.reg_read(UC_X86_REG_ECX) - 0x1fc
            args = [self.arg(0)]
            self.record('KingdomHeroAt', kingdom, args, True)
            self.ret(4, self.give('KingdomHeroAt', kingdom, args))
            return
        if a == HERO_ENTRY:
            kingdom = u.reg_read(UC_X86_REG_ECX) - 0x1fc
            args = [self.describe(self.arg(0)), self.arg(1)]
            self.record('KingdomHeroEntry', kingdom, args, True)
            self.ret(8, self.give('KingdomHeroEntry', kingdom, args))
            return
        if a in HERO_LIST:
            name, pop, nargs = HERO_LIST[a]
            kingdom = u.reg_read(UC_X86_REG_ECX) - 0x1fc
            args = [self.describe(self.arg(0))] + [self.arg(i) for i in range(1, nargs)]
            args[0] = 0 if self.arg(0) == 0 else args[0]
            self.record(name, kingdom, args, True)
            self.ret(pop, 0)
            return
        if a == FOG_QUERY:
            # Record the query and leave the tile iterator empty (+28 > +10):
            # the tile pass itself is the seam.
            q = self.arg(1)
            area = self.r(q) - 0x68
            self.record('FogRestoreTiles', 0, [self.describe(area), [self.r(q + 4 * i) for i in range(1, 5)]], False)
            this = u.reg_read(UC_X86_REG_ECX)
            self.w(this + 0x28, 1)
            self.w(this + 0x10, 0)
            self.ret(0x10, this)
            return
        if a == 0x4e1130:
            self.names_by_ptr[u.reg_read(UC_X86_REG_ECX)] = 'new_entry'
        if a == LOG_410C40:
            self.diags.append(hex(a))
            self.ret(0)
            return
        if h.VSTUB <= a < h.VSTUB + 0x10000 and self.stub_map[a] == ('actor', 0x24):
            this = u.reg_read(UC_X86_REG_ECX)
            self.record('DisplayName', this, [], True)
            out = self.arg(0)
            self.w(out, self.string('name:' + (self.names_by_ptr.get(this) or '?')))
            self.ret(4, out)
            return
        super().hook(u, a, n, x)


h.Machine = Machine


def base():
    return {
        'time_bits': bits(10), 'precision': 'Double53',
        'variables': {'counter': bits(2.0), 'level': bits(1.75), 'zero': 0, 'neg': bits(-3.0)},
        'triggers': [{'id': 10, 'enabled': 1, 'activation_bits': bits(1)}],
        'objects': {
            'actor_a': {'kind': 'actor', 'slot': 3, 'gen': 1, 'words': {'0x8': 0x1800020}},
            'actor_b': {'kind': 'actor', 'slot': 4, 'gen': 1, 'words': {'0x8': 0x1}},
            'kingdom_a': {'kind': 'kingdom', 'slot': 2, 'gen': 1, 'size': 0x400},
            'kingdom_b': {'kind': 'kingdom', 'slot': 5, 'gen': 1, 'size': 0x400},
            'area_a': {'kind': 'area', 'slot': 6, 'gen': 1},
            'camera_a': {'kind': 'camera', 'slot': 7, 'gen': 1},
            'music_a': {'kind': 'audio'},
            'sai_a': {'kind': 'data'},
            'vfx_a': {'kind': 'data'},
            'hero_data': {'kind': 'data'},
            'formation_a': {'kind': 'data'},
            'formation_b': {'kind': 'data'},
        },
        'kingdoms': {'kingdom_a': {'objectives': [], 'result': 0, 'team_members': None},
                     'kingdom_b': {'objectives': [], 'result': 0, 'team_members': None}},
        'manager': {'context_word': 0, 'subjects': [], 'condition_kingdom': 0, 'event_kingdom': 0, 'events': []},
        'returns': {
            'NamedActor': {'by_arg': {'actor_a': '@actor_a', 'actor_b': '@actor_b'}},
            'NamedKingdom': {'by_arg': {'kingdom_a': '@kingdom_a', 'kingdom_b': '@kingdom_b'}},
            'IsArea': {'area_a': 1},
            'IsCamera': {'camera_a': 1},
            'IsActorData': {'hero_data': 1, 'vfx_a': 1},
            'IsVfx': {'vfx_a': 1},
            'DataByIds:audio': {'by_arg': {'music_a': '@music_a'}},
            'DataByIds:sai': {'by_arg': {'sai_a': '@sai_a'}},
            'DataByIds:formation': {'by_arg': {'formation_a': '@formation_a', 'formation_b': '@formation_b'}},
            'DataByIds:actor_data': {'by_arg': {'hero_data': '@hero_data', 'vfx_a': '@vfx_a'}},
        },
    }


ROWS = []


def add(name, opcode, params, precision=None, **changes):
    c = base()
    for key, value in changes.items():
        if isinstance(value, dict) and isinstance(c.get(key), dict):
            merged = copy.deepcopy(c[key])
            for k, v in value.items():
                if isinstance(v, dict) and isinstance(merged.get(k), dict):
                    merged[k] = {**merged[k], **v}
                else:
                    merged[k] = v
            c[key] = merged
        else:
            c[key] = value
    c['element'] = h.element(opcode, params)
    for pc in (precision or ['Double53']):
        row = copy.deepcopy(c)
        row['precision'] = pc
        row['name'] = f'{name}_{pc}'
        ROWS.append(row)


ACTORS = [('named_a', 'mi_actor_named', 'actor_a'), ('named_missing', 'mi_actor_named', 'nobody'),
          ('id_a', 'mi_actor_id', A1), ('id_stale', 'mi_actor_id', 0x20003), ('condition', 'mi_actor_condition', 0)]
KINGDOMS = [('named_a', 'mi_kingdom_named', 'kingdom_a'), ('named_missing', 'mi_kingdom_named', 'nokingdom'),
            ('id_a', 'mi_kingdom_id', K1), ('condition', 'mi_kingdom_condition', 0), ('event', 'mi_kingdom_event', 0)]
CTX = {'context_word': A2, 'subjects': [A1], 'condition_kingdom': '@kingdom_b', 'event_kingdom': '@kingdom_a', 'events': []}
VALUES = [('f2', 'mi_fixed', bits(2.0)), ('fneg', 'mi_fixed', bits(-1.5)), ('int3', 'mi_int', 3),
          ('var', 'mi_variable', 'level'), ('varmissing', 'mi_variable', 'nothing'), ('fzero', 'mi_fixed', 0),
          ('fnegzero', 'mi_fixed', 0x80000000)]


def actor_param(v):
    return P('ma_actor', v[1], v[2])


def kingdom_param(v):
    return P('ma_kingdom', v[1], v[2])


def value_param(v):
    return P('ma_value', v[1], v[2])


# 4 cutscene_complete (event) ----------------------------------------------------
for strings in [[], ['intro'], ['other', 'intro'], ['INTRO'], ['intro2']]:
    for nm in ['intro', '', 'Intro']:
        add(f'cutscene4_{len(strings)}_{"-".join(strings)}_{nm or "empty"}', 4, [P('ma_string', 'mi_string', nm)],
            strings=strings)

# 11 actor_comparison_morale -----------------------------------------------------
ORG = {'kind': 'misc', 'words': {'0x5c': bits(40.0), '0x60': bits(80.0)}}
for a in ACTORS:
    for kind in range(7):
        for vn, vi, vv in [('half', 'mi_fixed', bits(50.0)), ('int', 'mi_int', 49), ('third', 'mi_fixed', bits(33.333))]:
            add(f'morale11_{a[0]}_{kind}_{vn}', 11, [actor_param(a), P('ma_compare', 'mi_compare', kind), P('ma_value', vi, vv)],
                objects={'org_a': ORG, 'actor_a': {'words': {'0x8': 0x1800020, '0x7c': '@org_a'}}}, manager=CTX)
add('morale11_no_org', 11, [actor_param(ACTORS[0]), P('ma_compare', 'mi_compare', 2), value_param(VALUES[0])])
for pc in ['Single24', 'Double53', 'Extended64']:
    add(f'morale11_precision', 11, [actor_param(ACTORS[0]), P('ma_compare', 'mi_compare', 4), P('ma_value', 'mi_fixed', bits(33.333))],
        precision=[pc], objects={'org_a': {'kind': 'misc', 'words': {'0x5c': bits(26.6664), '0x60': bits(80.0)}},
                                 'actor_a': {'words': {'0x8': 0x1800020, '0x7c': '@org_a'}}})

# 20 actor_in_formation ----------------------------------------------------------
for a in ACTORS:
    for fn in ['formation_a', 'formation_b', 'missing']:
        for cur in ['@formation_a', 0, 'noform']:
            objs = {'actor_a': {'words': {'0x8': 0x1800020, '0x7c': '@org_a'}},
                    'org_a': {'kind': 'misc', 'words': {'0x74': '@form_slot' if cur != 'noform' else 0}},
                    'form_slot': {'kind': 'misc', 'words': {'0xc': cur if cur != 'noform' else 0}}}
            add(f'formation20_{a[0]}_{fn}_{cur}', 20, [actor_param(a), P('ma_formation', 'mi_formation', fn)],
                objects=objs, manager=CTX)
add('formation20_no_org', 20, [actor_param(ACTORS[0]), P('ma_formation', 'mi_formation', 'formation_a')])

# 21 kingdom_has_compare_value_primary_resource --------------------------------
for k in KINGDOMS:
    for kind in range(6):
        for amount in [bits(2.4), bits(2.5), bits(-0.5), bits(-0.75), bits(3.0), bits(16777215.0), bits(1e-30)]:
            add(f'resource21_{k[0]}_{kind}_{amount:08x}', 21,
                [kingdom_param(k), P('ma_compare', 'mi_compare', kind), P('ma_value', 'mi_fixed', bits(2.0))],
                resources=[1, 0, 0],
                objects={'kingdom_a': {'words': {'0x1c4': '@res_a'}}, 'res_a': {'kind': 'misc', 'words': {'0x0': bits(9.0), '0x4': amount}}},
                manager=CTX)
add('resource21_no_primary', 21, [kingdom_param(KINGDOMS[0]), P('ma_compare', 'mi_compare', 2), value_param(VALUES[0])],
    resources=[1, 1], objects={'kingdom_a': {'words': {'0x1c4': '@res_a'}}, 'res_a': {'kind': 'misc'}})
for pc in ['Single24', 'Double53', 'Extended64']:
    add('resource21_precision', 21, [kingdom_param(KINGDOMS[0]), P('ma_compare', 'mi_compare', 2), P('ma_value', 'mi_fixed', bits(16777216.0))],
        precision=[pc], resources=[0],
        objects={'kingdom_a': {'words': {'0x1c4': '@res_a'}}, 'res_a': {'kind': 'misc', 'words': {'0x0': bits(16777215.0)}}})

# 24 kingdom_has_no_settlements -------------------------------------------------
for k in KINGDOMS:
    for count in [0, 1, 0xffffffff, 0x80000000, 7]:
        add(f'settle24_{k[0]}_{count:x}', 24, [kingdom_param(k)],
            objects={'kingdom_a': {'words': {'0x2d4': count}}}, manager=CTX)

# 26 hero_attached ---------------------------------------------------------------
for k in KINGDOMS:
    for dn in ['hero_data', 'missing']:
        for entry in ['@entry_alive', '@entry_dead', 0]:
            add(f'hero26_{k[0]}_{dn}_{entry}', 26, [P('ma_hero', 'mi_hero_named', dn), kingdom_param(k)],
                objects={'entry_alive': {'kind': 'misc', 'words': {'0x10': A1}},
                         'entry_dead': {'kind': 'misc', 'words': {'0x10': 0x20003}}},
                returns={'KingdomHeroEntry': entry}, manager=CTX)

# 35 actor_set_experience_max ----------------------------------------------------
for k in KINGDOMS[:3]:
    for item, dn in [('mi_hero_named', 'hero_data'), ('mi_hero_named', 'missing'),
                     ('mi_hero_random', 0), ('mi_actor_data_any', 0)]:
        for entry in ['@entry_a', 0]:
            for vn, vi, vv in VALUES[:4]:
                extra = {}
                add(f'expmax35_{k[0]}_{item}_{dn}_{entry}_{vn}', 35,
                    [kingdom_param(k), P('ma_hero', item, dn), P('ma_value', vi, vv)],
                    objects={'entry_a': {'kind': 'misc', 'words': {'0x18': 0x11111111}}},
                    returns={'KingdomHeroEntry': entry}, watch=[['entry_a', 0x18]], **extra)

# 37 actor_modify_morale ---------------------------------------------------------
UNIT = {
    'org': {'actor_a': {'words': {'0x8': 0x1800020, '0x7c': '@org_a'}}, 'org_a': {'kind': 'misc'}},
    'group': {'actor_a': {'words': {'0x8': 0x1800020, '0x80': '@grp'}}, 'grp': {'kind': 'misc', 'words': {'0x10': '@actor_b'}},
              'actor_b': {'words': {'0x8': 1, '0x7c': '@org_b'}}, 'org_b': {'kind': 'misc'}},
    'group_null': {'actor_a': {'words': {'0x8': 0x1800020, '0x80': '@grp'}}, 'grp': {'kind': 'misc'}},
    'none': {},
}
for un, objs in UNIT.items():
    for a in ACTORS[:4]:
        for v in VALUES + [('nan', 'mi_fixed', 0x7fc00000)]:
            extra = {'rust_unsupported': 'non-finite trigger value (54D9E0 domain)'} if v[0] == 'nan' else {}
            add(f'morale37_{un}_{a[0]}_{v[0]}', 37, [actor_param(a), value_param(v)], objects=objs, **extra)

# 41 global_toggle_fidgets -------------------------------------------------------
for flags in [0, 2, 5, 9, 0xd, 1, 4, 8, 0xf]:
    for perm in [0, 1, [1, 0], [0, 1]]:
        for b in [0, 1]:
            add(f'fidget41_{flags:x}_{perm}_{b}', 41, [P('ma_boolean', 'mi_boolean', b)], fidget_flags=flags,
                returns={'VarPermission': perm})
add('fidget41_wrong_item', 41, [P('ma_boolean', 'mi_int', 1)])

# 45 global_adjust_sun_dimmer ----------------------------------------------------
for (an, ai, av), (bn, bi, bv) in itertools.product(VALUES, VALUES[:4]):
    add(f'sun45_{an}_{bn}', 45, [P('ma_value', ai, av), P('ma_value', bi, bv)])

# 65 sai_change_profile_for_kingdom ---------------------------------------------
for k in KINGDOMS:
    for sn in ['sai_a', 'missing']:
        add(f'sai65_{k[0]}_{sn}', 65, [P('ma_sai_data', 'mi_sai_data', sn), kingdom_param(k)], manager=CTX)

# 68 game_speed_set (binary32 store of the FYL2X quotient) ----------------------
SPEEDS = [1.0, 2.0, 0.5, 4.0, 1.5, 3.0, 10.0, 100.0, 0.25, 0.1, 1e-3, 7.0, 1.1, 0.9, 1e30, 1e-30,
          1.401298464324817e-45, 3.4028234663852886e+38, -1.0, 0.0, -0.0]
for sp in SPEEDS:
    add(f'speed68_{sp!r}', 68, [P('ma_value', 'mi_fixed', bits(sp))], precision=['Single24', 'Double53', 'Extended64'])
for n in [1, 2, 3, 5, 16, 0]:
    add(f'speed68_int{n}', 68, [P('ma_value', 'mi_int', n)])
add('speed68_var', 68, [P('ma_value', 'mi_variable', 'level')])

# 84 vfx_on_actor ----------------------------------------------------------------
for a in ACTORS:
    for vn in ['vfx_a', 'hero_data', 'missing']:
        add(f'vfx84_{a[0]}_{vn}', 84, [P('ma_vfx', 'mi_vfx', vn), actor_param(a)], manager=CTX,
            returns={'IsVfx': {'vfx_a': 1, 'hero_data': 0}})

# 88 play_music ------------------------------------------------------------------
for mn in ['music_a', 'missing', '']:
    add(f'music88_{mn or "empty"}', 88, [P('ma_music', 'mi_music', mn)])

# --- round 2b: commands, teleports, heroes, music, camera, fog ------------------
LOC = ('loc', 'mi_location', [bits(10.5), bits(-3.25), bits(1.0)])
LOCS = [LOC, ('loc2', 'mi_location', [bits(-0.0), bits(1e-3), bits(-7.0)]), ('noloc', 'mi_int', 1)]
AREA = ('area', 'mi_area', 0x10006)
AREAS = [AREA, ('noarea', 'mi_area', 0x20006)]
OWN = {'Owner': {'actor_a': '@kingdom_a', 'actor_b': '@kingdom_b', 'actor_c': '@kingdom_a'}}


def area_objs(refs, center=(bits(1.0), bits(2.0), bits(3.0))):
    objs = {'area_a': {'kind': 'area', 'slot': 6, 'gen': 1,
                       'words': {'0x20': center[0], '0x24': center[1], '0x28': center[2]},
                       'lists': {'0x84': [f'@ref{i}' for i in range(len(refs))]}}}
    for i, gtid in enumerate(refs):
        objs[f'ref{i}'] = {'kind': 'misc', 'words': {'0x0': gtid}}
    return objs


ACTOR_C = {'kind': 'actor', 'slot': 8, 'gen': 1, 'words': {'0x8': 0x40}}
A3 = 0x10008

# 25 hero_attached_to_organization -----------------------------------------------
MEMBERS = {'org_a': {'kind': 'misc', 'words': {'0x2c': 3, '0x28': '@mem_arr'}},
           'mem_arr': {'kind': 'misc', 'words': {'0x0': '@m1', '0x4': 0, '0x8': '@m2'}},
           'm1': {'kind': 'misc', 'words': {'0x4': '@vfx_a'}}, 'm2': {'kind': 'misc', 'words': {'0x4': '@hero_data'}}}
for a in ACTORS:
    for dn in ['hero_data', 'vfx_a', 'missing']:
        for org in ['@org_a', 0]:
            objs = dict(MEMBERS)
            objs['actor_a'] = {'words': {'0x8': 0x1800020, '0x7c': org}}
            add(f'heroorg25_{a[0]}_{dn}_{org}', 25, [P('ma_hero', 'mi_hero_named', dn), actor_param(a)], objects=objs,
                manager=CTX)
for count in [0, 1, 2, 0xffffffff]:
    objs = dict(MEMBERS)
    objs['org_a'] = {'kind': 'misc', 'words': {'0x2c': count, '0x28': '@mem_arr'}}
    objs['actor_a'] = {'words': {'0x8': 0x1800020, '0x7c': '@org_a'}}
    add(f'heroorg25_count_{count:x}', 25, [P('ma_hero', 'mi_hero_named', 'hero_data'), actor_param(ACTORS[0])], objects=objs)
add('heroorg25_null_array', 25, [P('ma_hero', 'mi_hero_named', 'hero_data'), actor_param(ACTORS[0])],
    objects={'org_a': {'kind': 'misc', 'words': {'0x2c': 1}}, 'actor_a': {'words': {'0x8': 0x1800020, '0x7c': '@org_a'}}})

# 32 kingdom_actors_in_area_move_to_location ----------------------------------------
for k in KINGDOMS[:3]:
    for an, ai, av in AREAS:
        for ln, li, lv in LOCS:
            for refs in [[A1, A2, A3], [], [A2]]:
                objs = area_objs(refs)
                objs.update({'actor_c': ACTOR_C, 'cmd_move': {'kind': 'data'}})
                add(f'areamove32_{k[0]}_{an}_{ln}_{len(refs)}', 32, [kingdom_param(k), P('ma_area', ai, av), P('ma_location', li, lv)],
                    objects=objs, returns={**OWN, 'IsArea': {'area_a': 1}, 'ProcessCommand': 1},
                    command_defs={'5': 'cmd_move'})
add('areamove32_stale', 32, [kingdom_param(KINGDOMS[0]), P('ma_area', *AREA[1:]), P('ma_location', *LOC[1:])],
    objects={**area_objs([A1, 0x20003]), 'cmd_move': {'kind': 'data'}},
    returns={**OWN, 'IsArea': {'area_a': 1}}, command_defs={'5': 'cmd_move'})

# 39 actor_teleports_to_location -------------------------------------------------
PLACE_OK = {'ok': 1, 'pos': [bits(11.0), bits(-4.0)], 'facing': [bits(0.0), bits(1.0)]}
PLACE_KEEP = {'ok': 1}
PLACE_FAIL = {'ok': 0}
TELE_SOLO = {'actor_a': {'words': {'0x8': 0x1800020, '0x4': '@hero_data', '0x2c': bits(1.0), '0x30': bits(0.0)}}}
TELE_ORG = {
    'actor_a': {'words': {'0x8': 0x1800020, '0x4': '@hero_data', '0x2c': bits(1.0), '0x30': bits(0.0), '0x7c': '@org_t'}},
    'org_t': {'kind': 'misc', 'words': {'0x2c': 3, '0x28': '@tarr', '0x78': '@toffs', '0x40': '@actor_b'}},
    'tarr': {'kind': 'misc', 'words': {'0x0': '@actor_b', '0x4': 0, '0x8': '@actor_c'}},
    'toffs': {'kind': 'misc', 'words': {'0x0': bits(0.5), '0x4': bits(-0.5), '0x10': bits(1e-8), '0x14': bits(3.0)}},
    'actor_b': {'words': {'0x8': 1, '0x4': '@hero_data', '0x2c': bits(0.25), '0x30': bits(0.75)}},
    'actor_c': {**ACTOR_C, 'words': {'0x8': 0x40, '0x4': '@vfx_a', '0x2c': bits(-1.0), '0x30': bits(0.0)}},
}
for a in ACTORS[:4]:
    for ln, li, lv in LOCS:
        for setup_name, objs in [('solo', TELE_SOLO), ('org', TELE_ORG)]:
            for pn, places in [('ok', [PLACE_OK, PLACE_OK, PLACE_KEEP]), ('keep', [PLACE_KEEP] * 3), ('fail', [PLACE_FAIL]),
                               ('memberfail', [PLACE_OK, PLACE_FAIL])]:
                add(f'teleport39_{a[0]}_{ln}_{setup_name}_{pn}', 39, [actor_param(a), P('ma_location', li, lv)],
                    objects=objs, returns={'FindPlacement': places}, watch=[['actor_a', 8], ['actor_b', 8], ['actor_c', 8]],
                    precision=['Single24', 'Double53'] if pn == 'ok' else None)

# 33 kingdom_actors_in_area_teleport_to_location ------------------------------------
for k in KINGDOMS[:3]:
    for ln, li, lv in LOCS[:2]:
        for skip in [0, 1]:
            objs = area_objs([A1, 0x20003, A2, A3])
            objs.update(copy.deepcopy(TELE_ORG))
            objs['actor_a']['words'].update({'0x20': bits(5.0), '0x24': bits(6.5), '0x28': bits(7.25)})
            objs['actor_c'] = {**ACTOR_C, 'words': {'0x8': 0x40, '0x4': '@vfx_a', '0x20': bits(-1.0), '0x24': bits(0.1),
                                                    '0x28': bits(1e20), '0x80': '@grp'}}
            objs['grp'] = {'kind': 'misc', 'words': {'0x10': '@actor_b' if skip else 0}}
            add(f'areatele33_{k[0]}_{ln}_{skip}', 33, [kingdom_param(k), P('ma_area', *AREA[1:]), P('ma_location', li, lv)],
                objects=objs, returns={**OWN, 'IsArea': {'area_a': 1}, 'Actor4D12C0': {'actor_a': 0, 'actor_c': skip},
                                       'FindPlacement': [PLACE_OK, PLACE_OK, PLACE_KEEP, PLACE_FAIL, PLACE_OK]},
                watch=[['actor_a', 8], ['actor_b', 8], ['actor_c', 8]], precision=['Single24', 'Double53'])

# 38 actor_guard_location ---------------------------------------------------------
for a in ACTORS:
    for ln, li, lv in LOCS:
        for r in [0, 1, 0x100]:
            add(f'guard38_{a[0]}_{ln}_{r}', 38, [actor_param(a), P('ma_location', li, lv)],
                objects={'cmd_guard': {'kind': 'data'}}, command_defs={'2': 'cmd_guard'}, returns={'ProcessCommand': r},
                manager=CTX)
add('guard38_no_definition', 38, [actor_param(ACTORS[0]), P('ma_location', *LOC[1:])])

# 48 / 49 kingdom_gains_hero / kingdom_loses_hero ---------------------------------
for op in [48, 49]:
    for k in KINGDOMS[:3]:
        for item, dn in [('mi_hero_named', 'hero_data'), ('mi_hero_named', 'missing'), ('mi_actor_data_named', 'hero_data')]:
            for entry in ['@entry_free', '@entry_held', '@entry_busy', 0]:
                add(f'hero{op}_{k[0]}_{item}_{dn}_{entry}', op, [kingdom_param(k), P('ma_hero', item, dn)],
                    objects={'entry_free': {'kind': 'misc'}, 'entry_held': {'kind': 'misc', 'words': {'0x8': 1, '0xc': 5}},
                             'entry_busy': {'kind': 'misc', 'words': {'0x10': A1}}},
                    returns={'KingdomHeroEntry': entry}, manager=CTX)
    add(f'hero{op}_random_no_bank', op, [kingdom_param(KINGDOMS[0]), P('ma_hero', 'mi_hero_random', 0)])

# 89 play_game_music / 90 stop_music ---------------------------------------------
for player in ['@player_a', 0]:
    for set_ in ['@mset', 0]:
        for music in ['@music_a', 0]:
            for var in ['obs_music', '', 'unknown']:
                add(f'gamemusic89_{player}_{set_}_{music}_{var or "empty"}', 89, [],
                    objects={'player_a': {'kind': 'misc'}, 'pl': {'kind': 'misc', 'words': {'0x234': set_}},
                             'mset': {'kind': 'misc', 'words': {'0x38': music}}},
                    returns={'Players76AC60': player, 'Player765E00': {'player_a': '@pl', '*': 0} if player else 0,
                             'GameVarString': {'by_arg': {'ObserverGameMusic': var}},
                             'AudioLookup': {'by_arg': {'obs_music': '@music_a'}}})
for var in ['silence', '', 'unknown']:
    add(f'stopmusic90_{var or "empty"}', 90, [],
        returns={'GameVarString': {'by_arg': {'AudioMusicSilent': var}}, 'AudioLookup': {'by_arg': {'silence': '@music_a'}}})

# 106 camera_follows_actor / 113 camera_shake --------------------------------------
NAMED = {'kingdom_a': {'words': {'0x18': '$Kingdom A'}}, 'kingdom_b': {'words': {'0x18': '$Ké B'}}}
for k in KINGDOMS:
    for a in ACTORS:
        add(f'follow106_{k[0]}_{a[0]}', 106, [kingdom_param(k), actor_param(a)], objects=NAMED, manager=CTX)
for k in KINGDOMS:
    for vn, vi, vv in VALUES[:5] + [('tiny', 'mi_fixed', bits(1e-7)), ('big', 'mi_fixed', bits(3.0e12))]:
        for cut in [0, 1]:
            add(f'shake113_{k[0]}_{vn}_{cut}', 113, [kingdom_param(k), P('ma_value', vi, vv)], objects=NAMED,
                returns={'World4F03C0': cut}, manager=CTX)

# 111 fog_restored_in_area -----------------------------------------------------------
for an, ai, av in AREAS + [('notarea', 'mi_area', A1)]:
    add(f'fog111_{an}', 111, [P('ma_area', ai, av)],
        objects={'area_a': {'kind': 'area', 'slot': 6, 'gen': 1,
                            'words': {'0x20': bits(10.0), '0x24': bits(20.0), '0x2c': bits(4.5), '0x30': bits(-0.0)}}})

# --- round 3: random heroes (item 28) through the persistent register file ------
HERO_BITS = (1 << 17) | (1 << 8)
BANK = {
    'bank': {'kind': 'misc', 'words': {'0x24': '@bank_arr', '0x28': 4}},
    'bank_arr': {'kind': 'misc', 'words': {'0x0': '@d1', '0x4': '@d2', '0x8': '@d3', '0xc': '@d4'}},
    'd1': {'kind': 'data', 'words': {'0x174': HERO_BITS, '0x2e4': 1}},
    'd2': {'kind': 'data', 'words': {'0x174': 1 << 17, '0x2e4': 1}},
    'd3': {'kind': 'data', 'words': {'0x174': HERO_BITS | 0x40, '0x2e4': 0x100}},
    'd4': {'kind': 'data', 'words': {'0x174': HERO_BITS, '0x2e4': 0xff}},
}
HERO_RET = {'IsActorData': {'hero_data': 1, 'vfx_a': 1, 'd1': 1, 'd2': 1, 'd3': 1, 'd4': 0}}
for has in [[0, 0, 0], ['@entry_x', 0, 0], ['@entry_x', '@entry_x', '@entry_x'], [0, '@entry_x', 0]]:
    for r in [0, 1, 7, 0xffffffff]:
        add(f'randomhero48_{"-".join(str(x) for x in has)}_{r:x}', 48,
            [kingdom_param(KINGDOMS[0]), P('ma_hero', 'mi_hero_random', 0)],
            objects={**BANK, 'entry_x': {'kind': 'misc'}}, actor_data_bank='bank',
            returns={**HERO_RET, 'KingdomHeroEntry': has, 'Random': [r]})
for mask in [HERO_BITS, 0]:
    objs = copy.deepcopy(BANK)
    for d in ('d1', 'd4'):
        objs[d]['words']['0x174'] = mask
    add(f'randomhero48_none_{mask:x}', 48, [kingdom_param(KINGDOMS[0]), P('ma_hero', 'mi_hero_random', 0)],
        objects=objs, actor_data_bank='bank', returns={**HERO_RET, 'Random': [3]})
ENTRIES = {
    'h1': {'kind': 'misc', 'words': {'0x4': '@d1', '0x8': 1, '0xc': 5}},
    'h2': {'kind': 'misc', 'words': {'0x4': '@d1', '0xc': 5}},
    'h3': {'kind': 'misc', 'words': {'0x4': '@d4', '0x10': 0}},
    'h4': {'kind': 'misc', 'words': {'0x4': '@d2'}},
    'h5': {'kind': 'misc', 'words': {'0x4': 0}},
}
for count, at in [(5, ['@h1', '@h2', '@h3', '@h4', '@h5']), (3, ['@h2', 0, '@h4']), (0, []), (2, ['@h3', '@h1'])]:
    for r in [0, 1, 2, 0x80000001]:
        objs = {**BANK, **ENTRIES, 'kingdom_a': {'words': {'0x218': count}}}
        add(f'randomhero49_{count}_{"-".join(str(x) for x in at)}_{r:x}', 49,
            [kingdom_param(KINGDOMS[0]), P('ma_hero', 'mi_hero_random', 0)], objects=objs,
            returns={**HERO_RET, 'KingdomHeroAt': list(at), 'Random': [r]})
# The register file persists between elements: a non-text hero item reads the
# string an earlier element left in the same register.
NAMED35 = [kingdom_param(KINGDOMS[0]), P('ma_hero', 'mi_hero_named', 'hero_data'), P('ma_value', 'mi_fixed', bits(2.0))]
for op in [35, 48, 49]:
    add(f'register_prelude_{op}', op, [kingdom_param(KINGDOMS[0]), P('ma_hero', 'mi_hero_random', 0)],
        prelude=[h.element(35, NAMED35)],
        objects={**BANK, **ENTRIES, 'entry_a': {'kind': 'misc'}, 'kingdom_a': {'words': {'0x218': 2}}},
        actor_data_bank='bank',
        returns={**HERO_RET, 'KingdomHeroEntry': ['@entry_a', 0, 0, 0], 'KingdomHeroAt': ['@h3', '@h1'], 'Random': [1]},
        watch=[['entry_a', 0x18]])
add('register_prelude_25', 25, [P('ma_hero', 'mi_hero_random', 0), actor_param(ACTORS[0])],
    prelude=[h.element(26, [P('ma_hero', 'mi_hero_named', 'hero_data'), kingdom_param(KINGDOMS[0])])],
    objects={**MEMBERS, 'actor_a': {'words': {'0x8': 0x1800020, '0x7c': '@org_a'}}},
    returns={'KingdomHeroEntry': 0})
add('register_prelude_other_text', 26, [P('ma_hero', 'mi_hero_random', 0), kingdom_param(KINGDOMS[0])],
    prelude=[h.element(65, [P('ma_sai_data', 'mi_sai_data', 'sai_a'), kingdom_param(KINGDOMS[0])])],
    returns={'KingdomHeroEntry': '@entry_alive'}, objects={'entry_alive': {'kind': 'misc', 'words': {'0x10': A1}}})
add('register_prelude_two', 35, [kingdom_param(KINGDOMS[0]), P('ma_hero', 'mi_hero_random', 0), P('ma_value', 'mi_int', 4)],
    prelude=[h.element(35, NAMED35), h.element(41, [P('ma_boolean', 'mi_boolean', 1)]),
             h.element(84, [P('ma_vfx', 'mi_vfx', 'vfx_a'), actor_param(ACTORS[0])])],
    objects={'entry_a': {'kind': 'misc'}}, returns={'KingdomHeroEntry': '@entry_a', 'IsVfx': {'vfx_a': 1}},
    watch=[['entry_a', 0x18]])

if __name__ == '__main__':
    h.run_cases(ROWS, HERE / 'native-extended.json',
                'Actual 546AF0 element opcodes no campaign mission uses (round 2): selectors, TriggerElement leaves, '
                'and their engine seams.',
                sorted({v[0] for v in h.SEAMS.values()}) + ['virtual ' + v[0] for v in h.VIRTUALS.values()] +
                ['KingdomHeroEntry', 'Store'])
