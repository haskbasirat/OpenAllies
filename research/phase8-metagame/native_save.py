"""Original metagame strategic save format (METAGAME chunk payload) under Unicorn.

Writer KKC_MGState::Save 938EE0 (vtable AC77CC slot +0x20) and reader
KKC_MGState::Load 93A3B0 (slot +0x24, after the load-path factory 939930 /
constructor 939690, exactly as the save-game loader 4FC600 does after opening
the "METAGAME" chunk), with every sub-record run as original code:
  MG kingdom 95A470 / 95AAC0 (ctor 95A1C0), MG territory 955160 / 9565E0
  (ctor 954A40, owner 938BC0 + 959F20/95A850/954CF0, armies cleared 956570),
  territory army reference 954970 / 9549C0 (ctor 954860, snapshot ctor 93F590),
  army snapshot 93F1A0 / 9409E0 (captured by 9402E0 inside 956140 when an army
  enters a territory, 940100), event state 95B020 / 95B0F0 (ctor 95AD90),
  battle record 95F000 / 95F100 (ctors 95EFD0 / 95EF90), version helper
  8589B0, wide-string reader 41D290, id readers 4E7EC0 (KKC_FactionData),
  4DAAA0 (KKC_SharedPropertyData), 95AA00 (KKC_MGHelpPopupData), 940680
  (KKC_MGDivisionData), thing lookup 4BDBA0/427BD0, campaign lookup 939A20,
  map/campaign lookups 95D900, 95C0D0, 95C130, 939B60, hash 4154D0/5043F0/
  405030/9395E0/593570, error helper 41DFB0 (formats natively with 405B20).
Also the army MetaGame component record written inside the actor record of
DYNAMIC_THINGS: 93EEA0 / 940740 with the updatable id 50BC10 / 50BCD0.

Worlds: the original InitCreate (mg_campaign.run) of every shipped campaign
with two synthetic campaign events added to the campaign object (+50/+54)
before 939B90 runs (mg_campaign.build is wrapped), so InitCreate creates the
event states natively (95BE70 + 95AD60); perturbed with original code (940100 relocation incl. snapshot capture, 955620
owner change, 95EFD0 record ctor, 95AD90 event ctor) and plain field writes
(component +10 fought / +14 divisions before a relocation, kingdom +38/+3C,
+20 technologies, +2C help popups, MGState +0C/+10/+14/+18/+44/+45/+46/+48/
+4C/+50/+51, territory +20). Every operation is recorded for the Rust replay.

SEAMS (all recorded in the report):
  * stream: S = ctx+3C, vtable {+0 Read(dst,len), +4 Write(src,len),
    +C IsWriting()} over a Python byte buffer (engine file stream). Reading
    past the end raises (engine EOF behaviour is not established).
  * _CxxThrowException 96BEF5: stops the run and reports the message of the
    exception object 41DFB0 built (object +4 = message from 856340).
  * data managers (KKC data registries) are synthetic objects with the native
    layout read by 939A20/48D500/463FA0/93E170/936CF0/427BD0 (+14 alias count
    0, +30 bucket array, +34 bucket count, node {object, next}); buckets are
    chosen with the ORIGINAL hash 405030 run in Unicorn. Contents: campaign
    registry = the campaign object; shared properties = every shop technology
    object + synthetic ones; divisions = every division object; help popups and
    factions = synthetic objects (ids at +8); things = every army Thing
    (vtable +38 'is actor' seam returns 1) + one synthetic non-actor Thing
    (+38 returns 0). Data objects carry their IDS string at +8 (KKC data base
    layout used by every reader/writer here).
  * B43B40 (shared empty CStringW) is built by its original static
    initializer 9E73C0.
  * world object (B36534) is replaced by a larger copy whose +280 is a save
    session {+0 counter, +4 pointer->id hash} for 50BC10 and +284 a load table
    {+0 count, +4 slots} for 50BCD0 (engine save session).
  * CRT: realloc 97117C (seamed heap, see install_crt_realloc) and the wide
    printf pair _vscwprintf 96C85A / vswprintf 96C7EC used by 405B20 (C-locale
    %s / %hs / %d only; the CRT per-thread data is not initialised).
  * mg_campaign seams (CRT heap, __wcsicmp C locale, kingdom lookup, visuals,
    army actor creation, map table props, kingdom relations, army owner
    vtable+108) unchanged.
"""
import json, hashlib, random, struct, sys
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX
from mg_machine import P, V, VISITED, HERE
import mg_campaign
from mg_campaign import DATA
from mg_world import native_wstring, read_wstring

THROW = 0x96bef5
SYNTHETIC_EVENTS = ('mg_event_synthetic_lend_lease', 'MG_Event_Synthetic_Winter')

_original_build = mg_campaign.build


def build_with_events(m, w, campaign_index):
    """mg_campaign.build, then the synthetic campaign event data objects
    (ids at +4, read by 95C130 / written by 95B020) in campaign +50/+54, so
    the following 939B90 creates one event state per event."""
    result = _original_build(m, w, campaign_index)
    st = result[4]
    campaign = m.r32(st + 8)
    events = []
    for ids in SYNTHETIC_EVENTS:
        e = m.alloc(0x20)
        m.w32(e + 4, native_wstring(m, ids))
        events.append(e)
    w.vector(campaign + 0x50, events)
    return result


mg_campaign.build = build_with_events


class NativeThrow(Exception):
    pass


class StreamEnd(Exception):
    pass


