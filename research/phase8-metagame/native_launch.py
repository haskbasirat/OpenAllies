"""Original metagame battle generation (KKC_MetaGameRTSLauncher::RequestGame
948EF0, MetaGameRTSLauncher.cpp) up to the RTS launch seam, and the RTS
result decision 93A990, on perturbed InitCreate worlds of every shipped
campaign (mg_campaign).

Native code run (everything between the seams below executes from AA.exe):
  948EF0 RequestGame; 947560 participant builder (955E50 attacker iterator,
  93EB50 previous path territory, 957910 link lookup, 4A0A90 / 4A0900 /
  4A0960 RMC kingdom slot lookup, 938B70, 4BEDF0, 76B920 player by kingdom
  name, 955EB0 defending kingdom, 955E80 defender iterator, 946010 / 945200
  / 945BD0 / 945EF0 participant records); 506010 / 506370 RMC settings
  (slider midpoints); 5A44B0 / 511DF0 / 41D130 RMC game-settings copy;
  order constructors 75E700, 687510 / 686AA0, 4157A0 / 40F7D0 / 784130 /
  40FB70, 946400, 9464A0, 946580, 946620, 6990C0, 6971B0, 697110; 946810
  (SetPlayerKingdom(player, '') per session player); 9459F0 side stockpile
  sums (4CD440 / 4CD250 / 45F180 ResourceList, 938BC0, 954BE0 / 954B70,
  4CCD80 binary32 element adds); 9466C0 start properties (945C90 owned
  technologies, 945D10 division technologies of every army of the side);
  948BD0 / 9488B0 / 948770 additional actors; 946810; the local-player
  faction branch (5AA290); and 93A990 (93A320, 955420, 938B70, 933170,
  76B960, 765E00, 95F220 / 95F480 / 95F240 solve).

Seams (engine, not metagame) - each is recorded:
  * order senders (network order stream 766CC0..766D00): 75DD90
    SetSessionRMC, 683470 SetRMCSettings, 6837B0 SetMapSize, 40E790
    SetNewState, 40E6E0 SetMapType, 6961B0 SetPlayerKingdom, 6960F0
    CreateExplicitAI, 945600 SetKingdomMergeIntoIDS, 945540 SetKingdomColor,
    696270 SetKingdomNation, 696330 SetKingdomFaction, 9456C0
    AddKingdomStartProperty, 945900 SetKingdomBonusStockpile, 945780
    ClearKingdomAdditionalActors, 945840 AddKingdomAdditionalActor, 40DF00
    Launch (the RTS launch seam). The order object fields are decoded at the
    seam (ecx) and the sender returns (ret 4); its target argument must be 0.
  * 954130 ResolveBattle in 93A990 (recorded; proven separately by
    native_resolve.py), CRT heap, mg_campaign's InitCreate seams, army
    vtable+108 owner, kingdom vtable+140/+138 relations (team based or random).

Synthetic inputs laid out at native offsets (from the code):
  RMC object (map def +74): +08 IDS, +44 MapSize list (entry +0 width,
  +4 height), +50/+54 list of slider entries (+04 min, +08 max, +14 -> +08
  name), +68/+6C and +74/+78 slider entries (+08 name, +18 min, +1C max),
  +80/+84 int range, +90 game settings (marker words; +14 = 0x5000 + RMC
  index, +04 -> marker map-settings object), +F8/+FC major and +104/+108
  independent kingdom slots (slot +0C -> IDS string, +1C type land 0 /
  water 1, split by the authored `independent` flag as the RMC loader
  4A4EB0 does). Map def +78: rmc_defender_ids. Campaign kingdom data (95EB00):
  +08 name (synthetic marker '#name:<ids>'; localized names are not in
  metagame-data.json), +0C nation object, +1C color object, +34 / +44
  ResourceList (vtable A26F44, data, count, capacity). Army MetaGameComponent
  data: +24 / +28 rts land / water actor, +2C / +3C ResourceLists. Division
  data: +20/+24 technology list, +2C / +30 rts land / water actor. Resource
  registry B36530+34/+38 and ResourceList size B3A660+28 (same registry, a
  random permutation of the data resource names per world). Session B36550
  (+BC player list {player, next}, +E4 app state), local player B36558+8,
  player +0C AI flag, +24 kingdom name, +28 kingdom actor. MGState+0C faction
  object, +45 play requested, +48 RTS winner name. B43B40 (empty string) is
  built by its original static initializer 9E73C0.
  * RMC game-settings value type 5A2210 / 5A44B0 / 5A46D0 / 5A1600 and the
    map-settings default object 511DF0 (engine terrain registries): shallow
    copies (see Harness); SetSessionRMC records the settings marker (+14) and
    whether +04's map-settings fields were reset to the 511DF0 object.

Some worlds perturb the data (army / division rts actors, technologies,
stockpile bonus bits, kingdom colors / nations) and the kingdom actor names
(4BEDF0 reads actor+18); every override is recorded for the Rust replay.
Page 0 stays mapped for the fs:[0] SEH chain; any other access below 0x1000
(a native NULL dereference) stops the run and is recorded as a fault.
"""
import json, hashlib, random, struct, sys
from pathlib import Path
from mg_machine import P, V, VISITED, HERE
from mg_campaign import DATA, run
from mg_world import VT_ARMY, native_wstring, read_wstring
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_EDX

