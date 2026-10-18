"""Read-only PE lifecycle evidence and original 7A0C20 inactive-branch oracle.

This script never executes the lifecycle teardown/activation calls. It exports
their instruction windows, and separately executes only 7A0C20 with observer
byte +20 zero. There are no call/allocator/game-state seams in that execution.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / "reconstruction/tools/python-packages"))
import capstone
import unicorn
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDI, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW

sha = lambda b: hashlib.sha256(b).hexdigest()


class PE:
    def __init__(self, path, expected_sha):
        self.path = path
        self.raw = path.read_bytes()
        assert sha(self.raw) == expected_sha, path
        pe = struct.unpack_from("<I", self.raw, 0x3c)[0]
        count = struct.unpack_from("<H", self.raw, pe + 6)[0]
        opt = pe + 24
        self.base = struct.unpack_from("<I", self.raw, opt + 28)[0]
        self.image_size = struct.unpack_from("<I", self.raw, opt + 56)[0]
        section_table = opt + struct.unpack_from("<H", self.raw, pe + 20)[0]
        self.sections = [struct.unpack_from("<4I", self.raw, section_table + i * 40 + 8) for i in range(count)]

    def bytes(self, va, size):
        for virtual_size, rva, raw_size, offset in self.sections:
            if rva <= va - self.base and va - self.base + size <= rva + raw_size:
                start = offset + va - self.base - rva
                return self.raw[start:start + size]
        raise ValueError((hex(va), size))


SOURCES = {
    "original": PE(GAME / "AA.exe", "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26"),
    "v14": PE(GAME / "reconstruction/native-game/runtime-expanded-v14/AA.exe", "3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1"),
}
# Selected linear windows, not claims that Ghidra function bodyBytes define
# contiguous bounds. In particular Session::OnGWorldReadyForState has EH gaps.
WINDOWS = [
    ("load_game_task_constructor", 0x59E6F0, 0x40),
    ("load_game_task_run", 0x5A4280, 0xE0),
    ("load_for_editor", 0x5A3E20, 0x90),
    ("load_normal", 0x5A3EB0, 0x70),
    ("load_alternate", 0x5A3F20, 0x70),
    ("map_resource_load", 0x5A2B90, 0x59F),
    ("sim_destroy_gworld", 0x76AB80, 0x9E),
    ("gworld_uninit", 0x4F8690, 0x43B),
    ("gmap_destructor", 0x555030, 0xC7),
    ("sai_map_deleting_destructor", 0x7ACF30, 0x1C),
    ("sai_map_destructor", 0x7ACE50, 0xD9),
    ("sai_map_clear", 0x7AC9A0, 0x157),
    ("sai_deactivate", 0x79FF10, 0xBA),
    ("sai_map_dynamic_analysis", 0x7ABC30, 0x148),
    ("sai_activate", 0x7A0D40, 0x234),
    ("sai_constructor", 0x7A0AA0, 0x151),
    ("sai_observe_actor", 0x7A0C20, 0x117),
    ("session_check_finished_task", 0x775DB0, 0x12D),
    ("session_world_ready", 0x7757F0, 0x400),
    ("load_game_task_complete", 0x5A442D, 0x77),
    ("task_set_state", 0x5B18C0, 0x50),
    ("element_attach", 0x5705A0, 0x3C),
]


def static_evidence():
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    result = []
    lines = ["Static disassembly only. Native execution proof is separately in inactive-oracle.json."]
    for name, va, size in WINDOWS:
        raw = SOURCES["original"].bytes(va, size)
        other = SOURCES["v14"].bytes(va, size)
        assert raw == other, name
        instructions = [dict(address=f"{i.address:08x}", bytes=i.bytes.hex(), mnemonic=i.mnemonic, operands=i.op_str) for i in md.disasm(raw, va)]
        result.append(dict(name=name, address=f"{va:08x}", size=size, sha256=sha(raw), original_v14_identical=True, instructions=instructions))
        lines.append(f"\n{name} {va:08x} size={size} sha256={sha(raw)} original=v14")
        lines.extend(f"{i['address']}: {i['bytes']:<22} {i['mnemonic']} {i['operands']}" for i in instructions)
    tables = []
    for name, va, count in [("LoadGameTask", 0xA43794, 8), ("SAIMap", 0xA9D3E4, 1), ("SAI", 0xA9C644, 1)]:
        raw = SOURCES["original"].bytes(va, count * 4)
        assert raw == SOURCES["v14"].bytes(va, len(raw))
        entries = [f"{v:08x}" for v in struct.unpack("<" + "I" * count, raw)]
        tables.append(dict(name=name, address=f"{va:08x}", sha256=sha(raw), entries=entries, original_v14_identical=True))
    activation_references = {}
    for name, pe in SOURCES.items():
        direct = []
        for virtual_size, rva, raw_size, offset in pe.sections:
            raw = pe.raw[offset:offset + raw_size]
            for i in range(len(raw) - 4):
                if raw[i] == 0xE8 and pe.base + rva + i + 5 + struct.unpack_from("<i",raw,i+1)[0] == 0x7A0D40:
                    direct.append(f"{pe.base+rva+i:08x}")
        assert direct == ["00775b63", "00775bc3"]
        # This conservative byte scan can find data as well as code; both
        # matches are independently decoded in the world-ready window above.
        activation_references[name] = dict(relative_call_byte_candidates=direct, absolute_pointer_bytes_present=struct.pack("<I",0x7A0D40) in pe.raw, limit="No claim to exclude dynamically computed indirect calls outside the audited managed load lifecycle.")
    (HERE / "lifecycle-instructions.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    return dict(sources={name:dict(path=str(p.path), sha256=sha(p.raw)) for name,p in SOURCES.items()}, windows=result, vtables=tables, activation_references=activation_references)


STACK_BASE, STACK_SIZE, STACK = 0x1E000000, 0x10000, 0x1E008000
STOP, OBSERVER, ACTOR = 0x1E00F000, 0x20000000, 0x21000000


def inactive_case(source, precision, actor):
    pe = SOURCES[source]
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    u.mem_map(pe.base, (pe.image_size + 0xFFF) & ~0xFFF)
    for virtual_size, rva, size, offset in pe.sections:
        if size:
            u.mem_write(pe.base + rva, pe.raw[offset:offset + size])
    # Page zero is NOT mapped: null actor accesses would fault. No FS/SEH
    # instructions run in this narrow function branch.
    for address,size in [(STACK_BASE,STACK_SIZE),(OBSERVER,0x1000),(ACTOR,0x1000)]:
        u.mem_map(address,size)
    u.mem_write(OBSERVER, bytes([0xA5]) * 0x1000)
    u.mem_write(OBSERVER + 0x20, b"\0")
    u.mem_write(ACTOR, bytes(range(256)) * 16)
    actor_before = bytes(u.mem_read(ACTOR,0x1000))
    observer_before = bytes(u.mem_read(OBSERVER,0x1000))
    u.mem_write(STACK, struct.pack("<II", STOP, actor))
    u.reg_write(UC_X86_REG_ESP,STACK)
    u.reg_write(UC_X86_REG_ECX,OBSERVER)
    u.reg_write(UC_X86_REG_EDI,0xAABBCCDD)
    u.reg_write(UC_X86_REG_EAX,0x11223344)
    u.reg_write(UC_X86_REG_FPCW,precision)
    trace,reads,writes = [],[],[]
    def on_code(machine,address,size,_):
        raw=bytes(machine.mem_read(address,size))
        assert raw == SOURCES["original"].bytes(address,size) == SOURCES["v14"].bytes(address,size)
        trace.append(dict(address=f"{address:08x}",bytes=raw.hex()))
    def on_read(machine,access,address,size,value,_):
        reads.append(dict(pc=f"{machine.reg_read(UC_X86_REG_EIP):08x}",address=f"{address:08x}",size=size))
    def on_write(machine,access,address,size,value,_):
        writes.append(dict(pc=f"{machine.reg_read(UC_X86_REG_EIP):08x}",address=f"{address:08x}",size=size,value=value))
    u.hook_add(UC_HOOK_CODE,on_code)
    u.hook_add(UC_HOOK_MEM_READ,on_read)
    u.hook_add(UC_HOOK_MEM_WRITE,on_write)
    u.emu_start(0x7A0C20,STOP,count=50)
    assert u.reg_read(UC_X86_REG_EIP)==STOP
    assert u.reg_read(UC_X86_REG_ESP)==STACK+8
    assert u.reg_read(UC_X86_REG_EDI)==0xAABBCCDD
    assert bytes(u.mem_read(ACTOR,0x1000))==actor_before
    assert bytes(u.mem_read(OBSERVER,0x1000))==observer_before
    expected_pcs=[0x7A0C20,0x7A0C21,0x7A0C23,0x7A0C26,0x7A0C28,0x7A0D33,0x7A0D34]
    assert [int(x['address'],16) for x in trace]==expected_pcs
    assert [(int(x['address'],16),x['size']) for x in reads]==[(OBSERVER+0x20,1),(STACK-4,4),(STACK,4)]
    assert [(int(x['address'],16),x['size']) for x in writes]==[(STACK-4,4)]
    return dict(source=source,fpu_control_word=f"{precision:04x}",actor_pointer=f"{actor:08x}",observer_active_byte=0,returned=True,actor_reads=0,observer_unchanged=True,mapped_actor_unchanged=True,stack_delta=8,edi_preserved=True,trace=trace,memory_reads=reads,memory_writes=writes)


def main():
    evidence = static_evidence()
    (HERE / "lifecycle-static.json").write_text(json.dumps(evidence,indent=2)+"\n",encoding="utf-8")
    actors=[0,1,0xDEADBEEF,0xFFFFFFFF,ACTOR,ACTOR+0x80,ACTOR+0xFFF]
    rows=[inactive_case(source,precision,actor) for source in SOURCES for precision in (0x007F,0x027F,0x037F) for actor in actors]
    report=dict(scope="Executed only original 7A0C20 inactive observer branch; lifecycle static audit is separate.",unicorn_version=unicorn.__version__,capstone_version=capstone.__version__,sources=evidence['sources'],seams=[],page_zero_mapped=False,cases=rows,case_count=len(rows),passed=len(rows),limitations=["No active SAI classification or teardown/activation code executed.","No full load task, loaded map, or game session executed.","No Rust tests/builds run by this evidence producer."])
    (HERE / "inactive-oracle.json").write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
    findings = dict(
        date="2026-10-16",
        scope="Element attachment observer during the native managed LoadGame lifecycle",
        sources=evidence['sources'],
        static_proof=dict(
            load_task="5A4280 calls 76AB80 at 5A4307 before all three loading branches: 5A4323 -> 5A3E20, 5A432F -> 5A3F20, 5A4336 -> 5A3EB0.",
            load_chain="5A3E8E / 5A3F3B / 5A3ECB call 5A2B90; 5A3031 calls 4FC600.",
            teardown_chain="76ABE5 -> 4F8690; 4F89E8 -> 555030; 55506B calls GMap+4C vtable A9D3E4[0]=7ACF30; 7ACF33 -> 7ACE50; 7ACE7C -> 7AC9A0; 7AC9D7 -> 79FF10 when old SAIMap+5 != 0; 79FFC3 clears SAI+20.",
            activation_invariant="7A0D7F calls 7ABC30, which sets SAIMap+5 at 7ABC53, before 7A0E25 sets SAI+20=1.",
            completion="5A435B jumps to 5A442D after successful load; 5A445F -> 5B18C0 publishes task+14 state 2/3; 775E59 requires 2 before 775E76 -> 7757F0; 775B63 / 775BC3 call 7A0D40 for the supported session states.",
            windows=len(evidence['windows']),
            windows_identical_in_original_and_v14=True,
        ),
        executed_proof=dict(function="007a0c20",observer_active_byte=0,case_count=len(rows),passed=len(rows),actor_reads=0,seams=[],exact_instruction_count=7,precisions=["PC24","PC53","PC64"],source_variants=["original","v14"],actor_pointer_cases=[f"{actor:08x}" for actor in actors],memory_contract="Only observer+20 and stack reads; push EDI is the only write; actor and observer pages unchanged."),
        integration_requirement="Use an explicit inactive observer loading context justified by managed task teardown. Do not silently suppress an active observer; do not bypass earlier Element attachment refresh operations.",
        limitations=report['limitations'] + ["Static lifecycle audit does not execute the complete lifecycle.","Arbitrary direct GWorld::Load over an active world is outside this proof.","Complete SAI behavior is not implemented or proven."],
        artifacts={name:dict(bytes=(HERE/name).stat().st_size,sha256=sha((HERE/name).read_bytes())) for name in ["native_observer_lifecycle.py","README.txt","lifecycle-static.json","lifecycle-instructions.txt","inactive-oracle.json"]},
    )
    (HERE / "findings.json").write_text(json.dumps(findings,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(dict(static_windows=len(evidence['windows']),vtables=len(evidence['vtables']),inactive_cases_passed=len(rows))))


if __name__ == "__main__":
    main()