class Stream:
    """Fake engine stream at ctx+0x3C (seamed virtuals)."""

    def __init__(self, m):
        self.m = m
        code = m.alloc(0x40)
        m.u.mem_write(code, b'\xc3' * 0x40)
        self.vt = m.alloc(0x20)
        for i, fn in enumerate((self._read, self._write, self._unexpected, self._is_writing)):
            m.w32(self.vt + 4 * i, code + 8 * i)
            m.seams[code + 8 * i] = fn
        self.ctx = m.alloc(0x100)
        m.w32(self.ctx + 0x3c, self.vt)
        self.S = self.ctx + 0x3c
        self.mode = None
        self.buf = bytearray()
        self.pos = 0
        self.ops = []

    def begin_write(self):
        self.mode, self.buf, self.ops = 'w', bytearray(), []

    def begin_read(self, data):
        self.mode, self.buf, self.pos, self.ops = 'r', bytes(data), 0, []

    def _check(self, mm):
        assert mm.u.reg_read(UC_X86_REG_ECX) == self.S, 'stream call on another object'

    def _write(self, mm):
        self._check(mm)
        assert self.mode == 'w'
        src, n = mm.arg(0), mm.arg(1)
        self.buf += bytes(mm.u.mem_read(src, n))
        self.ops.append(('w', n))
        mm.ret(8)

    def _read(self, mm):
        self._check(mm)
        assert self.mode == 'r'
        dst, n = mm.arg(0), mm.arg(1)
        if self.pos + n > len(self.buf):
            raise StreamEnd(f'read {n} at {self.pos} of {len(self.buf)}')
        mm.u.mem_write(dst, self.buf[self.pos:self.pos + n])
        self.pos += n
        self.ops.append(('r', n))
        mm.ret(8)

    def _is_writing(self, mm):
        self._check(mm)
        mm.ret(0, 1 if self.mode == 'w' else 0)

    def _unexpected(self, mm):
        raise RuntimeError('stream vtable +8 called')


def throw_seam(mm):
    """_CxxThrowException(object, throw info): the 41DFB0 exception object
    (vtable A43DA4) holds its wide message at +4 (856900/856340)."""
    obj = mm.arg(0)
    msg_p = mm.r32(obj + 4)
    out = []
    p = msg_p
    while True:
        ch = int.from_bytes(mm.u.mem_read(p, 2), 'little')
        if ch == 0:
            break
        out.append(chr(ch))
        p += 2
    raise NativeThrow({'vtable': mm.r32(obj), 'throw_info': mm.arg(1), 'message': ''.join(out)})


def _read_w(mm, p):
    out = []
    while True:
        ch = int.from_bytes(mm.u.mem_read(p, 2), 'little')
        if ch == 0:
            return ''.join(map(chr, out))
        out.append(ch)
        p += 2


def _read_a(mm, p):
    out = bytearray()
    while True:
        b = mm.u.mem_read(p, 1)[0]
        if b == 0:
            return out.decode('latin1')
        out.append(b)
        p += 1


def _format(mm, fmt_p, va):
    """CRT wide printf (C locale) for the directives the metagame errors use."""
    fmt = _read_w(mm, fmt_p)
    out, i = [], 0
    while i < len(fmt):
        ch = fmt[i]
        if ch != '%':
            out.append(ch)
            i += 1
            continue
        if fmt.startswith('%%', i):
            out.append('%'); i += 2
        elif fmt.startswith('%hs', i):
            out.append(_read_a(mm, mm.r32(va))); va += 4; i += 3
        elif fmt.startswith('%s', i):
            out.append(_read_w(mm, mm.r32(va))); va += 4; i += 2
        elif fmt.startswith('%d', i) or fmt.startswith('%ld', i):
            n = 3 if fmt.startswith('%ld', i) else 2
            v = mm.r32(va); out.append(str(v - (1 << 32) if v >> 31 else v)); va += 4; i += n
        else:
            raise RuntimeError(f'unsupported directive in {fmt!r}')
    return ''.join(out)


def install_crt_format(m):
    """CRT seams: _vscwprintf 96C85A (length) and vswprintf 96C7EC (write),
    cdecl; the CRT per-thread data (_getptd 9738A9) is not initialised."""
    def count(mm):
        mm.ret(0, len(_format(mm, mm.arg(0), mm.arg(1))))
    def write(mm):
        text = _format(mm, mm.arg(1), mm.arg(2))
        mm.u.mem_write(mm.arg(0), text.encode('utf-16le') + b'\0\0')
        mm.ret(0, len(text))
    m.seams[0x96c85a] = count
    m.seams[0x96c7ec] = write


def install_crt_realloc(m):
    """CRT realloc 97117C (cdecl, reached from the CString allocator 85CAE0
    when a string buffer grows): a new block from the seamed heap with the old
    bytes copied (block sizes recorded by the malloc seams installed here;
    unknown older blocks copy the new size, the CString header bounds use)."""
    from mg_machine import HEAP, HEAP_SIZE
    sizes = {}
    def malloc(mm):
        n = max(mm.arg(0), 4)
        p = mm.alloc(n)
        sizes[p] = n
        mm.ret(0, p)
    def realloc(mm):
        old, n = mm.arg(0), mm.arg(1)
        p = mm.alloc(max(n, 4))
        sizes[p] = n
        if old:
            keep = min(n, sizes.get(old, n), HEAP + HEAP_SIZE - old)
            mm.u.mem_write(p, bytes(mm.u.mem_read(old, keep)))
        mm.ret(0, p)
    for va in (0x96e19f, 0x6f7810):
        m.seams[va] = malloc
    m.seams[0x97117c] = realloc


def run_native(m, target, args=(), ecx=0, count=50_000_000):
    """m.call that turns a native throw / stream end into a Python result."""
    try:
        eax, _ = m.call(target, args=args, ecx=ecx, count=count)
        return ('ok', eax)
    except RuntimeError as e:
        cause = e.__cause__
        while cause is not None and not isinstance(cause, (NativeThrow, StreamEnd)):
            cause = cause.__cause__ or cause.__context__
        if isinstance(cause, NativeThrow):
            return ('throw', cause.args[0])
        if isinstance(cause, StreamEnd):
            return ('eof', str(cause))
        raise


