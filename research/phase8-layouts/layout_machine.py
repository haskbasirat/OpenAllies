"""Unicorn machine for the phase-8 LayoutData oracles.

Builds on the definitions agent's harness (research/phase8-definitions/
native_machine.py: original 5BD080 TGI parser with line-source/heap/mutex
seams, page-0 null guard, C++ EH seam) and adds:

* 6F7810 operator new fills every new block with 0xA5 (nothing is zeroed by
  the harness; the original constructors initialize what they use);
* recorded leaves with full printf arguments: 410C40 / 41DFB0 throw helpers
  (format + arguments, then the original throw runs and the case stops) and
  the leveled log calls 404EA0..404F90 (channel, file, line, function,
  format, arguments);
* seams (recorded, never executed): 5A77E0 texture-resource load (returns a
  fixture handle), 4C42D0 KKC_ModifierPropertyData::LoadData (records the
  property object and node, returns);
* GameData at [B36530] (the parser harness object) with the original
  DataManagers at +464 (Property, 47EFD0(0x40, 1)), +470 (OrganizationLayout,
  481740(0x3D, 0), also [B3AB00]), +474 (Formation, 481A50(0x11, 0)) and +478
  (GroupFormation, 4840A0(2, 0)), argument values as in the GameData
  constructor 48A8E0 (48ABB3..48ACA6, 48B1B4).
"""
from pathlib import Path
import struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-definitions'))
import native_machine as dm
from native_machine import Machine as DefinitionsMachine, native_windows, P, V, H, lines_of
from unicorn.x86_const import *

DATA = ROOT.parent / 'reconstruction/assets/original/Data'
TEXTURE_BASE = 0x7e000000
GD = 0xb36530


def signed(v):
    return v - (1 << 32) if v & 0x80000000 else v


class Throw(Exception):
    pass