PCS = {'single24': 0x007f, 'double53': 0x027f, 'extended64': 0x037f}
KIND = {'land': 0, 'water': 1}
SENDS = {
    0x75dd90: 'set_session_rmc', 0x683470: 'set_rmc_settings', 0x6837b0: 'set_map_size',
    0x40e790: 'set_new_state', 0x40e6e0: 'set_map_type', 0x6961b0: 'set_player_kingdom',
    0x6960f0: 'create_explicit_ai', 0x945600: 'set_merge_into', 0x945540: 'set_color',
    0x696270: 'set_nation', 0x696330: 'set_faction', 0x9456c0: 'add_start_property',
    0x945900: 'set_bonus_stockpile', 0x945780: 'clear_additional_actors',
    0x945840: 'add_additional_actor', 0x40df00: 'launch',
}
SYNTH_ACTORS = ['synthetic_rts_actor_a', 'synthetic_rts_actor_b', 'Synthetic_RTS_Actor_C', 'usa_corps_dynamic']
SYNTH_TECHS = ['synthetic_tech_x', 'Synthetic_Tech_Y', 'infhqproduction']
FACTIONS = ['faction_allies', 'faction_axis']
SLIDER_NAMES = ['money', 'ammo', 'oil', 'game_speed', 'reinforcements', 'difficulty']