class World:
    """Save/load environment on top of an mg_campaign InitCreate world."""

    def __init__(self, ci):
        snap, ctx = mg_campaign.run(ci, keep=True)
        self.ci = ci
        self.init_snapshot = snap
        self.ctx = ctx
        m = self.m = ctx['m']
        self.w = ctx['w']
        self.st = ctx['st']
        self.c = ctx['c']
        m.seams[THROW] = throw_seam
        install_crt_format(m)
        install_crt_realloc(m)
        # B43B40: original static initializer (CStringW default ctor + atexit seam)
        m.call(0x9e73c0)
        self.stream = Stream(m)
        # data IDS at +8 of every data object the save writes
        self.thing_ids = {v: k for k, v in ctx['things'].items()}
        for ids, th in ctx['things'].items():
            m.w32(th + 8, native_wstring(m, ids))
        self.division_ids = {}
        for dv, dd in zip(ctx['divisions'], DATA['divisions']):
            m.w32(dv + 8, native_wstring(m, dd['ids']))
            self.division_ids[dv] = dd['ids']
        self.tech_ids = {v: k for k, v in ctx['techs'].items()}
        for ids, t in ctx['techs'].items():
            m.w32(t + 8, native_wstring(m, ids))
        # thing vtable: +38 'is actor' (engine Thing virtual) seam
        code = m.alloc(0x400)
        m.u.mem_write(code, b'\xc3' * 0x400)
        def unexpected(off):
            def f(mm):
                raise RuntimeError(f'thing vtable +{off:#x}')
            return f
        def vtable(is_actor):
            vt = m.alloc(0x200)
            for off in range(0, 0x200, 4):
                m.w32(vt + off, code + off + (0x200 if is_actor else 0))
                m.seams[code + off + (0x200 if is_actor else 0)] = unexpected(off)
            m.seams[code + 0x38 + (0x200 if is_actor else 0)] = lambda mm, v=is_actor: mm.ret(0, int(v))
            return vt
        self.vt_actor_thing, self.vt_plain_thing = vtable(True), vtable(False)
        thing_type = m.alloc(0x20)
        for th in ctx['things'].values():
            m.w32(th, self.vt_actor_thing)
            m.w32(th + 0x18, thing_type)
        # synthetic data objects (ids at +8)
        def data_object(ids, vt=0):
            o = m.alloc(0x40)
            m.w32(o, vt)
            m.w32(o + 8, native_wstring(m, ids))
            return o
        self.data_object = data_object
        self.plain_thing = data_object('mg_synthetic_non_actor_thing', self.vt_plain_thing)
        m.w32(self.plain_thing + 0x18, thing_type)
        self.factions = {ids: data_object(ids) for ids in ('faction_allies', 'faction_axis', 'Faction_Neutral')}
        self.popups = {ids: data_object(ids) for ids in ('mg_help_popup_purchase', 'mg_help_popup_attack', 'MG_Help_Popup_Victory')}
        self.extra_props = {ids: data_object(ids) for ids in ('shared_property_synthetic_a', 'Shared_Property_Synthetic_B')}
        # registries
        root = m.alloc(0x900)
        m.w32(0xb36530, root)
        campaign = m.r32(self.st + 8)
        self.campaign = campaign
        cm_holder = m.alloc(0x40)
        m.w32(root + 0x820, cm_holder)
        m.w32(cm_holder + 0x18, self.registry([campaign]))
        m.w32(root + 0xc, self.registry(list(ctx['things'].values()) + [self.plain_thing]))
        m.w32(0xb3aa4c, self.registry(list(self.factions.values())))
        m.w32(0xb3aa2c, self.registry(list(ctx['techs'].values()) + list(self.extra_props.values())))
        m.w32(0xb4a074, self.registry(list(self.popups.values())))
        m.w32(0xb4a06c, self.registry(list(ctx['divisions'])))
        # world replacement with an engine save session (+280) / load table (+284)
        old = m.r32(0xb36534)
        world = m.alloc(0x400)
        m.u.mem_write(world, bytes(m.u.mem_read(old, 0x100)))
        m.w32(0xb36534, world)
        self.session = m.alloc(0x40)
        m.w32(world + 0x280, self.session)
        self.load_table = m.alloc(0x10)
        m.w32(world + 0x284, self.load_table)
        self.reset_session(0)
        # synthetic campaign events (added before InitCreate, build_with_events)
        self.event_data = self.w.read_vector(campaign + 0x50)
        # map territory hash (+18, read by 95D900 through 4154D0): built with the
        # original bucket function 5043F0 and the original insert 9395E0
        mapo = m.r32(campaign + 0x18)
        self.map = mapo
        hb = 61
        arr = m.alloc(4 * hb)
        m.w32(mapo + 0x18, arr)
        m.w32(mapo + 0x1c, hb)
        m.w32(mapo + 0x20, 0)
        m.u.mem_write(mapo + 0x24, b'\0')
        for d in ctx['defs']:
            bucket, _ = m.call(0x5043f0, args=(m.r32(d + 4),), ecx=mapo + 0x18)
            slot = m.alloc(4)
            m.w32(slot, d)
            m.call(0x9395e0, args=(d + 4, slot, bucket), ecx=mapo + 0x18)
        self.kingdoms = self.w.read_vector(self.st + 0x1c)
        self.territories = self.w.read_vector(self.st + 0x28)
        self.armies = [a['actor'] for a in ctx['created']]
        self.comps = [m.r32(a + 0xcc) for a in self.armies]

    def registry(self, objects, buckets=13):
        m = self.m
        mgr = m.alloc(0x40)
        arr = m.alloc(4 * buckets)
        m.w32(mgr + 0x30, arr)
        m.w32(mgr + 0x34, buckets)
        for obj in objects:
            slot = m.alloc(4)
            m.w32(slot, m.r32(obj + 8))
            h, _ = m.call(0x405030, args=(slot,))
            b = h % buckets
            node = m.alloc(8)
            m.w32(node, obj)
            m.w32(node + 4, m.r32(arr + 4 * b))
            m.w32(arr + 4 * b, node)
        return mgr

    def reset_session(self, counter, table_slots=64):
        m = self.m
        m.w32(self.session, counter)
        buckets = m.alloc(4 * 7)
        m.w32(self.session + 4, buckets)
        m.w32(self.session + 8, 7)
        m.w32(self.session + 0xc, 0)
        m.u.mem_write(self.session + 0x10, b'\0')
        arr = m.alloc(4 * (table_slots + 1))
        m.w32(self.load_table, table_slots)
        m.w32(self.load_table + 4, arr)
        self.load_slots = arr

    # ------------------------------------------------------------ probes
    def install_probes(self):
        """Stream offsets at the entry of each sub-record writer (no seam:
        an extra Unicorn code hook that only reads the buffer length)."""
        self.probes = []
        names = {0x95a470: 'kingdom', 0x955160: 'territory', 0x93f1a0: 'snapshot',
                 0x95f000: 'record', 0x95b020: 'event', 0x93eea0: 'component', 0x50bc10: 'updatable'}
        for va, name in names.items():
            def cb(u, addr, size, _, name=name):
                if self.stream.mode == 'w':
                    self.probes.append((name, len(self.stream.buf)))
            self.m.u.hook_add(UC_HOOK_CODE, cb, begin=va, end=va)

    # ------------------------------------------------------------ save / load
    def save(self, st=None):
        st = st or self.st
        self.probes = []
        self.stream.begin_write()
        r = run_native(self.m, 0x938ee0, args=(self.stream.ctx,), ecx=st)
        assert r[0] == 'ok', r
        return bytes(self.stream.buf), list(self.probes)

    def load(self, data):
        """4FC600 order: 939930 (new MGState via 939690, published in B49FA0),
        then Load 93A3B0. B49FA0 is cleared first so the source state is not
        deleted by 939930 (it is kept for comparison)."""
        m = self.m
        m.w32(0xb49fa0, 0)
        r = run_native(m, 0x939930)
        assert r[0] == 'ok'
        new = r[1]
        assert m.r32(0xb49fa0) == new
        self.stream.begin_read(data)
        r = run_native(m, 0x93a3b0, args=(self.stream.ctx,), ecx=new)
        consumed = self.stream.pos
        m.w32(0xb49fa0, self.st)
        if r[0] == 'throw':
            return {'result': 'throw', 'message': r[1]['message'], 'consumed': consumed}, None
        if r[0] == 'eof':
            return {'result': 'eof', 'detail': r[1], 'consumed': consumed}, None
        return {'result': 'ok', 'consumed': consumed}, new

    # ------------------------------------------------------------ dumps
    def ids_at(self, p, off):
        return None if p == 0 else read_wstring(self.m, self.m.r32(p + off))

    def dump_snapshot(self, snap):
        m, vec = self.m, self.w.read_vector
        return {
            'fought': s32(m.r32(snap)),
            'territory': self.ids_at(m.r32(snap + 4), 4),
            'path': [self.ids_at(p, 4) for p in vec(snap + 8)],
            'divisions': [self.ids_at(d, 8) for d in vec(snap + 0x14)],
            'owner_name': read_wstring(m, m.r32(snap + 0x20)),
            'definition': self.ids_at(m.r32(snap + 0x24), 8),
        }

    def dump_state(self, st):
        m, vec = self.m, self.w.read_vector
        ks, ts = vec(st + 0x1c), vec(st + 0x28)
        def pos(lst, p):
            if p == 0:
                return None
            return lst.index(p) if p in lst else 'foreign'
        byte = lambda a: m.u.mem_read(a, 1)[0]
        kingdoms = [{
            'state_back_pointer': m.r32(k + 4) == st,
            'data': self.ids_at(m.r32(k + 8), 4),
            'actor_handle': m.r32(k + 0xc),
            'capital': pos(ts, m.r32(k + 0x10)),
            'territories': [pos(ts, t) for t in vec(k + 0x14)],
            'technologies': [self.ids_at(t, 8) for t in vec(k + 0x20)],
            'help_popups': [self.ids_at(p, 8) for p in vec(k + 0x2c)],
            'resources': s32(m.r32(k + 0x38)),
            'battles': s32(m.r32(k + 0x3c)),
        } for k in ks]
        territories = [{
            'index': m.r32(t + 4),
            'has_record': m.r32(t + 8) != 0,
            'definition': self.ids_at(m.r32(t + 0xc), 4),
            'visual_handle': m.r32(t + 0x20),
            'owner': pos(ks, m.r32(t + 0x24)),
            'enemy_army_count': s32(m.r32(t + 0x1c)),
            'armies': [{'handle': m.r32(r + 4), 'slot': s32(m.r32(r + 8)),
                        'snapshot': self.dump_snapshot(m.r32(r + 0xc))} for r in vec(t + 0x10)],
        } for t in ts]
        return {
            'campaign': self.ids_at(m.r32(st + 8), 8),
            'faction': self.ids_at(m.r32(st + 0xc), 8),
            'current': pos(ks, m.r32(st + 0x10)),
            'round': s32(m.r32(st + 0x14)),
            'rng': m.r32(st + 0x18),
            'bytes_44_45_46_50_51': [byte(st + o) for o in (0x44, 0x45, 0x46, 0x50, 0x51)],
            'string_48': read_wstring(m, m.r32(st + 0x48)),
            'last_purchased_handle': m.r32(st + 0x4c),
            'kingdoms': kingdoms,
            'territories': territories,
            'events': [{'event': self.ids_at(m.r32(e + 8), 4), 'flag': byte(e + 0xc),
                        'target': self.ids_at(m.r32(e + 0x10), 4)} for e in vec(st + 0x54)],
            'records': [{'territory': pos(ts, m.r32(e + 4)), 'attacker': pos(ks, m.r32(e + 8)),
                         'defender': pos(ks, m.r32(e + 0xc)), 'attacker_won': byte(e + 0x10)}
                        for e in vec(st + 0x60)],
        }

    # ------------------------------------------------------------ operations
    def apply(self, op):
        """Apply one recorded operation to the native source state."""
        m, w, st = self.m, self.w, self.st
        kind = op[0]
        if kind == 'fought':
            m.w32(self.comps[op[1]] + 0x10, op[2])
        elif kind == 'divisions':
            w.vector(self.comps[op[1]] + 0x14, [self.ctx['divisions'][i] for i in op[2]], 5)
        elif kind == 'move':
            m.call(0x940100, args=(self.territories[op[2]],), ecx=self.comps[op[1]])
        elif kind == 'owner':
            m.call(0x955620, args=(self.kingdoms[op[2]],), ecx=self.territories[op[1]])
        elif kind == 'waypoints':
            comp = self.comps[op[1]]
            head = tail = 0
            for t in op[2]:
                node = m.alloc(8)
                m.w32(node, self.territories[t])
                if tail:
                    m.w32(tail + 4, node)
                else:
                    head = node
                tail = node
            m.w32(comp + 0x30, head)
            m.w32(comp + 0x34, tail)
        elif kind == 'resources':
            m.w32(self.kingdoms[op[1]] + 0x38, op[2])
        elif kind == 'battles':
            m.w32(self.kingdoms[op[1]] + 0x3c, op[2])
        elif kind == 'technologies':
            pool = dict(self.ctx['techs'], **self.extra_props)
            w.vector(self.kingdoms[op[1]] + 0x20, [0 if i is None else pool[i] for i in op[2]], 10)
        elif kind == 'help_popups':
            w.vector(self.kingdoms[op[1]] + 0x2c, [0 if i is None else self.popups[i] for i in op[2]], 30)
        elif kind == 'visual':
            m.w32(self.territories[op[1]] + 0x20, op[2])
        elif kind == 'round':
            m.w32(st + 0x14, op[1])
        elif kind == 'rng':
            m.w32(st + 0x18, op[1])
        elif kind == 'flags':
            for off, v in zip((0x44, 0x45, 0x46, 0x50, 0x51), op[1:]):
                m.u.mem_write(st + off, bytes([v]))
        elif kind == 'string_48':
            m.w32(st + 0x48, native_wstring(m, op[1]))
        elif kind == 'last_purchased':
            m.w32(st + 0x4c, 0 if op[1] is None else m.r32(self.armies[op[1]] + 0x14))
        elif kind == 'faction':
            m.w32(st + 0xc, 0 if op[1] is None else self.factions[op[1]])
        elif kind == 'current':
            m.w32(st + 0x10, self.kingdoms[op[1]])
        elif kind == 'records':
            recs = []
            for t, ka, kd, won in op[1]:
                e = m.alloc(0x14)
                m.call(0x95efd0, args=(self.territories[t], self.kingdoms[ka], self.kingdoms[kd], won), ecx=e)
                recs.append(e)
            w.vector(st + 0x60, recs, 1)
        elif kind == 'events':
            evs = []
            for ev, flag, k in op[1]:
                e = m.alloc(0x14)
                m.call(0x95ad90, args=(self.campaign,), ecx=e)
                m.w32(e + 8, self.event_data[ev])
                m.u.mem_write(e + 0xc, bytes([flag]))
                m.w32(e + 0x10, 0 if k is None else self.ctx['kdatas'][k])
                evs.append(e)
            w.vector(st + 0x54, evs, 10)
        elif kind == 'registered':
            m.w32(self.comps[op[1]] + 0xc, 1 if op[2] else 0)
        else:
            raise ValueError(op)

    def perturb(self, rng):
        ops = []
        n_a, n_t, n_k = len(self.armies), len(self.territories), len(self.kingdoms)
        for _ in range(rng.randint(3, 14)):
            a = rng.randrange(n_a)
            if rng.random() < 0.4:
                ops.append(['fought', a, rng.choice([0, 0, 1, 2, 3])])
            if rng.random() < 0.3:
                ops.append(['divisions', a, [rng.randrange(len(self.ctx['divisions'])) for _ in range(rng.randint(0, 5))]])
            comp = self.comps[a]
            cur = self.territories.index(self.m.r32(comp + 0x20))
            tdef = self.m.r32(self.territories[cur] + 0xc)
            lk = self.w.read_vector(tdef + 0x68)
            if lk and rng.random() < 0.7:
                target = self.m.r32(rng.choice(lk) + 8)
                t = next(i for i, tp in enumerate(self.territories) if self.m.r32(tp + 0xc) == target)
            else:
                t = rng.randrange(n_t)
            ops.append(['move', a, t])
            if rng.random() < 0.25:
                ops.append(['fought', a, rng.choice([0, 1, 2])])
        for _ in range(rng.randint(0, 6)):
            ops.append(['owner', rng.randrange(n_t), rng.randrange(n_k)])
        for _ in range(rng.randint(0, 4)):
            ops.append(['waypoints', rng.randrange(n_a), [rng.randrange(n_t) for _ in range(rng.randint(0, 4))]])
        techs = list(self.ctx['techs']) + list(self.extra_props)
        for k in range(n_k):
            if rng.random() < 0.6:
                ops.append(['resources', k, rng.choice([0, 5, 40, 125, 1000, rng.randint(-50, 5000)])])
            if rng.random() < 0.4:
                ops.append(['battles', k, rng.choice([0, 1, 2, 7])])
            if rng.random() < 0.5:
                ops.append(['technologies', k, [rng.choice(techs) if rng.random() < 0.92 else None
                                                for _ in range(rng.randint(0, 6))]])
            if rng.random() < 0.3:
                ops.append(['help_popups', k, [rng.choice(list(self.popups)) if rng.random() < 0.9 else None
                                               for _ in range(rng.randint(0, 3))]])
        for _ in range(rng.randint(0, 3)):
            ops.append(['visual', rng.randrange(n_t), rng.choice([0, rng.getrandbits(32)])])
        ops.append(['round', rng.randint(1, 40)])
        ops.append(['rng', rng.getrandbits(32)])
        ops.append(['flags', rng.randint(0, 1), rng.randint(0, 1), rng.randint(0, 1), rng.randint(0, 1),
                    rng.choice([0, 1, 1, 2, 0x7f, 0xff])])
        if rng.random() < 0.5:
            ops.append(['string_48', rng.choice(['', 'rmc_battle_key', 'Territory_Philippines', 'x' * 40])])
        ops.append(['last_purchased', rng.choice([None, rng.randrange(n_a)])])
        ops.append(['faction', rng.choice([None] + list(self.factions))])
        ops.append(['current', rng.randrange(n_k)])
        if rng.random() < 0.7:
            ops.append(['records', [[rng.randrange(n_t), rng.randrange(n_k), rng.randrange(n_k), rng.randint(0, 1)]
                                    for _ in range(rng.randint(0, 4))]])
        if rng.random() < 0.3:
            ops.append(['events', [[rng.randrange(len(self.event_data)), rng.randint(0, 1),
                                    rng.choice([None, rng.randrange(n_k)])]
                                   for _ in range(rng.randint(0, 3))]])
        for _ in range(rng.randint(0, 4)):
            ops.append(['registered', rng.randrange(n_a), rng.random() < 0.8])
        for op in ops:
            self.apply(op)
        return ops

    # ------------------------------------------------------------ army components
    def dump_component(self, comp):
        m, vec = self.m, self.w.read_vector
        ts = self.territories
        def tpos(p):
            return None if p == 0 else (ts.index(p) if p in ts else 'foreign')
        way = []
        node = m.r32(comp + 0x30)
        while node:
            way.append(tpos(m.r32(node)))
            node = m.r32(node + 4)
        return {
            'fought': s32(m.r32(comp + 0x10)),
            'territory': tpos(m.r32(comp + 0x20)),
            'path': [tpos(t) for t in vec(comp + 0x24)],
            'waypoints': way,
            'divisions': [self.ids_at(d, 8) for d in vec(comp + 0x14)],
        }

    def component_round_trip(self, a, counter):
        """93EEA0 on army a's component, then 940740 into a fresh component
        (B49FA0 = source state, as the territory indices refer to it)."""
        m, w = self.m, self.w
        self.reset_session(counter)
        self.stream.begin_write()
        r = run_native(m, 0x93eea0, args=(self.stream.ctx,), ecx=self.comps[a])
        assert r[0] == 'ok', r
        data = bytes(self.stream.buf)
        row = {'army': a, 'session_counter': counter, 'registered': m.r32(self.comps[a] + 0xc) != 0,
               'bytes': data.hex(), 'source': self.dump_component(self.comps[a]),
               'session_counter_after': m.r32(self.session)}
        row['load'] = self.component_load(data)
        return row

    def component_load(self, data):
        m, w = self.m, self.w
        fresh = m.alloc(0x40)
        w.vector(fresh + 0x14, [], 5)
        w.vector(fresh + 0x24, [], 1)
        self.stream.begin_read(data)
        r = run_native(m, 0x940740, args=(self.stream.ctx,), ecx=fresh)
        out = {'result': r[0], 'consumed': self.stream.pos}
        if r[0] == 'throw':
            out['message'] = r[1]['message']
        elif r[0] == 'ok':
            out['component'] = self.dump_component(fresh)
            slots = [i for i in range(65) if m.r32(self.load_slots + 4 * i) == fresh + 8]
            out['registered_slots'] = slots
        return out


