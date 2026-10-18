"""Synthetic native world for the zone/fog oracles (layout from the original code).

GWorld ([0xB36534]):
  +0x30  GMap*            +0x130 presentation object (50C4C0 this)
  +0x154 kingdom vector {data, count} (may contain nulls; 4F0680/4F06B0 skip them)
  +0x19C leaf zone-set table*, +0x1A0 union zone-set table* (table+4: buckets,
         size, count, auto-grow byte; 515FA0/514E90 insert, 515C90/515D20 remove)
  +0x244 release ZoneSolver*, +0x248 move ZoneSolver*
GMap: +4 tile shift, +8 tile size (binary32), +0xC/+0x10 world extent,
      +0x1C width, +0x20 height, +0x28 tiles (0x30 bytes each).
Tile: +0 terrain chain, +4 zone set*, +8 actor list {actor, next}, +0xC tail,
      +0x14 fog word (low 16 explored, high 16 visible), +0x18 supply word,
      +0x1C GTID, +0x20 flags.
Kingdom: +0x18C index, +0x190 non-major word, +0x228 shared vision mask,
      +0x22C supply mask (u16).
Actor: vtable (+0x108 owner, +0x138 allied(k), +0x140 enemy(k), +0x64/+0x68/
      +0x6C tile visibility callbacks), +8 flags, +0x14 id, +0x20/+0x24 position,
      +0x7C/+0x88/+0x90/+0xB0/+0xD4 component pointers, +0xB8 detection
      component (+0x10 reveal mask), +0x148 zone vector {data, size, capacity}.
"""
import struct
from zones_machine import Machine, GWORLD_GLOBAL, f32

FOG_SETTINGS_GLOBAL = 0xB39AB0
SAI_GLOBAL = 0xB36528
SAI_OBJECT_GLOBAL = 0xB3652C
TILE = 0x30
LEAF_VTABLE, UNION_VTABLE = 0xA323E8, 0xA32388
ZONE_VTABLE = 0xA3229C