def f32(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def rand_f32(rng):
    r = rng.random()
    if r < 0.15:
        return 0
    if r < 0.5:
        return f32(rng.choice([5, 10, 20, 25, 50, 75, 100, 150, 200, 400, 1000, 0.5, 2.5]))
    if r < 0.8:
        return f32(rng.uniform(-3000, 3000))
    # raw bit patterns: wide exponents (finite, normal), both signs
    e = rng.randint(90, 160)
    return (rng.getrandbits(1) << 31) | (e << 23) | rng.getrandbits(23)


class Harness:
    def __init__(self, ci, cw, rng, relation_mode, perturb_data):
        self.ci = ci
        self.rng = rng
        self.data = json.loads(json.dumps(DATA))       # deep copy, overrides applied below
        self.overrides = {'armies': {}, 'divisions': {}, 'kingdoms': {}}
        if perturb_data:
            self._perturb_data()
        snap, ctx = run(ci, cw=cw, keep=True)
        self.ctx = ctx
        m, w, st = ctx['m'], ctx['w'], ctx['st']
        self.m, self.w, self.st = m, w, st
        self.c = self.data['campaigns'][ci]
        self.mp = self.data['maps'][self.c['map']]
        self.kingdoms = w.read_vector(st + 0x1c)
        self.territories = w.read_vector(st + 0x28)
        self.tindex = {p: i for i, p in enumerate(self.territories)}
        self.defs = ctx['defs']
        self.actors = ctx['actors']
        self.aindex = {p: i for i, p in enumerate(self.actors)}
        self.created = ctx['created']
        self.army_index = {a['actor']: i for i, a in enumerate(self.created)}
        self.comps = [m.r32(a['actor'] + 0xcc) for a in self.created]
        self.ops = []
        n = len(self.actors)
        if relation_mode == 'random':
            enemy = [[a != b and rng.random() < 0.55 for b in range(n)] for a in range(n)]
            r138 = [[(a == b) or rng.random() < 0.45 for b in range(n)] for a in range(n)]
            w.relations['enemy_140'] = enemy
            w.relations['relation_138'] = r138
        self.enemy = w.relations['enemy_140']
        self.r138 = w.relations['relation_138']
        self.null_access = None
        from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
        m.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._null_hook, begin=0, end=0xfff)
        # static initializer 9E73C0: B43B40 = empty string (4013E0); atexit seamed
        m.call(0x9e73c0)
        # RMC game settings (RMC +90, a 0x68-byte engine value type: default
        # constructor 5A2210 / 5A14C0, copy 5A44B0, assignment 5A46D0,
        # destructor 5A1600) and its +04 map-settings object (511DF0, whose
        # 511D90 reads the engine terrain registry B36530+4A0): opaque engine
        # data passed through RequestGame unchanged except for +04's fields.
        # Seams: shallow value copies with a fresh +04 object copied field by
        # field; the RMC's own +90 object is a marker object.
        def map_settings(mm):
            this = mm.u.reg_read(UC_X86_REG_ECX)
            mm.w32(this, 0xa31990)
            for off in range(4, 0x1c, 4):
                mm.w32(this + off, 0)
            mm.ret(0, this)
        def settings_copy(this, src):
            m.u.mem_write(this, bytes(m.u.mem_read(src, 0x68)))
            sub = m.alloc(0x1c)
            if m.r32(src + 4):
                m.u.mem_write(sub, bytes(m.u.mem_read(m.r32(src + 4), 0x1c)))
            m.w32(this + 4, sub)
        def settings_default(mm):
            this = mm.u.reg_read(UC_X86_REG_ECX)
            m.u.mem_write(this, bytes(0x68))
            m.w32(this, 0xa44264)
            sub = m.alloc(0x1c); m.w32(sub, 0xa31990); m.w32(this + 4, sub)
            mm.ret(0, this)
        def settings_copy_ctor(mm):
            this = mm.u.reg_read(UC_X86_REG_ECX)
            settings_copy(this, mm.arg(0))
            mm.ret(4, this)
        m.seams[0x511df0] = map_settings
        m.seams[0x5a2210] = settings_default
        m.seams[0x5a44b0] = settings_copy_ctor
        m.seams[0x5a46d0] = settings_copy_ctor
        m.seams[0x5a1600] = lambda mm: mm.ret(0)
        self.objects = {}                                # pointer -> ('kind', name)
        self._build()
        self._seams()

    def _null_hook(self, u, access, address, size, value, _):
        # page 0 stays mapped for the fs:[0] SEH chain (FS base 0); any other
        # access below 0x1000 is a native NULL dereference: stop and record it.
        pc = u.reg_read(UC_X86_REG_EIP)
        code = bytes(u.mem_read(pc, 4))
        if 0x64 in code[:3]:
            return
        if self.null_access is None:
            self.null_access = (pc, address)
        u.emu_stop()

    # ------------------------------------------------------------ data perturbation
    def _perturb_data(self):
        rng, d = self.rng, self.data
        for a in d['armies']:
            ov = {}
            if rng.random() < 0.5:
                ov['rts_actor_ids'] = rng.choice(SYNTH_ACTORS + [''])
            if rng.random() < 0.5:
                ov['rts_water_actor_ids'] = rng.choice(SYNTH_ACTORS + [''])
            for key in ('stockpile_bonus', 'water_stockpile_bonus'):
                if rng.random() < 0.6:
                    ov[key] = [[name, rand_f32(rng)] for name, _ in a[key]['values']]
            for k, v in ov.items():
                if k.endswith('bonus'):
                    a[k] = {'values': v}
                else:
                    a[k] = v
            if ov:
                self.overrides['armies'][a['ids']] = ov
        for dv in d['divisions']:
            ov = {}
            if rng.random() < 0.4:
                ov['rts_actor_ids'] = rng.choice(SYNTH_ACTORS + [''])
            if rng.random() < 0.4:
                ov['rts_water_actor_ids'] = rng.choice(SYNTH_ACTORS + [''])
            if rng.random() < 0.4:
                techs = list(dv['technologies'])
                for _ in range(rng.randint(0, 2)):
                    techs.insert(rng.randint(0, len(techs)), rng.choice(SYNTH_TECHS + [None]))
                ov['technologies'] = techs
            dv.update(ov)
            if ov:
                self.overrides['divisions'][dv['ids']] = ov
        c = d['campaigns'][self.ci]
        for ki, k in enumerate(c['kingdoms']):
            ov = {}
            for key in ('stockpile_bonus', 'water_stockpile_bonus'):
                if rng.random() < 0.7:
                    ov[key] = [[name, rand_f32(rng)] for name, _ in k[key]['values']]
            if rng.random() < 0.2:
                ov['color_ids'] = rng.choice(['', 'green', 'blue'])
            if rng.random() < 0.2:
                ov['nation_ids'] = rng.choice(['', 'nation_usa', 'nation_ger'])
            for kk, v in ov.items():
                k[kk] = {'values': v} if kk.endswith('bonus') else v
            if ov:
                self.overrides['kingdoms'][str(ki)] = ov

    # ------------------------------------------------------------ native objects
    def obj(self, kind, name):
        key = (kind, name)
        if key not in self.named:
            o = self.m.alloc(0x40)
            self.m.w32(o + 8, native_wstring(self.m, name))
            self.named[key] = o
            self.objects[o] = (kind, name)
        return self.named[key]

    def ref(self, kind, name):
        """NULL for an empty IDS (loaders store NULL), else the registry object."""
        return 0 if not name else self.obj(kind, name)

    def resource_list(self, at, values):
        m = self.m
        by_name = dict(values)
        m.w32(at, 0xa26f44)
        data = m.alloc(4 * max(1, len(self.registry)))
        for i, r in enumerate(self.registry):
            m.w32(data + 4 * i, by_name[r])
        m.w32(at + 4, data)
        m.w32(at + 8, len(self.registry))
        m.w32(at + 0xc, len(self.registry))

    def _build(self):
        m, w, rng, d = self.m, self.w, self.rng, self.data
        self.named = {}
        # resource registry (B36530+34/+38) and ResourceList size (B3A660+28)
        names = [n for n, _ in d['armies'][0]['stockpile_bonus']['values']]
        self.registry = list(names)
        if rng.random() < 0.5:
            rng.shuffle(self.registry)
        root = m.alloc(0x900)
        m.w32(0xb36530, root)
        w.vector(root + 0x34, [self.obj('resource', r) for r in self.registry])
        sizes = m.alloc(0x40)
        m.w32(sizes + 0x28, len(self.registry))
        m.w32(0xb3a660, sizes)
        # techs: owned technology objects of mg_campaign + synthetic ones
        for ids, o in self.ctx['techs'].items():
            self.named[('tech', ids)] = o
            self.objects[o] = ('tech', ids)
        # RMC objects
        self.rmc_obj = {}
        self.sliders = {}
        for ri, r in enumerate(d['rmcs']):
            o = m.alloc(0x200)
            m.w32(o + 8, native_wstring(m, r['ids']))
            entries = []
            for (wb, hb) in r['map_sizes']:
                e = m.alloc(0x10)
                m.w32(e, wb); m.w32(e + 4, hb)
                entries.append(e)
            w.vector(o + 0x44, entries)
            sl = self._sliders(o)
            self.sliders[r['ids']] = sl
            gs = o + 0x90
            m.w32(gs, 0xa44264)                           # marker settings object
            sub = m.alloc(0x1c); m.w32(sub, 0xa31990)
            for off in range(4, 0x1c, 4):
                m.w32(sub + off, 0x6000 + ri * 0x10 + off)
            m.w32(gs + 4, sub)
            for off in range(8, 0x68, 4):
                m.w32(gs + off, 0x7000 + off)
            m.w32(gs + 0x14, 0x5000 + ri)
            major, indep = [], []
            for k in r['kingdoms']:
                slot = m.alloc(0x40)
                cell = m.alloc(0x10)
                m.w32(cell, native_wstring(m, k['ids']))
                m.w32(slot + 0xc, cell)
                m.w32(slot + 0x1c, KIND[k['kind']])
                (indep if k['independent'] else major).append(slot)
            w.vector(o + 0xf8, major)
            w.vector(o + 0x104, indep)
            self.rmc_obj[r['ids'].lower()] = o
            self.objects[o] = ('rmc', r['ids'])
        for t, dp in zip(self.mp['territories'], self.defs):
            o = self.rmc_obj.get(t['rmc_ids'].lower(), 0) if t['rmc_ids'] else 0
            m.w32(dp + 0x74, o)
            m.w32(dp + 0x78, native_wstring(m, t['rmc_defender_ids']))
        # campaign kingdom data (95EB00 layout)
        for k, kd in zip(self.c['kingdoms'], self.ctx['kdatas']):
            m.w32(kd + 8, native_wstring(m, k['name']))
            m.w32(kd + 0xc, self.ref('nation', k['nation_ids']))
            m.w32(kd + 0x1c, self.ref('color', k['color_ids']))
            self.resource_list(kd + 0x34, k['stockpile_bonus']['values'])
            self.resource_list(kd + 0x44, k['water_stockpile_bonus']['values'])
        # army MetaGameComponent data (thing +5B8)
        for ids, th in self.ctx['things'].items():
            a = next(x for x in d['armies'] if x['ids'] == ids)
            g = m.r32(th + 0x5b8)
            m.w32(g + 0x24, self.ref('actor', a['rts_actor_ids']))
            m.w32(g + 0x28, self.ref('actor', a['rts_water_actor_ids']))
            self.resource_list(g + 0x2c, a['stockpile_bonus']['values'])
            self.resource_list(g + 0x3c, a['water_stockpile_bonus']['values'])
            self.objects[th] = ('army', ids)
        # divisions: technologies and rts actors
        for dv, dd in zip(self.ctx['divisions'], d['divisions']):
            w.vector(dv + 0x20, [self.ref('tech', t) for t in dd['technologies']])
            m.w32(dv + 0x2c, self.ref('actor', dd['rts_actor_ids']))
            m.w32(dv + 0x30, self.ref('actor', dd['rts_water_actor_ids']))
        # session objects
        self.session = m.alloc(0x200)
        m.w32(0xb36550, self.session)
        self.local_holder = m.alloc(0x20)
        m.w32(0xb36558, self.local_holder)
        # quick battle settings for the 93A990 solve (95F270/95F370 via B39AB0)
        settings = m.alloc(0x300)
        bonus = m.alloc(4); m.w32(bonus, DATA['settings']['technology_bonus_bits'])
        prob = m.alloc(4); m.w32(prob, DATA['settings']['attack_win_probability_bits'])
        m.w32(settings + 0x208, bonus); m.w32(settings + 0x20c, prob)
        m.w32(0xb39ab0, settings)
        self.actor_names = [read_wstring(m, m.r32(a + 0x18)) for a in self.actors]

    def _sliders(self, o):
        m, w, rng = self.m, self.w, self.rng
        out = {'list1': [], 'list2': [], 'list3': []}
        e1 = []
        for _ in range(rng.randint(0, 4)):
            name = rng.choice(SLIDER_NAMES)
            lo, hi = rand_f32(rng), rand_f32(rng)
            e = m.alloc(0x20)
            m.w32(e + 4, lo); m.w32(e + 8, hi)
            nm = m.alloc(0x10)
            m.w32(nm + 8, native_wstring(m, name))
            m.w32(e + 0x14, nm)
            e1.append(e)
            out['list1'].append([name, lo, hi])
        w.vector(o + 0x50, e1)
        for key, off in (('list2', 0x68), ('list3', 0x74)):
            es = []
            for _ in range(rng.randint(0, 3)):
                name = rng.choice(SLIDER_NAMES)
                lo, hi = rand_f32(rng), rand_f32(rng)
                e = m.alloc(0x20)
                m.w32(e + 8, native_wstring(m, name))
                m.w32(e + 0x18, lo); m.w32(e + 0x1c, hi)
                es.append(e)
                out[key].append([name, lo, hi])
            w.vector(o + off, es)
        lo = rng.choice([0, 1, 2, -5, rng.randint(-2**31, 2**31 - 1)])
        hi = rng.choice([0, 3, 8, 100, rng.randint(-2**31, 2**31 - 1)])
        m.w32(o + 0x80, lo); m.w32(o + 0x84, hi)
        out['int_range'] = [lo, hi]
        return out

    # ------------------------------------------------------------ seams
    def _seams(self):
        m = self.m
        self.orders = []
        for va, kind in SENDS.items():
            m.seams[va] = self._sender(kind)
        self.resolved = []
        def resolve(mm):
            a, d, t, won = mm.arg(0), mm.arg(1), mm.arg(2), mm.arg(3) & 0xff
            self.resolved.append([self.kindex(a), self.kindex(d), self.tindex[t], won])
            mm.ret(0, 1)
        m.seams[0x954130] = resolve

    def kindex(self, p):
        return None if p == 0 else self.kingdoms.index(p)

    def string(self, p):
        return read_wstring(self.m, self.m.r32(p))

    def named_ref(self, p, kind):
        if p == 0:
            return None
        k, name = self.objects[p]
        assert k == kind, (k, kind)
        return name

    def player_ref(self, p):
        return None if p == 0 else self.players.index(p)

    def _sender(self, kind):
        def seam(mm):
            o = mm.u.reg_read(UC_X86_REG_ECX)
            assert mm.arg(0) == 0, (kind, mm.arg(0))
            r32 = mm.r32
            s = lambda off: self.string(o + off)
            if kind == 'set_session_rmc':
                gs = o + 8
                sub = r32(gs + 4)
                rec = [kind, self.named_ref(r32(o + 4), 'rmc'), s(0x70), r32(o + 0x74),
                       r32(gs + 0x14), [r32(sub + off) for off in (4, 8, 0x10, 0x18)]]
            elif kind == 'set_rmc_settings':
                def lst(at):
                    p, n = r32(at), r32(at + 4)
                    return [[self.string(p + 8 * i), r32(p + 8 * i + 4)] for i in range(n)]
                rec = [kind, r32(o + 4), lst(o + 8), lst(o + 0x14), r32(o + 0x20) - (1 << 32) * (r32(o + 0x20) >> 31)]
            elif kind == 'set_map_size':
                rec = [kind, r32(o + 4), r32(o + 8)]
            elif kind == 'set_new_state':
                b = lambda off: mm.u.mem_read(o + 8 + off, 1)[0]
                st = [r32(o + 8), b(4), self.string(o + 8 + 8), r32(o + 8 + 0xc), r32(o + 8 + 0x10),
                      b(0x14), b(0x15), b(0x16), r32(o + 8 + 0x18), r32(o + 8 + 0x1c), b(0x20), r32(o + 8 + 0x24)]
                rec = [kind, r32(o + 4), st]
            elif kind == 'set_map_type':
                rec = [kind, r32(o + 4)]
            elif kind == 'set_player_kingdom':
                rec = [kind, self.player_ref(r32(o + 4)), s(8)]
            elif kind == 'create_explicit_ai':
                rec = [kind, s(4), s(8), s(0xc)]
            elif kind == 'set_merge_into':
                rec = [kind, s(4), s(8)]
            elif kind == 'set_color':
                rec = [kind, s(4), self.named_ref(r32(o + 8), 'color')]
            elif kind == 'set_nation':
                rec = [kind, s(4), self.named_ref(r32(o + 8), 'nation')]
            elif kind == 'set_faction':
                rec = [kind, s(4), self.named_ref(r32(o + 8), 'faction')]
            elif kind == 'add_start_property':
                rec = [kind, s(4), self.named_ref(r32(o + 8), 'tech')]
            elif kind == 'set_bonus_stockpile':
                rec = [kind, s(4), self.named_ref(r32(o + 8), 'resource'), r32(o + 0xc)]
            elif kind == 'clear_additional_actors':
                rec = [kind, s(4)]
            elif kind == 'add_additional_actor':
                rec = [kind, s(4), self.named_ref(r32(o + 8), 'actor')]
            else:
                rec = [kind]
            self.orders.append(rec)
            mm.ret(4)
        return seam

    # ------------------------------------------------------------ perturbation ops
    def move(self, a, t):
        self.m.call(0x940100, args=(self.territories[t],), ecx=self.comps[a])
        self.ops.append(['move', a, t])

    def owner(self, t, k):
        tp = self.territories[t]
        if k is None:
            # 955620(NULL) dereferences NULL in 95A850 (native crash): synthetic
            # unowned state as in native_resolve.set_territory_owner.
            old = self.m.r32(tp + 0x24)
            if old:
                self.m.call(0x959f20, args=(tp,), ecx=old)
            self.m.w32(tp + 0x24, 0)
            self.m.call(0x954cf0, ecx=tp)
        else:
            self.m.call(0x955620, args=(self.kingdoms[k],), ecx=tp)
        self.ops.append(['owner', t, k])

    def tech(self, k, ids):
        kp = self.kingdoms[k]
        cur = self.w.read_vector(kp + 0x20)
        o = self.obj('tech', ids)
        if o not in cur:
            cur.append(o)
            self.w.vector(kp + 0x20, cur)
        self.ops.append(['tech', k, ids])

    def def_territory(self, dp):
        return next(i for i, tp in enumerate(self.territories) if self.m.r32(tp + 0xc) == dp)

    def army_territory(self, a):
        p = self.m.r32(self.comps[a] + 0x20)
        return None if p == 0 else self.tindex[p]

    def linked(self, t):
        dp = self.m.r32(self.territories[t] + 0xc)
        return [self.def_territory(self.m.r32(l + 8)) for l in self.w.read_vector(dp + 0x68)]

    # ------------------------------------------------------------ session
    def make_session(self):
        m, rng = self.m, self.rng
        n = len(self.actors)
        names = []
        for k in rng.sample(range(n), n):
            r = rng.random()
            if r < 0.95:
                names.append(self.c['kingdoms'][k]['ids'] if rng.random() < 0.3 else self.actor_names[k])
            elif r < 0.975:
                names.append(self.c['kingdoms'][k]['ids'].upper())     # case-sensitive lookups miss
        for _ in range(rng.choice([0, 0, 1, 2])):
            names.append(rng.choice(['', 'spectator', self.c['kingdoms'][rng.randrange(n)]['ids']]))
        players = []
        nodes = []
        desc = []
        for name in names:
            p = m.alloc(0x40)
            ai = 1 if rng.random() < 0.55 else 0
            if rng.random() < 0.05:
                ai = rng.choice([2, 0x80, 0xff])
            m.u.mem_write(p + 0xc, bytes([ai]))
            m.w32(p + 0x24, native_wstring(m, name))
            actor = rng.randrange(n) if rng.random() < 0.9 else None
            m.w32(p + 0x28, 0 if actor is None else self.actors[actor])
            players.append(p)
            desc.append({'kingdom_name': name, 'ai': ai, 'kingdom_actor': actor})
        head = 0
        for p in reversed(players):
            node = m.alloc(8)
            m.w32(node, p)
            m.w32(node + 4, head)
            head = node
        m.w32(self.session + 0xbc, head)
        self.players = players
        humans = [i for i, d in enumerate(desc) if d['ai'] == 0]
        r = rng.random()
        if r < 0.05 or not players:
            local = None
        elif humans and r < 0.85:
            local = rng.choice(humans)
        else:
            local = rng.randrange(len(players))
        m.w32(self.local_holder + 8, 0 if local is None else players[local])
        fac = rng.choice(FACTIONS + [None]) if rng.random() < 0.9 else None
        m.w32(self.st + 0xc, 0 if fac is None else self.obj('faction', fac))
        return {'players': desc, 'local': local, 'faction': fac}

    def rename_actors(self):
        """Kingdom actor names (actor+18, read by 4BEDF0) differ from the IDS
        in some worlds (attackers look players up by actor name)."""
        for i, a in enumerate(self.actors):
            if self.rng.random() < 0.3:
                name = self.rng.choice([self.c['kingdoms'][i]['ids'].upper(), 'renamed_' + str(i),
                                        self.c['kingdoms'][(i + 1) % len(self.actors)]['ids']])
                self.m.w32(a + 0x18, native_wstring(self.m, name))
                self.actor_names[i] = name

    # ------------------------------------------------------------ native runs
    def participants(self, t):
        m, w = self.m, self.w
        launcher = m.alloc(0x10)
        m.w32(launcher, 0xac67bc)
        m.w32(launcher + 4, self.territories[t])
        lst = m.alloc(0x10)
        w.vector(lst, [], 4)
        try:
            guarded(self, 0x947560, args=(lst,), ecx=launcher, count=20_000_000)
        except RuntimeError as e:
            return {'fault': fault_text(e)}
        out = []
        thing_ids = {v: k for k, v in self.ctx['things'].items()}
        for rec in w.read_vector(lst):
            kd = m.r32(rec + 0xc)
            out.append({
                'rmc_id': self.string(rec),
                'kind': m.r32(rec + 4),
                'player': self.player_ref(m.r32(rec + 8)),
                'kingdom': None if kd == 0 else self.ctx['kdatas'].index(kd),
                'merge_into': self.string(rec + 0x10),
                'armies': [thing_ids[x] for x in w.read_vector(rec + 0x14)],
                'divisions': [self.ctx['divisions'].index(x) for x in w.read_vector(rec + 0x20)],
            })
        return {'participants': out}

    def request_game(self, t):
        m = self.m
        launcher = m.alloc(0x10)
        m.w32(launcher, 0xac67bc)
        m.w32(launcher + 4, self.territories[t])
        self.orders = []
        try:
            guarded(self, 0x948ef0, ecx=launcher, count=50_000_000)
        except RuntimeError as e:
            return {'fault': fault_text(e), 'orders': self.orders}
        return {'orders': self.orders}

    def rts_outcome(self, winner, app_state, play_requested):
        m = self.m
        m.w32(self.session + 0xe4, app_state)
        m.u.mem_write(self.st + 0x45, bytes([play_requested]))
        m.w32(self.st + 0x48, native_wstring(m, winner))
        self.resolved = []
        rng_before = m.r32(self.st + 0x18)
        try:
            guarded(self, 0x93a990, args=(0,), ecx=self.st, count=20_000_000)
        except RuntimeError as e:
            return {'fault': fault_text(e)}
        return {'resolved': self.resolved, 'play_requested_after': m.u.mem_read(self.st + 0x45, 1)[0],
                'rng_before': rng_before, 'rng_after': m.r32(self.st + 0x18)}