class Machine(DefinitionsMachine):
    def __init__(self):
        self.recorded = []      # ordered seam/log/throw events
        self.textures = []
        self.property_loads = []
        self.last_throw = None
        self.fill = True
        self.string_table = {}
        self.lookups = []
        super().__init__()
        self.gd = self.r(GD)
        assert self.gd
        # 4B9F40 string-table seam: [B36544] table object, B43B48 the static
        # empty default CString (as in the display-name oracle).
        self.w(0xb43b48, self.string([]))
        self.w(0xb36544, self.alloc(0x40))

    # -- operator new with 0xA5 fill ------------------------------------
    def hook(self, u, pc, size, user):
        if pc == 0x6f7810:
            n = max(1, self.arg())
            p = self.alloc(n)
            if self.fill:
                self.u.mem_write(p, b'\xa5' * n)
            self.ret(p)
            return
        if pc in (0x410c40, 0x41dfb0):
            esp = u.reg_read(UC_X86_REG_ESP)
            fmt = self.raw_wstr(self.r(esp + 4))
            args = self.printf_args(fmt, esp + 8)
            self.last_throw = {'function': hex(pc), 'format': fmt, 'args': args}
            self.recorded.append({'event': 'throw', **self.last_throw})
            # fall through: the original helper builds and throws the object
        if pc == 0x96bef5:
            # _CxxThrowException(object, ThrowInfo): observation only. The
            # CStrings built just before (401B00) carry the node exception's
            # message and field name (5B54D0 shape).
            esp = u.reg_read(UC_X86_REG_ESP)
            self.recorded.append({'event': 'cxx_throw', 'throwinfo': hex(self.r(esp + 8)),
                                  'messages': list(self.messages[-3:])})
        if pc in dm.LOG_LEAVES:
            esp = u.reg_read(UC_X86_REG_ESP)
            fmt = self.raw_wstr(self.r(esp + 20))
            self.recorded.append({'event': 'log', 'level_fn': hex(pc), 'channel': hex(self.r(esp + 4)),
                                  'file': self.raw_wstr(self.r(esp + 8)), 'line': self.r(esp + 12),
                                  'function': self.raw_wstr(self.r(esp + 16)), 'format': fmt,
                                  'args': self.printf_args(fmt, esp + 24)})
            self.ret()
            return
        if pc == 0x4b9f40:
            # thiscall(&key, &default) ret 8 -> CString*: fixture string table.
            key = self.wstr(self.r(self.arg()))
            text = self.string_table.get(key)
            self.lookups.append({'key': key, 'found': text is not None})
            self.recorded.append({'event': 'string_lookup', 'key': key, 'found': text is not None})
            if text is None:
                self.ret(self.arg(1), pop=8)
            else:
                obj = self.alloc(4)
                self.w(obj, self.cstring(text))
                self.ret(obj, pop=8)
            return
        if pc == 0x5a77e0:
            # cdecl (CString *path, int); texture resource load (presentation).
            path = self.wstr(self.r(self.arg()))
            handle = TEXTURE_BASE + 0x10 * len(self.textures)
            self.textures.append(path)
            self.recorded.append({'event': 'texture', 'path': path, 'flag': self.arg(1), 'handle': hex(handle)})
            self.ret(handle)
            return
        if pc == 0x4c42d0:
            # thiscall KKC_ModifierPropertyData::LoadData(node) ret 4: seam.
            this = u.reg_read(UC_X86_REG_ECX)
            node = self.arg()
            self.property_loads.append((this, node))
            self.recorded.append({'event': 'property_load', 'object': hex(this), 'node': hex(node)})
            self.ret(0, pop=4)
            return
        super().hook(u, pc, size, user)

    def printf_args(self, fmt, at):
        out = []
        i = 0
        while i < len(fmt):
            if fmt[i] != '%':
                i += 1
                continue
            i += 1
            while i < len(fmt) and fmt[i] in 'hl':
                i += 1
            if i >= len(fmt):
                break
            c = fmt[i]
            i += 1
            if c == '%':
                continue
            v = self.r(at)
            at += 4
            if c == 's':
                try:
                    out.append(self.raw_wstr(v) if v else None)
                except Exception:
                    out.append(hex(v))
            else:
                out.append(signed(v))
        return out

    # -- managers ---------------------------------------------------------
    def build_managers(self):
        self.fill = False
        mgrs = {}
        for off, ctor, args, size in [(0x464, 0x47efd0, [0x40, 1], 0x4c), (0x470, 0x481740, [0x3d, 0], 0x4c),
                                      (0x474, 0x481a50, [0x11, 0], 0x4c), (0x478, 0x4840a0, [2, 0], 0x4c)]:
            obj = self.alloc(0x100)
            r = self.thiscall(ctor, obj, args)
            assert 'eax' in r, r
            self.w(self.gd + off, obj)
            mgrs[off] = obj
        self.fill = True
        assert self.r(0xb3ab00) == mgrs[0x470]
        self.mgr = mgrs
        return mgrs

    def set_current_file(self, path):
        """48D660 head: GameData+4 = 85A550(resource path) (directory)."""
        res = self.alloc(0x40)
        self.w(res + 0x38, self.cstring(path))
        out = self.alloc(4)
        self.cdecl(0x85a550, [out, res + 0x38])
        self.w(self.gd + 4, self.r(out))
        return self.wstr(self.r(out))

    def run_vf0(self, mgr, node):
        """DataManager vf0 483A40(node): vf1C type create/LoadData."""
        self.last_throw = None
        r = self.thiscall(0x483a40, mgr, [node])
        if 'thrown' in r:
            return {'thrown': self.last_throw}
        return {'object': r['eax']}

    def register_created(self, mgr, create_fn, ids):
        """The manager's own create function (registration without LoadData)."""
        cell = self.alloc(4)
        self.w(cell, self.cstring(ids))
        self.last_throw = None
        r = self.thiscall(create_fn, mgr, [cell])
        if 'thrown' in r:
            return {'thrown': self.last_throw}
        return {'object': r['eax']}

    def manager_entries(self, mgr):
        """Manager +24 vector (count +28): definitions in ordinal order."""
        base, n = self.r(mgr + 0x24), self.r(mgr + 0x28)
        return [self.r(base + 4 * i) for i in range(n)]

    def children(self, node):
        return [self.r(self.r(node + 0x34) + 4 * i) for i in range(self.r(node + 0x38))]

    def node_name(self, node):
        return self.wstr(self.r(node))

    def parse_file(self, rel, data=None):
        data = data if data is not None else (DATA / rel).read_bytes()
        r = self.reparse(data, rel)
        if r['error'] is not None:
            raise RuntimeError(f'parse {rel}: {r["error"]}')
        return r
