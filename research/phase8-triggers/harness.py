"""Generic Unicorn harness for actual 546AF0 trigger elements (phase8).

The ORIGINAL AA.exe image executes 546AF0, its validation (541900), register
conversion (5422F0/54A5A0), every selector (54D9E0..54E9F0), every TriggerElement
leaf and the trigger-owned Kingdom objective/mission-result methods.  Only the
functions listed in SEAMS and the VIRTUALS table are replaced; each replaced call
is recorded with its receiver and arguments so the Rust port must reproduce the
same call sequence.  Everything else that runs is pinned in visited_windows.

World objects are fixture memory blocks: a registry (0xB364C0) maps GTIDs to
blocks, every block has a per-kind vtable whose slots are recording stubs.
"""
from pathlib import Path
import copy, hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase7-triggers'))
import native_core_program as core  # noqa: E402  (phase7 base machine)
from unicorn.x86_const import *  # noqa: E402,F403

BASE = core.BASE
VSTUB = 0x6000000                # virtual-call stubs (own mapping)
SSTUB = 0x6100000                # string manager / float trampolines / format return
FMT_RET = SSTUB + 0x300
SCRATCH = SSTUB + 0x400

KINDS = ['actor', 'kingdom', 'area', 'camera', 'data', 'objective', 'team', 'audio', 'misc']
MANAGER, WORLD, CONTEXT = core.MANAGER, core.WORLD, core.CONTEXT
GAMEDATA = BASE + 0x50000        # [0xB36530]
GAMEVARS = BASE + 0x52000        # [0xB39AB0]
SETTINGS = BASE + 0x54000        # [0xB39940]
AUDIO = BASE + 0x56000           # [0xB36548]
UI = BASE + 0x58000              # World+0x130
BUS = BASE + 0x5A000             # [0xB39D8C]