def fault_text(e):
    s = str(e)
    return s[:300]


def guarded(h, target, **kw):
    """m.call that reports a NULL access (stopped run) as a RuntimeError."""
    h.null_access = None
    try:
        return h.m.call(target, **kw)
    except AssertionError:
        if h.null_access is None:
            raise
    except RuntimeError:
        if h.null_access is None:
            raise
    pc, address = h.null_access
    raise RuntimeError(f'null access {address:#x} at {pc:#x}')


def make_battle(h, rng):
    """Moves armies so that a territory holds defenders and attackers that
    entered from linked territories (sometimes from elsewhere)."""
    m = h.m
    n_terr = len(h.territories)
    candidates = [i for i, tp in enumerate(h.territories)
                  if h.mp['territories'][h.ctx['defs'].index(m.r32(tp + 0xc))]['kind'] == 'Normal']
    t = rng.choice(candidates) if rng.random() < 0.93 else rng.randrange(n_terr)
    links = h.linked(t)
    # evacuate most armies already standing there (they may have entered from
    # anywhere, e.g. InitCreate armies with a one-entry path)
    others = [i for i in range(n_terr) if i != t]
    here = [a for a in range(len(h.created)) if h.army_territory(a) == t]
    keep = 0.0 if rng.random() < 0.85 else 0.3
    for a in here:
        if rng.random() >= keep:
            h.move(a, rng.choice(others))
    # owner
    if rng.random() < 0.15:
        h.owner(t, rng.randrange(len(h.kingdoms)))
    owner_p = m.r32(h.territories[t] + 0x24)
    owner = None if owner_p == 0 else h.kingdoms.index(owner_p)
    armies = [i for i in range(len(h.created))]
    def kingdom_of(a):
        return h.aindex[h.w.army_owner[h.created[a]['actor']]]
    # defenders: owner's and allied armies moved in directly
    if owner is not None:
        friends = [a for a in armies if not h.enemy[owner][kingdom_of(a)]]
        for a in rng.sample(friends, min(len(friends), rng.randint(0, 3))):
            if h.army_territory(a) != t:
                h.move(a, t)
    # attackers: enemies of the owner (or anyone when unowned)
    foes = [a for a in armies if owner is None or h.enemy[owner][kingdom_of(a)]]
    if rng.random() < 0.1:
        foes = armies
    for a in rng.sample(foes, min(len(foes), rng.randint(1, 5))):
        r = rng.random()
        if links and r < 0.95:
            o = rng.choice(links)
            if h.army_territory(a) != o:
                h.move(a, o)
            h.move(a, t)
        elif r < 0.975:
            o = rng.randrange(n_terr)
            if h.army_territory(a) != o:
                h.move(a, o)
            if o != t:
                h.move(a, t)
        else:
            if h.army_territory(a) != t:
                h.move(a, t)
    for _ in range(rng.randint(0, 3)):
        k = rng.randrange(len(h.kingdoms))
        pool = list(h.ctx['techs'].keys()) + SYNTH_TECHS
        h.tech(k, rng.choice(pool))
    return t