def s32(v):
    return v - (1 << 32) if v >> 31 else v


def identities(wd):
    m = wd.m
    return {
        'kingdom_actor_handles': [m.r32(a + 0x14) for a in wd.ctx['actors']],
        'kingdom_names': [read_wstring(m, m.r32(a + 0x18)) for a in wd.ctx['actors']],
        'army_handles': [m.r32(a + 0x14) for a in wd.armies],
        'army_owner_kingdom': [wd.ctx['actors'].index(wd.w.army_owner[a]) for a in wd.armies],
    }


def registry_contents(wd):
    """What the synthetic data managers of this world hold (registry order)."""
    return {
        'factions': list(wd.factions),
        'shared_properties': list(wd.ctx['techs']) + list(wd.extra_props),
        'help_popups': list(wd.popups),
        'things': [[ids, True] for ids in wd.ctx['things']] + [['mg_synthetic_non_actor_thing', False]],
        'divisions': [d['ids'] for d in DATA['divisions']],
        'campaigns': [wd.c['ids']],
        'events': [wd.ids_at(e, 4) for e in wd.event_data],
    }


def round_trip(wd):
    data, probes = wd.save()
    src = wd.dump_state(wd.st)
    res, new = wd.load(data)
    row = {'bytes': data.hex(), 'probes': probes, 'source': src, 'load': res}
    assert res['result'] == 'ok', res
    row['loaded'] = wd.dump_state(new)
    again, _ = wd.save(new)
    row['resave_identical'] = again == data
    if again != data:
        row['resave_bytes'] = again.hex()
    return row, data, probes


