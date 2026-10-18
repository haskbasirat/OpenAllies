"""Execute original-AA 892B7A..892D5E only; no D3D/client/culling execution.

Stable read-only frustum domain. Renderer flag+54 is a required explicit seam.
Instruction observers inject pre-store/load/service failures, not virtual-read
claims. No downloaded Kohan implementation or DLL is loaded or executed.
"""
from pathlib import Path
import copy
import hashlib
import json
import random
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / "research/phase8-movement"))
import harness as h
from unicorn.x86_const import UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBX, UC_X86_REG_ESP
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

ORIGINAL = h.P
OFFSETS = [0x34C, 0x350, 0x3FC, 0x40C] + list(range(0x6E0, 0x720, 4))
STORE_SITES = {
    0x892B7D:0x34C, 0x892B8F:0x350,
    0x892BE5:0x700, 0x892BEB:0x6F0, 0x892C01:0x6E0,
    0x892C17:0x6E0, 0x892C1F:0x710, 0x892C2F:0x6E4,
    0x892C39:0x704, 0x892C3F:0x6E8, 0x892C45:0x6F8,
    0x892C4B:0x708, 0x892C55:0x6F4, 0x892C61:0x714,
    0x892C6E:0x70C, 0x892C74:0x71C,
    0x892C94:0x710, 0x892C9A:0x6F0, 0x892CB0:0x6E0,
    0x892CBC:0x6E0, 0x892CC4:0x700, 0x892CCA:0x6E4,
    0x892CD6:0x714, 0x892CE0:0x6E8, 0x892CE6:0x6F8,
    0x892CF0:0x6F4, 0x892CFC:0x704, 0x892D09:0x708,
    0x892D0F:0x70C, 0x892D1F:0x71C, 0x892D31:0x718,
    0x892D3E:0x3FC, 0x892D4C:0x6EC, 0x892D52:0x6FC,
    0x892D58:0x40C,
}


class Missing(Exception):
    pass


class Machine(h.Machine):
    def __init__(self, case, image):
        h.P = ORIGINAL if image == "original" else h.V
        super().__init__(case["precision"])
        self.case = case
        self.trace = []
        self.renderer = self.alloc(0x800)
        self.frustum = self.alloc(0x20)
        for offset in OFFSETS:
            self.w32(self.renderer + offset, 0xA5A50000 + offset)
        self.w32(self.frustum, *case["frustum"], int(case["orthographic"]))
        helper = self.alloc(4)
        vtable = self.alloc(0x60)
        self.w32(self.renderer + 0x5E8, helper)
        self.w32(helper, vtable)
        self.w32(vtable + 0x54, h.STUBS)
        self.seams[h.STUBS] = self.flag
        self.u.reg_write(UC_X86_REG_ESI, self.renderer)
        self.u.reg_write(UC_X86_REG_EDI, self.frustum)
        self.u.reg_write(UC_X86_REG_EBX, 0)
        self.u.reg_write(UC_X86_REG_ESP, h.STACK)
        self.u.hook_add(h.UC_HOOK_CODE, self.boundary)

    def event(self, kind, offset=None):
        self.trace.append([kind] if offset is None else [kind, offset])
        n = len(self.trace)
        for change in self.case.get("mutations", []):
            if change["event"] == n:
                self.w32(self.renderer + change["offset"], change["bits"])
        if self.case.get("stop") == n:
            raise Missing(str(n))

    def flag(self, machine):
        self.event("flag54")
        self.ret(0, int(self.case["flag54"]))

    def boundary(self, u, pc, size, _):
        if pc == 0x892D5E:
            u.emu_stop()
        elif pc in STORE_SITES:
            self.event("store", STORE_SITES[pc])
        elif pc == 0x892D29:
            self.event("load", 0x708)
        elif pc == 0x892D37:
            self.event("load", 0x718)

    def execute(self):
        outcome = "ok"
        try:
            self.u.emu_start(0x892B7A, 0x892D5E, count=10000)
        except Missing:
            outcome = "missing"
        if outcome == "ok":
            assert self.fpu_top() == 0
        assert self.r32(self.frustum, 7) == self.case["frustum"] + [int(self.case["orthographic"])]
        return {"outcome":outcome, "trace":self.trace,
                "words":[[offset,self.r32(self.renderer+offset)] for offset in OFFSETS]}


def inputs():
    rng = random.Random(0x892B7A)
    out = []
    samples = [(-0.75,0.75,-0.5625,0.5625,0.125,512.0),
               (-0.4,0.9,-0.1,0.7,0.0625,1024.0),
               (-17.125,6.0625,4.25,-3.5,-2.25,0.5),
               (-0.0,1.0,0.0,-1.0,0.0,1.0),
               (1.0,-2.0,5.0,-7.0,5.0,-20.0)]
    for _ in range(35):
        left=rng.uniform(-10,0); right=rng.uniform(0.1,12)
        bottom=rng.uniform(-10,0); top=rng.uniform(0.1,9)
        near=rng.uniform(-0.1,2); far=rng.uniform(3,2000)
        samples.append((left,right,top,bottom,near,far))
    for index, bounds in enumerate(samples):
        for ortho in (False,True):
            for flag in (False,True):
                out.append({"name":f"bounds_{index}_ortho{int(ortho)}_flag{int(flag)}",
                            "frustum":[h.f2b(x) for x in bounds],"orthographic":ortho,"flag54":flag})
    # Explicit pre-store/service/load failures on every boundary of each branch.
    for base in copy.deepcopy(out[:4]):
        base["precision"]="Double53"
        trace=Machine(base,"original").execute()["trace"]
        for n in range(1,len(trace)+1):
            row=copy.deepcopy(base);row["name"]+=f"_stop{n}";row["stop"]=n;out.append(row)
        load_index=next(i+1 for i,event in enumerate(trace) if event==["load",0x708])
        row=copy.deepcopy(base);row["name"]+="_matrix_cache_reads"
        row["mutations"]=[{"event":load_index,"offset":0x708,"bits":h.f2b(3.25)},
                          {"event":load_index+1,"offset":0x708,"bits":h.f2b(7.125)},
                          {"event":load_index+2,"offset":0x718,"bits":h.f2b(-9.5)}]
        out.append(row)
    return out


def main():
    rows=[]
    for source in inputs():
        for pc in h.PRECISION:
            case=copy.deepcopy(source);case["precision"]=pc
            for image in ("original","v14"):
                rows.append({**case,"image":image,"output":Machine(case,image).execute()})
    for i in range(0,len(rows),2):
        assert rows[i]["output"]==rows[i+1]["output"]
    document={"schema":1,"scope":__doc__,"original_sha256":h.ORIGINAL_SHA,
              "v14_sha256":h.V14_SHA,"cases":rows,
              "instructions":[{"va":hex(a),"bytes":raw} for a,raw in sorted(h.VISITED.items())]}
    output=HERE/"native-projection.json"
    output.write_text(json.dumps(document,indent=1)+"\n")
    listing=[]
    for address,size in [(0x892970,0x34),(0x892B7A,0x1E4)]:
        listing.extend(f"{i.address:08x}: {i.bytes.hex()} {i.mnemonic} {i.op_str}"
                       for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(ORIGINAL.range(address,size),address))
    (HERE/"aa-projection-instructions.txt").write_text("\n".join(listing)+"\n")
    print(json.dumps({"rows":len(rows),"instructions":len(h.VISITED),
                      "sha256":hashlib.sha256(output.read_bytes()).hexdigest()}))


if __name__=="__main__":
    main()