class World:
    def __init__(self, m: Machine, width=8, height=8, shift=3, tile_size=8.0, kingdoms=(), fog_enabled=1,
                 sai=False, table_size=3):
        self.m = m
        self.events = m.events
        g = self.g = m.alloc(0x400)
        m.w(GWORLD_GLOBAL, g)
        # zone-set hash tables (auto-grow on, small initial size exercises rehash)
        self.leaf_table = self._table(table_size)
        self.union_table = self._table(table_size)
        m.w(g + 0x19c, self.leaf_table)
        m.w(g + 0x1a0, self.union_table)
        # map
        self.width, self.height, self.shift = width, height, shift
        self.tile_size = tile_size
        mp = self.map = m.alloc(0x100)
        m.w(mp + 4, shift)
        m.w(mp + 8, f32(tile_size))
        m.w(mp + 0xc, f32(width * tile_size))
        m.w(mp + 0x10, f32(height * tile_size))
        m.w(mp + 0x1c, width)
        m.w(mp + 0x20, height)
        self.tiles = m.alloc(TILE * width * height)
        m.w(mp + 0x28, self.tiles)
        m.w(g + 0x30, mp)
        # kingdoms
        self.kingdoms = []
        for k in kingdoms:
            self.kingdoms.append(self._kingdom(**k) if k is not None else 0)
        vec = m.array(self.kingdoms or [0])
        m.w(g + 0x154, vec)
        m.w(g + 0x158, len(self.kingdoms))
        # fog-of-war setting [[B39AB0]+108]
        fog = m.alloc(0x200)
        cell = m.alloc(4)
        m.w(cell, fog_enabled)
        m.w(fog + 0x108, cell)
        m.w(FOG_SETTINGS_GLOBAL, fog)
        # SAI flag [[B36528]+20] and hooks (7A99A0.. on [B3652C])
        sai_obj = m.alloc(0x40)
        m.wb(sai_obj + 0x20, 1 if sai else 0)
        m.w(SAI_GLOBAL, sai_obj)
        m.w(SAI_OBJECT_GLOBAL, m.alloc(0x40))
        for addr, name in ((0x7a99a0, 'sai_occupied_inc'), (0x7a99d0, 'sai_occupied_dec'),
                           (0x7a9a00, 'sai_explored_inc'), (0x7a9a30, 'sai_visible_inc'),
                           (0x7a9a60, 'sai_visible_dec'), (0x7a9a90, 'sai_explored_dec')):
            m.seam(addr, self._sai(name))
        self.actors = []
        self.actor_index = {}

    def _sai(self, name):
        def f(m):
            self.events.append([name, self.tile_index(m.arg(1)), self.kingdom_index(m.arg(2))])
            m.ret(0, 8)
        return f

    def _table(self, size):
        m = self.m
        t = m.alloc(0x14)
        m.w(t, 0)
        m.w(t + 4, m.array([0] * size))
        m.w(t + 8, size)
        m.w(t + 0xc, 0)
        m.wb(t + 0x10, 1)
        return t

    def _kingdom(self, index, nonmajor=0, shared=None, supply=None):
        m = self.m
        k = m.alloc(0x300)
        m.w(k + 0x18c, index)
        m.w(k + 0x190, nonmajor)
        m.w(k + 0x228, (1 << index) if shared is None else shared)
        m.w16(k + 0x22c, (1 << index) if supply is None else supply)
        return k

    def kingdom_index(self, p):
        if p == 0:
            return None
        return self.kingdoms.index(p)

    def tile(self, x, y):
        return self.tiles + TILE * (y * self.width + x)

    def tile_index(self, p):
        if p == 0:
            return None
        d = p - self.tiles
        assert d % TILE == 0 and 0 <= d // TILE < self.width * self.height, hex(p)
        return d // TILE

    # actors -------------------------------------------------------------
    def actor(self, owner=None, allied=(), enemy=(), flags=0, ident=0, reveal_mask=None,
              b0=0, zone_slots=9, record=True):
        """Synthetic actor; owner/allied/enemy are kingdom indices."""
        m = self.m
        a = m.alloc(0x200)
        idx = len(self.actors)
        info = {'owner': owner, 'allied': set(allied), 'enemy': set(enemy)}
        self.actors.append(a)
        self.actor_index[a] = idx

        def owner_fn(m, info=info, idx=idx):
            v = 0 if info['owner'] is None else self.kingdoms[info['owner']]
            if record:
                self.events.append(['owner', idx])
            m.ret(v)

        def rel(name, key):
            def f(m, info=info, idx=idx):
                k = self.kingdom_index(m.arg(1))
                v = 1 if k in info[key] else 0
                if record:
                    self.events.append([name, idx, k])
                m.ret(v, 4)
            return f
        vt = m.vtable({0x108: owner_fn, 0x138: rel('allied', 'allied'), 0x140: rel('enemy', 'enemy')})
        m.w(a, vt)
        m.w(a + 8, flags)
        m.w(a + 0x14, ident or (1000 + idx))
        m.w(a + 0xb0, b0)
        if reveal_mask is not None:
            comp = m.alloc(0x40)
            m.w(comp + 0x10, reveal_mask)
            m.w(a + 0xb8, comp)
        else:
            m.w(a + 0xb8, 0)
        if zone_slots:
            data = m.array([0] * zone_slots)
            m.w(a + 0x148, data)
            m.w(a + 0x14c, zone_slots)
            m.w(a + 0x150, zone_slots)
        info['vt'] = vt
        info['addr'] = a
        return a

    def actor_idx(self, p):
        if p == 0:
            return None
        return self.actor_index[p]

    # zone-set structure dump ------------------------------------------------
    def set_list(self, s):
        """Native iteration order of a zone set: [(actor index, flags)]."""
        out = []
        m = self.m
        while s:
            vt = m.r(s)
            if vt == LEAF_VTABLE:
                out.append([self.actor_idx(m.r(s + 8)), m.r16(s + 0xc)])
                return out
            assert vt == UNION_VTABLE, hex(vt)
            head = m.r(s + 0xc)
            assert m.r(head) == LEAF_VTABLE
            out.append([self.actor_idx(m.r(head + 8)), m.r16(head + 0xc)])
            s = m.r(s + 8)
        return out

    def live_sets(self):
        """Every interned zone set (leaf and union tables), as native lists, sorted."""
        m = self.m
        lists = []
        for table, link in ((self.leaf_table, 0x10), (self.union_table, 0x14)):
            buckets, size = m.r(table + 4), m.r(table + 8)
            count = 0
            for i in range(size):
                e = m.r(buckets + 4 * i)
                while e:
                    lists.append(self.set_list(e) + [['refs', m.r(e + 4)]])
                    count += 1
                    e = m.r(e + link)
            assert count == m.r(table + 0xc), (count, m.r(table + 0xc))
        return sorted(lists, key=lambda l: str(l))


# ---------------------------------------------------------------------------
# Full zone world (KKC_Zone objects, tile actor lists, actor callbacks)

SVAR_GUARDS = {  # static-init guard, cached pointer (pre-initialised SVar statics)
    'move': (0xB3D460, 0xB3D45C), 'remove': (0xB3D44C, 0xB3D448), 'kingdom': (0xB3D454, 0xB3D450)}
EFFECTS = {0x568d80: 'Guard', 0x57ab70: 'ControlOrganization', 0x567840: 'ControlB0',
           0x58cea0: 'Type3', 0x564300: 'Type8'}