def u32_patch(offset, value):
    return {'offset': offset, 'remove': 4, 'insert': struct.pack('<I', value & 0xffffffff).hex()}


def wstr_patch(data, offset, fn):
    """Same-length rewrite of the UTF-16 string whose u16 count is at offset."""
    n = struct.unpack_from('<H', data, offset)[0]
    a, b = offset + 2, offset + 2 + 2 * n
    text = fn(data[a:b].decode('utf-16le'))
    assert len(text) == n
    return {'offset': a, 'remove': b - a, 'insert': text.encode('utf-16le').hex()}


def apply_patch(data, p):
    o = p['offset']
    return data[:o] + bytes.fromhex(p['insert']) + data[o + p['remove']:]


def error_cases(wd, base, probes):
    """Reader edge cases: patches of the last round's native bytes (`base`)."""
    cases = []
    data, probes2 = wd.save()
    assert data == base and probes2 == probes
    first = {}
    for name, off in probes:
        first.setdefault(name, off)
    def case(name, patch, note):
        res, new = wd.load(apply_patch(data, patch))
        row = {'case': name, 'note': note, 'patch': patch, 'native': res}
        if new is not None:
            row['loaded'] = wd.dump_state(new)
        cases.append(row)
    case('mgstate_version_1', u32_patch(0, 1), '8589B0 max 0')
    case('mgstate_version_negative', u32_patch(0, 0xffffffff), '8589B0 v<0')
    case('kingdom_version_1', u32_patch(first['kingdom'], 1), '95AAC0 max 0')
    case('territory_version_1', u32_patch(first['territory'], 1), '9565E0 max 0')
    case('snapshot_version_2', u32_patch(first['snapshot'], 2), '9409E0 max 1')
    case('snapshot_version_0', u32_patch(first['snapshot'], 0), '9409E0 accepts 0, same fields')
    if 'record' in first:
        case('record_version_1', u32_patch(first['record'], 1), '95F100 max 0')
    cur = 4 + 2 + 2 * struct.unpack_from('<H', data, 4)[0]
    case('current_case_swapped', wstr_patch(data, cur, str.swapcase), 'current kingdom compared with wcscmp 96CB83 (case-sensitive)')
    case('kingdom_data_upper', wstr_patch(data, first['kingdom'] + 4, str.upper), '95C0D0 case-insensitive')
    case('territory_definition_upper', wstr_patch(data, first['territory'] + 8, str.upper), '95D900 case-insensitive')
    case('campaign_upper', wstr_patch(data, 4, str.upper), '939A20 registry case-insensitive')
    # truncated payloads are not run natively (engine stream EOF behaviour unknown)
    return cases