# Direct seams: address -> (name, stack bytes popped by callee, argument kinds).
# Kinds: o=object/pointer, w=word, f=float bits, s=pointer-to-CString, t=wchar*,
# v=pointer-to-vec3, b=byte.  'this' is recorded when the seam is a method.
SEAMS = {
    0x4f8e90: ('NamedActor', 4, 's', False),          # world name table
    0x4f8f00: ('NamedKingdom', 4, 's', False),
    0x4e64c0: ('KingdomDefaultLose', 0, '', True),
    0x4f5490: ('WorldRemoveObject', 4, 'o', False),
    0x4d7960: ('ActorDestroy', 0, '', True),
    0x7476e0: ('PlaySpeech', 0x14, 'owwww', False),     # audio system [B36548]
    0x7474f0: ('PlaySound', 0x20, 'owwwwVww', False),
    0x42d4b0: ('PostMessage', 0, 'xwot', False),        # cdecl (bus, code, kingdom, text)
    0x50c880: ('FlashUiButton', 4, 's', False),         # World+130 UI
    0x4be150: ('SortObjectivesVar', 0, '', False),
    0x405370: ('GameVar', 4, 's', False),
    0x4e9560: ('KingdomResultText', 8, 'xw', True),
    0x4f5290: ('LocalKingdom', 0, '', False),
    0x50d370: ('UiMissionEnded', 0, '', False),
    0x4f4da0: ('WorldMissionResult', 4, 'o', False),
    0x48d500: ('DifficultyByName', 4, 't', False),
    0x578d10: ('OrganizationHasMember', 4, 'w', True),
    0x59aba0: ('RealTimeMs', 0, '', False),             # [B396E0] performance clock
    0x50d120: ('UiScriptCommand', 4, 's', False),       # World+130 script command
    0x4f1080: ('WorldSetCutsceneMode', 4, 'b', False),
    0x6fe710: ('Camera6FE710', 0, '', False),           # *[B40048]
    0x700a90: ('Camera700A90', 4, 'w', False),
    0x4e3ad0: ('KingdomAddProperty', 0x10, 'owww', True),
    0x4e23d0: ('KingdomRemoveProperty', 8, 'ow', True),
    0x4cef70: ('ActorRemoveProperty', 8, 'ow', True),
    0x50cd00: ('UiDisplayBanner', 8, 'sw', False),
    0x4edb10: ('ViewSetSunColors', 0x10, 'vvvw', False),   # World+4C view
    0x4e2080: ('KingdomAddResource', 8, 'ww', True),
    0x4f1340: ('WorldShowCounter', 4, 'P', False),
    0x4f1390: ('WorldHideCounter', 0, '', False),
    0x50c3e0: ('UiFlashActor', 4, 'o', False),
    0x4f8c30: ('WorldScriptListen', 4, 's', False),
    0x4f5e70: ('WorldScriptUnlisten', 4, 's', False),
    0x4ce770: ('ActorGainExperience', 0xc, 'www', True),
    0x50d280: ('SpawnVfx', 0xc, 'ovv', False),
    0x40bc50: ('ChatMessageAll', 0, 'xwt', False),       # cdecl (B399B0, 0xF, text)
    0x40bf70: ('ChatMessageKingdom', 0, 'xwowt', False),  # cdecl (B399B0, 0xF, kingdom, 0, text)
    0x45ebb0: ('AnimationByName', 4, 't', True),
    0x4ce730: ('ActorPlayAnimation', 4, 'o', True),
    0x4d4c60: ('Actor4D4C60', 4, 'b', True),
    0x4d9b60: ('ProcessCommand', 4, 'o', True),
    0x554b30: ('FindPlacement', 0x18, '22owwb', False),   # World+30
    0x4fa030: ('WorldCreateActor', 4, 'S', False),
    0x4fa150: ('WorldCreateActor150', 4, 'S', False),
    0x5adc90: ('VideoPath', 8, 'xs', False),
    0x410cb0: ('GameVarString', 8, 'xt', False),        # 0xB39988 string variables
    0x4f0360: ('AreaFirstActor', 4, 'o', False),
    0x4454a0: ('Rotate2D', 0, '2w', False),             # cdecl (vec2*, angle)
    0x58b5e0: ('TransportPath', 0, 'xwwww', False),     # cdecl (out*, a.x, a.y, b.x, b.y)
    0x58b760: ('TransportPathValid', 0, 'wwww', False),  # cdecl (a.x, a.y, b.x, b.y)
    0x4f9c10: ('WorldCreateTransport', 4, 'S', False),
    0x4c2290: ('CreateGroup', 4, 'w', False),            # this = new 0x18 block
    0x58ba90: ('TransportLoad', 0x1c, 'osowwww', True),   # this = transport+C0
    0x496ef0: ('FlareReadyPosition', 4, 'w', False),    # this = flare+57C -> +2C
    0x496f10: ('FlareReadyActor', 4, 'w', False),
    0x572e50: ('CreateFlare', 0, 'Sooww', False),        # cdecl
    0x4c86d0: ('AbilityUserInit', 4, 'o', False),        # this = stack ability user
    0x4c9f10: ('AbilityUse', 4, 'R', False),
    0x4ce080: ('AbilityUserRelease', 0, '', False),
    0x4e0fb0: ('KingdomHasProperty', 8, 'ow', True),
    0x4f0950: ('ActorInsideArea', 8, 'oo', False),
    0x76b960: ('KingdomPlayer', 4, 'o', False),
}
# Seams whose native result is an x87 ST0 value: (name, pop, kinds, record_this).
FLOAT_SEAMS = {
    0x4dbd80: ('CameraPitch', 0, '', True),
    0x4dbd40: ('CameraYaw', 0, '', True),
    0x700b10: ('Camera700B10', 0, '', False),
    0x453ed0: ('FlareVar', 4, 't', False),               # 0xB39AA8 float variable
}
# KKC_DataManager<T>::GetDataByIDSWarn instances (identical template bodies).
BANKS = {0x54e680: 'audio', 0x54df80: 'actor_data', 0x54e080: 'property', 0x54e480: 'command',
         0x54e280: 'weather', 0x54e380: 'sun', 0x54e580: 'formation', 0x54e780: 'sai',
         0x54e180: 'ability'}
for _a, _bank in BANKS.items():
    SEAMS[_a] = ('DataByIds:' + _bank, 8, 'tx', False)
