"""Native tile zeroing and actual WorldRules mode explored-load gate.

Only mode-name registration/storage and allocation are seams. Mode payload
initialization,59E520/511860,557E6E..91,54EA50 and tile zeroing execute.
No game process starts and default replay never overwrites the fixture.
"""
from pathlib import Path
import hashlib
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase8-movement"))
import harness as h
from unicorn.x86_const import *

H = lambda value: hashlib.sha256(value).hexdigest()
LEAVES = {0x6f7810, 0x4976b0}


class Machine(h.Machine):
    def __init__(self, pc, image):
        super().__init__(pc)
        if image is h.V:
            old = (h.P.image_size + 4095) & ~4095
            new = (image.image_size + 4095) & ~4095
            if new > old:
                self.u.mem_map(image.base + old, new - old)
            self.u.mem_write(image.base, image.b[:4096])
            for vs, r, size, offset in image.sections:
                if size:
                    self.u.mem_write(image.base + r, image.b[offset:offset + min(size, max(vs, size))])
        self.table = self.alloc(7 * 4)
        self.w32(0xb3b198, self.table)
        self.registrations = []
        self.seams[0x6f7810] = lambda _: self.ret(value=self.alloc(self.arg()))
        self.seams[0x4976b0] = lambda _: self.register()
        self.call(0x4981a0, self.alloc(0x30))
        assert len(self.registrations) == 7

    def register(self):
        index, text, payload = self.arg(), self.arg(1), self.arg(2)
        name = bytearray()
        while self.u.mem_read(text, 2) != b"\0\0":
            name += self.u.mem_read(text, 2)
            text += 2
        self.registrations.append({"mode": index, "name": name.decode("utf-16-le"),
                                   "payload": list(self.u.mem_read(payload, 2))})
        entry = self.alloc(12)
        self.w32(entry + 8, payload)
        self.w32(self.table + index * 4, entry)
        self.ret(12)

    def mode(self, mode, high, saved, version):
        stream, header, rules, tile, record = [self.alloc(n) for n in (0xa0, 0x70, 0x20, 0x30, 8)]
        self.w32(stream + 0x90, header)
        self.w32(header + 4, rules)
        self.w32(rules + 4, mode)
        self.call(0x59e520, stream)
        gate = self.u.reg_read(UC_X86_REG_EAX) & 255
        self.w32(tile + 0x14, high)
        self.w32(record, saved)
        self.u.reg_write(UC_X86_REG_ESP, h.STACK)
        self.w32(h.STACK + 0x10, version)
        self.w32(h.STACK + 0x5c, stream)
        self.u.reg_write(UC_X86_REG_EBP, tile)
        self.u.reg_write(UC_X86_REG_EBX, record + 4)
        self.u.emu_start(0x557e6e, 0x557e91, count=1000)
        assert self.u.reg_read(UC_X86_REG_EIP) == 0x557e91
        expected = ((high & 0xffff0000) | saved) if version >= 4 and gate else high
        assert self.r32(tile + 0x14) == expected
        return {"mode": mode, "terrain_version": version, "initial_fog": high,
                "saved_explored": saved, "gate_al": gate, "fog_after": expected}

    def zero(self, cells):
        tile = self.alloc(cells * 48)
        self.u.mem_write(tile, b"\xa5" * (cells * 48))
        map_object = self.alloc(0x60)
        self.u.reg_write(UC_X86_REG_ESP, h.STACK)
        self.u.reg_write(UC_X86_REG_EAX, tile)
        self.u.reg_write(UC_X86_REG_EDI, cells * 48)
        self.u.reg_write(UC_X86_REG_ESI, map_object)
        self.u.emu_start(0x5566aa, 0x5566c6, count=10000)
        assert self.r32(map_object + 0x28) == tile
        raw = bytes(self.u.mem_read(tile, cells * 48))
        assert raw == bytes(cells * 48)
        world = self.alloc(0x80)
        self.w32(0xb36534, world)
        self.w32(world + 0x30, map_object)
        self.w8(map_object + 0x2c, 1)
        # Actual constructor54FDC0 source binding follows zero fill. It may
        # change variation/static flags, never zone/fog/supply words here.
        for i in range(cells):
            variation, terrain = self.alloc(0x30), self.alloc(0x60)
            self.w32(variation + 0x1c, terrain)
            self.w32(terrain + 0x1c, i % 3)
            self.w8(terrain + 0x4c, i % 4)
            self.call(0x54fdc0, tile + i * 48, (variation, 0, 0))
            assert [self.r32(tile + i * 48 + off) for off in (4, 0x14, 0x18)] == [0, 0, 0]
        return {"cells": cells, "all_48_byte_words_zero": True,
                "after_actual_54FDC0_binding_zone_words_zero": True,
                "zone_set": 0, "fog": 0, "supply": 0, "initial_zero_sha256": H(raw)}


def main():
    cases = []
    registration = None
    for pc in h.PRECISION:
        for image_name, image in [("original", h.P), ("v14", h.V)]:
            m = Machine(pc, image)
            if registration is None:
                registration = m.registrations
            assert m.registrations == registration
            rows = [m.mode(mode, high, saved, version)
                    for mode in range(7)
                    for high, saved, version in [(0, 0x3210, 7), (0xa55a00ff, 0x9876, 6), (0x12345678, 0xabcd, 3)]]
            cases.append({"precision": pc, "image": image_name, "mode_rows": rows,
                          "zero_rows": [m.zero(n) for n in [1, 12]]})
    for pc in h.PRECISION:
        pair = [x for x in cases if x["precision"] == pc]
        assert pair[0]["mode_rows"] == pair[1]["mode_rows"]
        assert pair[0]["zero_rows"] == pair[1]["zero_rows"]
    pins = [{"va": hex(a), "bytes": raw} for a, raw in sorted(h.VISITED.items()) if a not in LEAVES]
    leaf_pins = [{"va": hex(a), "bytes": raw} for a, raw in sorted(h.VISITED.items()) if a in LEAVES]
    doc = {"schema": 1, "scope": "Actual mode constructor payloads, stream->WorldRules getter, terrain explored-load gate and GMap initial48byte tile zeroing; not whole GWorldLoad",
           "producer_sha256": H(Path(__file__).read_bytes()), "original_sha256": h.ORIGINAL_SHA,
           "v14_sha256": h.V14_SHA, "registration": registration, "cases": cases,
           "executed_address_byte_pins": pins, "intercepted_leaf_entry_pins": leaf_pins,
           "seams": {"0x6f7810": "allocation", "0x4976b0": "register native-constructed mode payload/name into typed table"}}
    raw = (json.dumps(doc, indent=1) + "\n").encode()
    output = HERE / "native-initial-zones.json"
    if "--write" in sys.argv:
        output.write_bytes(raw)
    else:
        assert json.loads(output.read_text()) == doc
    print(json.dumps({"machines": len(cases), "mode_rows": sum(len(x["mode_rows"]) for x in cases),
                      "zero_rows": sum(len(x["zero_rows"]) for x in cases), "executed_pins": len(pins),
                      "leaf_entries": len(leaf_pins), "sha256": H(raw), "immutable": "--write" not in sys.argv}))


if __name__ == "__main__":
    main()