class ZoneWorld(World):
    """World with real KKC_Zone objects and recorded actor/tile services.

    Pre-initialised statics (the original reads them through 4B2E60 once):
    the three ZoneBuilderFastDetectionZones SVar pointers. Seams: 404F60 log,
    50C4C0 presentation, 4EE700 multi-tile test, 4EA200 actor tile, component
    effects 568D80/57AB70/567840/58CEA0/564300, SAI hooks."""

    def __init__(self, m, fast=1, effects_gate=1, solvers=(1, 1), **kw):
        super().__init__(m, **kw)
        g = self.g
        svar = m.alloc(4)
        m.wb(svar, fast)
        for guard, ptr in SVAR_GUARDS.values():
            m.w(guard, m.r(guard) | 1)
            m.w(ptr, svar)
        self.svar = svar
        # presentation object and solver objects (fast path: only non-null)
        m.w(g + 0x130, m.alloc(0x10))
        m.w(g + 0x244, m.alloc(0x40) if solvers[0] else 0)
        m.w(g + 0x248, m.alloc(0x58) if solvers[1] else 0)
        # 54F7F0 gate: [B36550]+E4 != 3 || GWorld+48 != 0
        o = m.alloc(0x100)
        m.w(0xB36550, o)
        m.w(o + 0xe4, 0 if effects_gate else 3)
        m.w(g + 0x48, 0)
        self.zone_index = {}
        self.zones = []
        m.seam(0x404f60, self._log)
        m.seam(0x50c4c0, self._present)
        m.seam(0x4ee700, lambda m: (self.events.append(['multi', self.actor_idx(m.ecx())]),
                                    m.ret(self.multi.get(self.actor_idx(m.ecx()), 0))))
        m.seam(0x4ea200, self._actor_tile)
        for a, name in EFFECTS.items():
            m.seam(a, self._effect(name))
        self.multi = {}
        self.actor_tiles = {}

    def _log(self, m):
        self.events.append(['log', m.arg(6), m.arg(7) & 0xffff])
        m.ret(0)

    def _present(self, m):
        self.events.append(['present', self.kingdom_index(m.arg(1)), m.arg(2), m.arg(3), m.arg(4),
                            m.arg(5) & 0xffff])
        m.ret(0, 0x14)

    def _actor_tile(self, m):
        a = self.actor_idx(m.ecx())
        x, y = self.actor_tiles.get(a, (0, 0))
        self.events.append(['actor_tile', a])
        m.ret(self.tiles + 0x30 * (y * self.width + x))

    def _effect(self, name):
        def f(m):
            self.events.append(['effect', name, m.ecx(), self.actor_idx(m.arg(1))])
            m.ret(0, 4)
        return f

    def full_actor(self, owner=None, allied=(), enemy=(), flags=0, reveal_mask=None, b4=1,
                   components=None, redirect=None, tile=(0, 0), multi=0):
        a = self.actor(owner=owner, allied=allied, enemy=enemy, flags=flags, reveal_mask=reveal_mask)
        idx = self.actor_idx(a)
        info_vt = self.m.r(a)
        m = self.m

        def cb(name, nargs):
            def f(m, idx=idx):
                args = [m.arg(i + 1) for i in range(nargs)]
                ev = [name, idx, self.kingdom_index(args[0])] + [x & 0xff for x in args[1:]]
                self.events.append(ev)
                m.ret(0, 4 * nargs)
            return f
        for off, f in ((0x64, cb('reveal', 3)), (0x68, cb('hide', 1)), (0x6c, cb('unexplore', 1)),
                       (0xb4, lambda m, idx=idx, b4=b4: (self.events.append(['b4', idx]), m.ret(b4)))):
            m.w(info_vt + off, m.stub(f))
        for off, v in (components or {}).items():
            m.w(a + off, v)
        if redirect is not None:
            c = m.alloc(0x20)
            m.w(c + 0x10, redirect[0])
            m.wb(c + 0x1c, redirect[1])
            m.w(a + 0x80, c)
        else:
            m.w(a + 0x80, 0)
        self.actor_tiles[idx] = tile
        self.multi[idx] = multi
        return a

    def place(self, actor, x, y):
        """Prepend actor to tile (x, y)'s +8 list (tail +0xC kept)."""
        m = self.m
        t = self.tile(x, y)
        node = m.alloc(8)
        m.w(node, actor)
        m.w(node + 4, m.r(t + 8))
        m.w(t + 8, node)
        if m.r(t + 0xc) == 0:
            m.w(t + 0xc, node)

    def zone(self, actor, zone_type):
        """A real KKC_Zone (0x516640) in actor+0x148[type]."""
        m = self.m
        z = m.alloc(0x18, fill=0xcd)
        m.call(0x516640, ecx=z, args=(actor, 1 << zone_type), pop=8)
        data = m.r(actor + 0x148)
        m.w(data + 4 * zone_type, z)
        self.zone_index[z] = len(self.zones)
        self.zones.append(z)
        return z

    def zone_state(self, z):
        m = self.m
        return [m.r(z + 8), m.r(z + 0xc), m.r(z + 0x10), m.r16(z + 0x14), m.rb(z + 0x16)]

    def tile_state(self, t):
        m = self.m
        p = self.tiles + 0x30 * t
        return [self.set_list(m.r(p + 4)), m.r(p + 0x14), m.r16(p + 0x18)]