# Virtual methods by (kind, vtable offset): (name, stack bytes popped, arg kinds)
VIRTUALS = {
    ('actor', 0x108): ('Owner', 0, ''),
    ('actor', 0x158): ('Vt158', 0, ''),
    ('actor', 0x15c): ('Vt15C', 0, ''),
    ('actor', 0x160): ('TransferControl', 4, 'o'),
    ('actor', 0xa8): ('IsKingdom', 0, ''),
    ('kingdom', 0xa8): ('IsKingdom', 0, ''),
    ('kingdom', 0x108): ('Owner', 0, ''),
    ('area', 0xbc): ('IsArea', 0, ''),
    ('actor', 0xbc): ('IsArea', 0, ''),
    ('camera', 0xc0): ('IsCamera', 0, ''),
    ('actor', 0xc0): ('IsCamera', 0, ''),
    ('data', 0x38): ('IsActorData', 0, ''),
    ('data', 0x48): ('IsVfx', 0, ''),
    ('misc', 0x28): ('UiFindWidget', 4, 't'),
    ('actor', 0x70): ('IsVisibleTo', 8, 'ow'),
    ('actor', 0x74): ('IsExploredBy', 8, 'ow'),
}
for _kind in KINDS:
    VIRTUALS.setdefault((_kind, 0xbc), ('IsArea', 0, ''))
    VIRTUALS.setdefault((_kind, 0xc0), ('IsCamera', 0, ''))