def synthetic_cases(ci):
    """Fresh worlds with one unregistered / unusual object, written by the
    native writer and read by the native reader."""
    out = []
    def fresh():
        wd = World(ci)
        wd.install_probes()
        return wd
    def run_case(name, setup, note):
        wd = fresh()
        setup(wd)
        data, _ = wd.save()
        res, new = wd.load(data)
        row = {'case': name, 'note': note, 'bytes': data.hex(), 'native': res,
               'identities': identities(wd), 'source': wd.dump_state(wd.st)}
        if new is not None:
            row['loaded'] = wd.dump_state(new)
        out.append(row)
    def unknown_tech(wd):
        wd.w.vector(wd.kingdoms[0] + 0x20, [wd.ctx['techs'][next(iter(wd.ctx['techs']))], wd.data_object('tech_unregistered_synthetic')])
    run_case('unknown_technology', unknown_tech, '4DAAA0 -> 41DFB0')
    def unknown_popup(wd):
        wd.w.vector(wd.kingdoms[1] + 0x2c, [wd.data_object('mg_help_popup_unregistered')])
    run_case('unknown_help_popup', unknown_popup, '95AA00 -> 41DFB0')
    def unknown_faction(wd):
        wd.m.w32(wd.st + 0xc, wd.data_object('faction_unregistered'))
    run_case('unknown_faction', unknown_faction, '4E7EC0 -> 41DFB0')
    def unknown_division(wd):
        bad = wd.data_object('division_unregistered')
        wd.w.vector(wd.comps[0] + 0x14, [wd.ctx['divisions'][0], bad], 5)
        t = wd.territories.index(wd.m.r32(wd.comps[0] + 0x20))
        wd.m.call(0x940100, args=(wd.territories[t],), ecx=wd.comps[0])
    run_case('unknown_snapshot_division', unknown_division, '940680 -> 41DFB0 (snapshot captured by 940100)')
    def non_actor_definition(wd):
        a = wd.armies[1]
        keep = wd.m.r32(a + 4)
        wd.m.w32(a + 4, wd.plain_thing)
        t = wd.territories.index(wd.m.r32(wd.comps[1] + 0x20))
        wd.m.call(0x940100, args=(wd.territories[t],), ecx=wd.comps[1])
        wd.m.w32(a + 4, keep)
    run_case('non_actor_snapshot_definition', non_actor_definition, '4BDBA0: thing vtable+38 false -> null, no error')
    def unknown_definition(wd):
        a = wd.armies[2]
        keep = wd.m.r32(a + 4)
        wd.m.w32(a + 4, wd.data_object('mg_thing_unregistered', wd.vt_actor_thing))
        t = wd.territories.index(wd.m.r32(wd.comps[2] + 0x20))
        wd.m.call(0x940100, args=(wd.territories[t],), ecx=wd.comps[2])
        wd.m.w32(a + 4, keep)
    run_case('unknown_snapshot_definition', unknown_definition, '427BD0 not found -> null, no error')
    def unknown_snapshot_territory(wd):
        refs = wd.w.read_vector(wd.territories[0] + 0x10)
        fake = wd.m.alloc(0x80)
        wd.m.w32(fake + 4, native_wstring(wd.m, 'territory_unregistered'))
        snap = wd.m.r32(refs[0] + 0xc)
        wd.m.w32(snap + 4, fake)
        path = wd.w.read_vector(snap + 8)
        wd.w.vector(snap + 8, path + [fake, wd.ctx['defs'][0]], 5)
    run_case('unknown_snapshot_territory_and_path', unknown_snapshot_territory, '95D900 not found -> null, no error')
    def unknown_record_kingdoms(wd):
        fk = wd.m.alloc(0x40)
        fd = wd.m.alloc(0x20)
        wd.m.w32(fd + 4, native_wstring(wd.m, 'kingdom_unregistered'))
        wd.m.w32(fk + 8, fd)
        e = wd.m.alloc(0x14)
        wd.m.call(0x95efd0, args=(wd.territories[3], fk, wd.kingdoms[0], 1), ecx=e)
        e2 = wd.m.alloc(0x14)
        wd.m.call(0x95efd0, args=(wd.territories[4], wd.kingdoms[1], fk, 0), ecx=e2)
        wd.w.vector(wd.st + 0x60, [e, e2], 1)
    run_case('unknown_record_kingdoms', unknown_record_kingdoms, '938BC0 not found -> null, no error')
    def unknown_record_territory(wd):
        ft = wd.m.alloc(0x28)
        fd = wd.m.alloc(0x80)
        wd.m.w32(fd + 4, native_wstring(wd.m, 'territory_unregistered'))
        wd.m.w32(ft + 0xc, fd)
        e = wd.m.alloc(0x14)
        wd.m.call(0x95efd0, args=(ft, wd.kingdoms[0], wd.kingdoms[1], 1), ecx=e)
        wd.w.vector(wd.st + 0x60, [e], 1)
    run_case('unknown_record_territory', unknown_record_territory, '939B60 not found -> null territory, no error')
    def unknown_events(wd):
        fe = wd.m.alloc(0x20)
        wd.m.w32(fe + 4, native_wstring(wd.m, 'mg_event_unregistered'))
        fd = wd.m.alloc(0x20)
        wd.m.w32(fd + 4, native_wstring(wd.m, 'kingdom_unregistered'))
        evs = []
        for data_obj, k in ((fe, fd), (wd.event_data[0], 0)):
            e = wd.m.alloc(0x14)
            wd.m.call(0x95ad90, args=(wd.campaign,), ecx=e)
            wd.m.w32(e + 8, data_obj)
            wd.m.u.mem_write(e + 0xc, b'\x01')
            wd.m.w32(e + 0x10, k)
            evs.append(e)
        wd.w.vector(wd.st + 0x54, evs, 10)
    run_case('unknown_event_ids', unknown_events, '95C130/95C0D0 not found -> null; event states are not stored')
    return out