def main():
    rng = random.Random(0x948ef0)
    worlds = []
    counts = {}
    plan = []
    for ci in range(len(DATA['campaigns'])):
        plan.append((ci, 'double53', 'team', False))
        plan.append((ci, 'double53', 'random', True))
        plan.append((ci, 'double53', 'team', True))
        plan.append((ci, 'single24', 'random', True))
        plan.append((ci, 'extended64', 'random', True))
        plan.append((ci, 'single24', 'team', False))
    for ci, pc, relation_mode, perturb in plan:
        h = Harness(ci, PCS[pc], rng, relation_mode, perturb)
        if perturb:
            h.rename_actors()
        battles = []
        for b in range(30):
            ops_before = len(h.ops)
            t = make_battle(h, rng)
            session = h.make_session()
            case = {'ops': h.ops[ops_before:], 'territory': t, 'session': session,
                    'actor_names': list(h.actor_names)}
            case['participants'] = h.participants(t)
            case['request'] = h.request_game(t)
            outcomes = []
            for _ in range(3):
                pend_names = ['', h.mp['territories'][h.ctx['defs'].index(m_def(h, t))]['rmc_defender_ids'],
                              'kingdom_attacker_1', 'Kingdom_Defender', 'someone']
                winner = rng.choice(pend_names)
                app = 5 if rng.random() < 0.9 else rng.choice([0, 4, 6])
                play = 1 if rng.random() < 0.9 else 0
                res = h.rts_outcome(winner, app, play)
                outcomes.append({'winner': winner, 'app_state': app, 'play_requested': play, 'result': res})
            case['rts'] = outcomes
            battles.append(case)
            req = case['request']
            if 'fault' in req:
                counts['fault ' + req['fault'].split(' at ')[-1]] = counts.get('fault ' + req['fault'].split(' at ')[-1], 0) + 1
            else:
                counts['launched'] = counts.get('launched', 0) + 1
                for o in req['orders']:
                    counts['order ' + o[0]] = counts.get('order ' + o[0], 0) + 1
            for o in outcomes:
                r = o['result']
                key = 'rts fault' if 'fault' in r else f"rts resolved={len(r['resolved'])}"
                counts[key] = counts.get(key, 0) + 1
        worlds.append({
            'campaign': ci, 'precision': pc, 'relation_mode': relation_mode,
            'enemy_140': h.enemy, 'relation_138': h.r138,
            'resource_registry': h.registry, 'data_overrides': h.overrides,
            'rmc_sliders': h.sliders, 'battles': battles,
        })
        print(ci, pc, relation_mode, perturb, counts, file=sys.stderr)
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'counts': dict(sorted(counts.items())),
        'worlds': worlds,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-launch.json'
    out.write_text(json.dumps(report) + '\n')
    print(json.dumps({'counts': counts, 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


def m_def(h, t):
    return h.m.r32(h.territories[t] + 0xc)


if __name__ == '__main__':
    main()