class Machine(core.Machine):
    def __init__(self, c):
        self.calls = []
        self.objects = {}
        self.names_by_ptr = {}
        self.vtables = {}
        self.stub_map = {}
        super().__init__(c)
        u = self.u
        # fs:[0] (SEH chain) lives at address 0..3: only FS-prefixed accesses
        # may touch page 0; any other access is a native null dereference.
        from loader import guard_null_page
        guard_null_page(u)
        u.mem_map(VSTUB, 0x10000)
        u.mem_map(SSTUB, 0x1000)
        u.mem_write(VSTUB, b'\xc3' * 0x10000)
        for ptr, value in [(0xb36530, GAMEDATA), (0xb39ab0, GAMEVARS), (0xb39940, SETTINGS),
                           (0xb36548, AUDIO), (0xb39d8c, BUS)]:
            self.w(ptr, value)
        self.w(WORLD + 0x130, UI)
        self.w(WORLD + 0x48, c.get('world_48', 0))
        # Every settings / game-variable slot points at a zero dword unless a case
        # sets it, so no native read can fall through to the fs:[0] page.
        zero = self.alloc(16)
        for base_ptr in (SETTINGS, GAMEVARS):
            for off in range(0, 0x800, 4):
                self.w(base_ptr + off, zero)
        warn = self.alloc(4)
        self.u.mem_write(warn, bytes([c.get('warn_invalid_ids', 0)]))
        self.w(SETTINGS + 0x104, warn)
        camlog = self.alloc(4)
        self.u.mem_write(camlog, bytes([c.get('camera_log', 0)]))
        self.w(SETTINGS + 0x25c, camlog)
        # Fake ATL IAtlStringMgr returned by 856370: [0]Allocate [1]Free
        # [2]Reallocate [3]GetNilString [4]Clone.  Only GetNilString is reached.
        self.strmgr = self.alloc(16)
        self.strmgr_vt = self.alloc(32)
        self.w(self.strmgr, self.strmgr_vt)
        for i in range(5):
            self.w(self.strmgr_vt + i * 4, SSTUB + i * 4)
        self.u.mem_write(SSTUB, b'\xc3' * 0x100)
        # Float-return trampolines: fld dword [FSCR]; ret n
        self.fscr = SSTUB + 0x100
        self.ftramp = {}
        for i, pop in enumerate([0, 4, 8, 0xc]):
            code = SSTUB + 0x200 + i * 16
            self.u.mem_write(code, b'\xd9\x05' + struct.pack('<I', self.fscr) +
                             (b'\xc3' if pop == 0 else b'\xc2' + struct.pack('<H', pop)))
            self.ftramp[pop] = code
        self.fmt_frames = []
        self.nil_strings = set()
        # Original CRT startup step 96BD7F (part of __fpmath): fills the
        # _cfltcvt_tab pointers used by the wide printf floating conversions.
        self.call(0x96bd7f)
        # CRT per-thread data (_getptd 9738A9): the image's static initial
        # locale info B353F0 (C locale; the game never changes LC_CTYPE or
        # LC_NUMERIC away from "C", see loader.py), so _wcsicmp (96CAD6)
        # executes its native ASCII-only fold and printf uses '.'.
        self.locinfo = self.r(0xb35444)
        assert self.locinfo == 0xb353f0
        self.ptd = self.alloc(0x100)
        self.w(self.ptd + 0x64, self.locinfo)
        # Static SortObjectives binding already performed (one-time var bind).
        self.u.mem_write(0xb3cbe0, b'\x01')
        registry = self.alloc(0x60010)
        self.registry = registry
        self.w(0xb364c0, registry)
        for i, kind in enumerate(KINDS):
            vt = self.alloc(0x400)
            self.vtables[kind] = vt
            for slot in range(0x100):
                stub = VSTUB + i * 0x400 + slot * 4
                self.w(vt + slot * 4, stub)
                self.stub_map[stub] = (kind, slot * 4)
        # First pass: allocate every object so fields may reference others.
        for name, spec in c.get('objects', {}).items():
            p = self.alloc(spec.get('size', 0x400))
            self.objects[name] = p
            self.names_by_ptr[p] = name
        for name, spec in c.get('objects', {}).items():
            p = self.objects[name]
            self.w(p, self.vtables[spec['kind']])
            if 'slot' in spec:
                gtid = (spec['gen'] << 16) | spec['slot']
                self.w(p + 0x14, gtid)
                self.u.mem_write(registry + 4 + spec['slot'] * 2, struct.pack('<H', spec['gen']))
                self.w(registry + 0x20004 + spec['slot'] * 4, p)
            for off, value in spec.get('words', {}).items():
                self.w(p + int(off, 0), self.value(value))
            for off, value in spec.get('bytes', {}).items():
                self.u.mem_write(p + int(off, 0), bytes([value]))
            for off, items in spec.get('lists', {}).items():
                self.w(p + int(off, 0), self.linked([self.value(v) for v in items]) if items else 0)
        # Stale registry slots (generation mismatch) for negative selector cases.
        for slot, gen in c.get('stale_slots', []):
            self.u.mem_write(registry + 4 + slot * 2, struct.pack('<H', gen))
        self.objective_defs = []
        defs = c.get('objective_defs', [])
        table = self.alloc(4 * max(1, len(defs)))
        for i, d in enumerate(defs):
            p = self.alloc(0x20)
            self.w(p + 4, self.string(d['name']))
            self.w(p + 0xc, self.string(d['text']))
            self.u.mem_write(p + 0x10, bytes([d['flag']]))
            self.w(table + i * 4, p)
            self.objective_defs.append(p)
            self.names_by_ptr[p] = 'objective:' + d['name']
        self.w(WORLD + 0x250, table)
        self.w(WORLD + 0x254, len(defs))
        for kname, kspec in c.get('kingdoms', {}).items():
            k = self.objects[kname]
            entries = kspec.get('objectives', [])
            cap = max(4, len(entries))
            arr = self.alloc(4 * cap)
            for i, (dname, state) in enumerate(entries):
                e = self.alloc(8)
                self.w(e, self.objective_defs[[d['name'] for d in defs].index(dname)])
                self.w(e + 4, state)
                self.w(arr + i * 4, e)
            self.w(k + 0x300, arr)
            self.w(k + 0x304, len(entries))
            self.w(k + 0x308, cap)
            self.w(k + 0x19c, kspec.get('result', 0))
            team = kspec.get('team_members')
            if team is None:
                self.w(k + 0x1ec, 0)
            else:
                t = self.alloc(0x50)
                self.w(t + 0x44, self.linked([k] * team) if team else 0)
                self.w(k + 0x1ec, t)
            self.u.mem_write(k + 0x238, bytes([kspec.get('victory_238', 1), kspec.get('flag_239', 0)]))
        m = c.get('manager', {})
        self.w(MANAGER + 0x10, m.get('context_word', 0))
        subjects = m.get('subjects', [])
        if subjects:
            head = self.linked(subjects)
            self.w(MANAGER + 0x14, head)
            node = head
            while self.r(node + 4):
                node = self.r(node + 4)
            self.w(MANAGER + 0x18, node)
            self.w(MANAGER + 0x1c, MANAGER + 0x14)
            self.w(MANAGER + 0x20, head)
        else:
            self.w(MANAGER + 0x20, 0)
        self.w(MANAGER + 0x24, self.value(m.get('condition_kingdom', 0)))
        self.w(MANAGER + 0x28, self.value(m.get('event_kingdom', 0)))
        self.event_vt = {}
        events = m.get('events', [])
        nodes = []
        for ev in events:
            nodes.append(self.event(ev))
        self.w(MANAGER + 0x50, self.linked(nodes) if nodes else 0)
        if c.get('map_difficulty') is not None:
            p = self.alloc(4)
            self.w(p, c['map_difficulty'])
            self.w(GAMEVARS + 0x104, p)
        self.returns = copy.deepcopy(c.get('returns', {}))
        cam = self.alloc(0x400)
        camp = self.alloc(4)
        self.w(camp, cam)
        self.w(0xb40048, camp)
        self.names_by_ptr[cam] = 'camera_controller'
        uiroot = self.alloc(0x100)
        self.w(0xb3653c, uiroot)
        if 'ui_root' in self.objects:
            self.w(uiroot + 0x74, self.objects['ui_root'])
        vars990 = self.alloc(0x200)
        self.w(0xb39990, vars990)
        for off in range(0, 0x200, 4):
            self.w(vars990 + off, zero)
        for off, bits_ in c.get('vars990', {}).items():
            q = self.alloc(16)
            if isinstance(bits_, list):
                for i, w in enumerate(bits_):
                    self.w(q + 4 + i * 4, w)
            else:
                self.w(q, bits_)
            self.w(vars990 + int(off, 0), q)
        self.w(WORLD + 0xd8, c.get('world_d8', 0))
        kingdoms = c.get('world_kingdoms', [])
        arr = self.alloc(4 * max(1, len(kingdoms)))
        for i, k in enumerate(kingdoms):
            self.w(arr + i * 4, self.objects[k] if k else 0)
        self.w(WORLD + 0x154, arr)
        self.w(WORLD + 0x158, len(kingdoms))
        slots = c.get('world_148', [])
        sarr = self.alloc(4 * max(1, len(slots)))
        for i, k in enumerate(slots):
            self.w(sarr + i * 4, self.objects[k] if k else 0)
        self.w(WORLD + 0x148, sarr)
        self.w(WORLD + 0x14c, len(slots))
        players = self.alloc(0x100)
        self.w(0xb36550, players)
        resources = c.get('resources', [])
        rarr = self.alloc(4 * max(1, len(resources)))
        for i, flag18 in enumerate(resources):
            rr = self.alloc(0x20)
            self.w(rr + 0x18, flag18)
            self.w(rarr + i * 4, rr)
        self.w(GAMEDATA + 0x34, rarr)
        # Command definition registry GameData+298 indexed by ordinal (4D9E20).
        cmds = self.alloc(0x100)
        for ordinal, name in c.get('command_defs', {}).items():
            self.w(cmds + int(ordinal) * 4, self.objects[name])
        self.w(GAMEDATA + 0x298, cmds)
        self.w(GAMEDATA + 0x38, len(resources))

    def parameter(self, p, q):
        super().parameter(p, q)
        if p['storage'] == 7 and p['value'].get('inline') is not None:
            self.w(q + 36, self.string(p['value']['inline']))

    def arg(self, i=0):
        return self.r(self.u.reg_read(UC_X86_REG_ESP) + 4 + i * 4)

    # --- value helpers ------------------------------------------------------
    def string(self, s):
        if s in self.strings:
            return self.strings[s]
        raw = s.encode('utf-16le')
        p = self.alloc(18 + len(raw)) + 16
        self.w(p - 12, len(raw) // 2)
        self.w(p - 8, len(raw) // 2)
        self.w(p - 4, 0x100000)       # never released by the fixture
        self.u.mem_write(p, raw + b'\0\0')
        self.strings[s] = p
        self.names_by_ptr.setdefault(p, None)
        return p

    def value(self, v):
        if isinstance(v, str):
            if v.startswith('@'):
                return self.objects[v[1:]]
            if v.startswith('$'):
                return self.string(v[1:])
            if v.startswith('gtid:'):
                return self.gtid(v[5:])
        return v

    def gtid(self, name):
        spec = self.c['objects'][name]
        return (spec['gen'] << 16) | spec['slot']

    def event(self, ev):
        kind = ev['kind']
        vt = self.event_vt.get(kind)
        if vt is None:
            vt = self.alloc(16)
            code = self.alloc(16)
            # mov eax, kind ; ret
            self.u.mem_write(code, b'\xb8' + struct.pack('<I', kind) + b'\xc3')
            self.w(vt, code)
            self.event_vt[kind] = vt
        p = self.alloc(0x20)
        self.w(p, vt)
        for i, word in enumerate(ev.get('words', [])):
            self.w(p + 4 + i * 4, self.value(word))
        if 'text' in ev:
            self.w(p + 8, self.string(ev['text']))
        return p

    def describe(self, v):
        if v in self.names_by_ptr and self.names_by_ptr[v]:
            return '@' + self.names_by_ptr[v]
        return v

    def command(self, p):
        return {'definition': self.describe_word(self.r(p + 8)), 'flags': self.r(p + 0xc),
                'words': [self.describe_word(self.r(p + 0x10 + i * 4)) for i in range(8)],
                'byte30': self.u.mem_read(p + 0x30, 1)[0], 'shared34': self.r(p + 0x34),
                'word38': self.r(p + 0x38), 'byte3c': self.u.mem_read(p + 0x3c, 1)[0]}

    def describe_word(self, v):
        if v in self.names_by_ptr and self.names_by_ptr[v]:
            return '@' + self.names_by_ptr[v]
        if v in self.nil_strings:
            return '$'
        for text, ptr in self.strings.items():
            if ptr == v:
                return '$' + text
        return v

    def describe_arg(self, kind, v):
        if kind == 'o':
            return 0 if v == 0 else self.describe(v)
        if kind == 's':
            return self.text(self.r(v))
        if kind == 't':
            return self.text(v)
        if kind == 'v':
            return [self.r(v + i * 4) for i in range(3)]
        if kind == 'V':
            return 0 if v == 0 else [self.r(v + i * 4) for i in range(3)]
        if kind in ('P', '2'):
            return [self.r(v), self.r(v + 4)]
        if kind == 'R':
            return [self.describe_word(self.r(v + i * 4)) for i in range(12)]
        if kind == 'S':
            return [self.describe_word(self.r(v + i * 4)) for i in range(0x54 // 4)]
        if kind == 'b':
            return v & 0xff
        return v

    def give(self, name, this, args=()):
        if isinstance(this, dict):
            this = 0
        spec = self.returns.get(name, 0)
        if isinstance(spec, list):
            spec = spec.pop(0) if spec else 0
        if isinstance(spec, dict) and 'by_arg' in spec:
            spec = spec['by_arg'].get(str(args[0]), 0)
        elif isinstance(spec, dict):
            key = self.names_by_ptr.get(this) or '*'
            spec = spec.get(key, spec.get('*', 0))
        return self.value(spec)

    # --- hooks --------------------------------------------------------------
    def record(self, name, this, args, has_this):
        entry = {'call': name}
        if has_this:
            entry['this'] = this if isinstance(this, dict) else self.describe(this)
        if args:
            entry['args'] = args
        self.calls.append(entry)

    def hook(self, u, a, n, x):
        if 0x400000 <= a < 0xc00000:
            self.visited[a] = n
        if a in SEAMS:
            name, pop, kinds, has_this = SEAMS[a]
            this = u.reg_read(UC_X86_REG_ECX)
            args = [self.describe_arg(k, self.arg(i)) for i, k in enumerate(kinds) if k != 'x']
            if name == 'ProcessCommand':
                this = self.command(this)
                has_this = True
            self.record(name, this, args, has_this)
            ret = self.special(name, this, args)
            self.ret(pop, ret)
            return
        if VSTUB <= a < VSTUB + 0x10000:
            kind, off = self.stub_map[a]
            spec = VIRTUALS.get((kind, off))
            if spec is None:
                raise RuntimeError(f'unexpected virtual {kind}+{off:#x}')
            name, pop, kinds = spec
            this = u.reg_read(UC_X86_REG_ECX)
            args = [self.describe_arg(k, self.arg(i)) for i, k in enumerate(kinds)]
            self.record(name, this, args, True)
            self.ret(pop, self.give(name, this, args))
            return
        if a in FLOAT_SEAMS:
            name, pop, kinds, has_this = FLOAT_SEAMS[a]
            this = u.reg_read(UC_X86_REG_ECX)
            args = [self.describe_arg(k, self.arg(i)) for i, k in enumerate(kinds) if k != 'x']
            self.record(name, this, args, has_this)
            self.w(self.fscr, self.give(name, this, args))
            u.reg_write(UC_X86_REG_EIP, self.ftramp[pop])
            return
        if a == 0x856370:
            self.ret(0, self.strmgr)
            return
        if SSTUB <= a < SSTUB + 0x100:
            slot = (a - SSTUB) // 4
            if slot != 3:
                raise RuntimeError(f'unexpected string manager slot {slot}')
            nil = self.alloc(0x20)
            self.w(nil, self.strmgr)
            self.w(nil + 0xc, 0x100000)
            self.nil_strings.add(nil + 0x10)
            self.ret(0, nil)
            return
        if a == 0x405b20:
            # CString::FormatV(fmt, va): run the ORIGINAL CRT _vswprintf (96C7EC)
            # into a scratch buffer, then assign the result to the CString.
            sp = u.reg_read(UC_X86_REG_ESP)
            ret, fmt, va = self.r(sp), self.r(sp + 4), self.r(sp + 8)
            self.fmt_frames.append((ret, u.reg_read(UC_X86_REG_ECX), sp))
            sp -= 4
            self.w(sp, FMT_RET)
            self.w(sp + 4, SCRATCH)
            self.w(sp + 8, fmt)
            self.w(sp + 0xc, va)
            u.reg_write(UC_X86_REG_ESP, sp)
            u.reg_write(UC_X86_REG_EIP, 0x96c7ec)
            return
        if a == FMT_RET:
            ret, owner, sp = self.fmt_frames.pop()
            self.w(owner, self.string(self.text(SCRATCH)))
            u.reg_write(UC_X86_REG_ESP, sp + 4 + 8)
            u.reg_write(UC_X86_REG_EIP, ret)
            return
        if a == 0x401b00:
            # CString constructor from a literal: assign an owned fixture string.
            self.w(u.reg_read(UC_X86_REG_ECX), self.string(self.text(self.arg())))
            self.ret(4, u.reg_read(UC_X86_REG_ECX))
            return
        if a == 0x6f7430:
            self.ret()
            return
        if a == 0x9738a9:
            self.ret(0, self.ptd)
            return
        if a in (0x404f30, 0x404fc0, 0x405cb0):
            self.diags.append(hex(a))
            self.ret()
            return
        super().hook(u, a, n, x)

    def special(self, name, this, args):
        if name == 'KingdomResultText':
            out = self.arg(0)
            self.w(out, self.string('result:' + (self.names_by_ptr.get(this) or '?')))
            return out
        if name == 'GameVar':
            known = self.returns.get(name, {}).get('by_arg', {})
            if args[0] not in known or known[args[0]] is None:
                return 0          # variable missing (native reads through notify byte)
            value = known[args[0]]
            p = self.alloc(8)
            b = self.alloc(4)
            self.u.mem_write(b, bytes([value & 0xff]))
            self.w(p, b)
            return p
        if name == 'FindPlacement':
            spec = self.returns.get('FindPlacement', {'ok': 0})
            pos, facing = self.arg(0), self.arg(1)
            for i, w in enumerate(spec.get('pos', [])):
                self.w(pos + i * 4, w)
            for i, w in enumerate(spec.get('facing', [])):
                self.w(facing + i * 4, w)
            return spec.get('ok', 0)
        if name == 'VideoPath':
            out = self.arg(0)
            self.w(out, self.string('video:' + args[0]))
            return out
        if name == 'Rotate2D':
            out = self.returns.get('Rotate2D', [0, 0])
            self.w(self.arg(0), out[0])
            self.w(self.arg(0) + 4, out[1])
            return 0
        if name == 'TransportPath':
            out = self.arg(0)
            r = self.returns.get('TransportPath', [0, 0])
            self.w(out, r[0])
            self.w(out + 4, r[1])
            return out
        if name == 'CreateGroup':
            self.names_by_ptr[this] = 'new_group'
            return this
        if name == 'GameVarString':
            out = self.arg(0)
            self.w(out, self.string('var:' + args[0]))
            return out
        if name == 'UiFindWidget':
            return self.give(name, this, args)
        if name == 'OrganizationHasMember':
            members = self.c['objects'][self.names_by_ptr[this]].get('members', [])
            return int(any(self.gtid(m) == args[0] for m in members))
        if name == 'DifficultyByName':
            v = self.give(name, this, args)
            if v is None or v == 0:
                return 0
            p = self.alloc(0x10)
            self.w(p + 0xc, v['level'] if isinstance(v, dict) else v)
            return p
        return self.give(name, this, args)

    # --- run ------------------------------------------------------------------
    def run(self):
        out = super().run()
        out['calls'] = self.calls
        out['context_word'] = self.r(MANAGER + 0x10)
        out['condition_kingdom'] = self.describe(self.r(MANAGER + 0x24))
        out['event_kingdom'] = self.describe(self.r(MANAGER + 0x28))
        subjects = []
        node = self.r(MANAGER + 0x14)
        while node:
            subjects.append(self.r(node))
            node = self.r(node + 4)
        out['subjects'] = subjects
        cursor = self.r(MANAGER + 0x20)
        out['cursor'] = self.r(cursor) if cursor else None
        kingdoms = {}
        for kname in self.c.get('kingdoms', {}):
            k = self.objects[kname]
            arr, count = self.r(k + 0x300), self.r(k + 0x304)
            entries = []
            for i in range(count):
                e = self.r(arr + i * 4)
                entries.append([self.names_by_ptr[self.r(e)][len('objective:'):], self.r(e + 4)])
            kingdoms[kname] = {'objectives': entries, 'result': self.r(k + 0x19c)}
        out['kingdoms'] = kingdoms
        out['world_290'] = self.describe(self.r(WORLD + 0x290))
        out['flags8'] = {n: self.r(p + 8) for n, p in self.objects.items()}
        return out


def param(arg, item, value, flag=1):
    return core.param(arg, item, value) | {'flag': flag}


def element(opcode, params):
    return {'name': core.NAME[opcode], 'opcode': opcode, 'parameters': params}


def run_cases(cases, path, scope, seams):
    rows = []
    visited = {}
    for c in cases:
        m = None
        try:
            m = Machine(c)
            rows.append({'input': c, 'output': m.run()})
            visited.update(m.visited)
        except Exception as e:
            pc = hex(m.u.reg_read(UC_X86_REG_EIP)) if m else '?'
            raise RuntimeError(f"{c['name']} at {pc}") from e
    windows = []
    for a, n in sorted(visited.items()):
        raw = core.IMAGE.at(a, n)
        assert raw == core.ACCEPTED.at(a, n), hex(a)
        windows.append({'address': hex(a), 'bytes': raw.hex()})
    producer = hashlib.sha256(Path(sys.argv[0]).read_bytes()).hexdigest()
    harness = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    doc = {'schema': 1, 'original_sha256': core.pe.ORIGINAL_SHA, 'v14_sha256': core.pe.ACCEPTED_SHA,
           'producer_sha256': producer, 'harness_sha256': harness, 'cases': rows,
           'visited_windows': windows, 'seams': seams, 'scope': scope}
    path.write_text(json.dumps(doc, separators=(",", ":")) + '\n')
    print(json.dumps({'file': path.name, 'cases': len(rows),
                      'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))
    return rows