def component_cases(wd):
    """Component record reader edge cases on native bytes."""
    rows = []
    wd.reset_session(5)
    wd.m.w32(wd.comps[0] + 0xc, 1)
    wd.stream.begin_write()
    assert run_native(wd.m, 0x93eea0, args=(wd.stream.ctx,), ecx=wd.comps[0])[0] == 'ok'
    data = bytes(wd.stream.buf)
    upd = len(data) - 8
    def case(name, payload, note):
        wd.reset_session(5)
        rows.append({'case': name, 'note': note, 'bytes': payload.hex(), 'native': wd.component_load(payload)})
    def patched(p):
        return apply_patch(data, p)
    case('component_registered', data, 'updatable registered: id from the session counter')
    case('component_version_2', patched(u32_patch(0, 2)), '940740 max 1')
    case('component_version_0', patched(u32_patch(0, 0)), '940740 accepts 0, same fields')
    case('updatable_version_0', patched(u32_patch(upd, 0)), '50BCD0: version 0 reads no id')
    case('updatable_version_2', patched(u32_patch(upd, 2)), '50BCD0 max 1')
    case('updatable_id_negative', patched(u32_patch(upd + 4, 0xffffffff)), '50BCD0: id < 0 registers nothing')
    bad = wd.data_object('division_unregistered')
    wd.w.vector(wd.comps[3] + 0x14, [bad], 5)
    wd.stream.begin_write()
    assert run_native(wd.m, 0x93eea0, args=(wd.stream.ctx,), ecx=wd.comps[3])[0] == 'ok'
    case('component_unknown_division', bytes(wd.stream.buf), '940680 -> 41DFB0')
    wd.m.w32(wd.comps[4] + 0x20, 0)
    wd.stream.begin_write()
    assert run_native(wd.m, 0x93eea0, args=(wd.stream.ctx,), ecx=wd.comps[4])[0] == 'ok'
    rows.append({'case': 'component_null_territory_write', 'note': '93EEA0 writes -1 for a null territory; 9389F0 does not bounds-check, so it is not reloaded here',
                 'bytes': bytes(wd.stream.buf).hex(), 'source': wd.dump_component(wd.comps[4])})
    return rows


SEAMS = __doc__.split('SEAMS (all recorded in the report):')[1].strip()


def main():
    rounds_per_campaign = int(sys.argv[1]) if len(sys.argv) > 1 else 6
    rng = random.Random(0x938ee0)
    campaigns = []
    errors = []
    synthetic = []
    components_edge = []
    totals = {'saves': 0, 'components': 0}
    for ci in range(len(DATA['campaigns'])):
        wd = World(ci)
        wd.install_probes()
        row = {'campaign': ci, 'ids': wd.c['ids'], 'identities': identities(wd),
               'registries': registry_contents(wd), 'rounds': []}
        for r in range(rounds_per_campaign + 1):
            ops = [] if r == 0 else wd.perturb(rng)
            rt, data, probes = round_trip(wd)
            rt['ops'] = ops
            comps = []
            counter = rng.randint(0, 20)
            for a in range(len(wd.armies)):
                c = wd.component_round_trip(a, counter)
                counter = c['session_counter_after']
                if counter > 40:
                    counter = rng.randint(0, 20)
                comps.append(c)
            rt['components'] = comps
            totals['saves'] += 1
            totals['components'] += len(comps)
            row['rounds'].append(rt)
        errors.append({'campaign': ci, 'base_round': len(row['rounds']) - 1, 'cases': error_cases(wd, data, probes)})
        components_edge.append({'campaign': ci, 'cases': component_cases(wd)})
        campaigns.append(row)
        if ci == 0:
            synthetic = synthetic_cases(ci)
        print(f'campaign {ci}: {len(row["rounds"])} rounds', flush=True)
    report = {
        'schema': 1, 'scope': __doc__.split('SEAMS')[0].strip(), 'seams': SEAMS,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'registries': {
            'factions': ['faction_allies', 'faction_axis', 'Faction_Neutral'],
            'help_popups': ['mg_help_popup_purchase', 'mg_help_popup_attack', 'MG_Help_Popup_Victory'],
            'extra_shared_properties': ['shared_property_synthetic_a', 'Shared_Property_Synthetic_B'],
            'non_actor_things': ['mg_synthetic_non_actor_thing'],
            'events': list(SYNTHETIC_EVENTS),
        },
        'campaigns': campaigns,
        'error_cases': errors,
        'synthetic_cases': synthetic,
        'component_cases': components_edge,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-save.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'saves': totals['saves'], 'components': totals['components'],
                      'error_cases': sum(len(e['cases']) for e in errors),
                      'synthetic_cases': len(synthetic),
                      'component_cases': sum(len(e['cases']) for e in components_edge),
                      'visited': len(VISITED), 'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
