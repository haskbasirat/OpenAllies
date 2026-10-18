"""Read-only Berlin clock-owner probe; writes only this new research directory.

Actual 4EC4A0 runs against explicit laboratory current World words. The actual
class9 name virtual is a REQUIRED STOP, not a successful replacement. No Berlin
runtime time value, whole startup, or clock lifetime is inferred from these rows.
"""
from pathlib import Path
import hashlib
import json
import sys
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / "research/skirmish-20261016-store-memory-refresh"))
from native_memory_refresh import Machine, h
from capstone import Cs, CS_ARCH_X86, CS_MODE_32


def sha(data):
    return hashlib.sha256(data).hexdigest()


def create_json(name, value):
    with (HERE / name).open("x", encoding="utf-8", newline="\n") as out:
        json.dump(value, out, indent=2)
        out.write("\n")


def main():
    words = [0, 0x80000000, 0x3F800001, 0x40500000, 0x42260000, 0x42288000, 0x4B000001]
    rows = []
    for pc in h.CW:
        for current in words:
            values = []
            for image in (h.P, h.V):
                case = dict(
                    **{"class": 9},
                    time=0x3F800000,
                    flags=0x20000000,
                    position=[0x3FC00000, 0xC0200000],
                    variant=17,
                    owner=0,
                    display="Pine",
                    key="",
                    text="",
                    kingdom="",
                    area="",
                    camera="",
                    stop="name",
                    data_variations=False,
                )
                machine = Machine(pc, image, case)
                # Deliberately differ from constructor input, tick/end/step.
                # Timestamp must be the current E0 word at execution, unchanged.
                for offset, value in [(0xDC, 17), (0xE0, current), (0xE4, 0x42990000), (0xE8, 0x3D800000)]:
                    machine.w(machine.world + offset, value)
                result = machine.run()
                assert result["outcome"] == "name"
                assert result["time"] == current
                assert result["name"] == "old"
                assert result["position"] == [0, 0]
                assert result["definition_raw"] == 0xA5A5A5A5
                values.append(result)
            assert values[0] == values[1]
            rows.append({"pc": pc, "current_world_e0": current, "native": values[0]})
    required_name = 0x4EF270
    pins = [
        {"va": hex(a), "bytes": raw, "kind": "required_name_entry" if a == required_name else "executed"}
        for a, raw in sorted(h.VISITED.items())
    ]
    ranges = [
        ("refresh_first_store", 0x4EC4A0, 0x4EC4D6),
        ("record_create_prefix", 0x4ECA80, 0x4ECB8C),
        ("saved_world_clock_read", 0x4FC6C6, 0x4FC71C),
        ("fresh_world_clock_init", 0x4F27E0, 0x4F28A6),
        ("world_step_clock_update", 0x4F9970, 0x4F9A3B),
    ]
    disassembler = Cs(CS_ARCH_X86, CS_MODE_32)
    disassembly = []
    for label, start, end in ranges:
        original = h.P.range(start, end-start)
        assert h.V.range(start, end-start) == original
        disassembly.append({
            "label": label, "start": hex(start), "end_exclusive": hex(end),
            "both_pe_bytes": original.hex(),
            "instructions": [
                {"va": hex(i.address), "bytes": bytes(i.bytes).hex(), "mnemonic": i.mnemonic, "operands": i.op_str}
                for i in disassembler.disasm(original, start)
            ],
        })
    target = "native-clock-read-probe-v2.json"
    create_json(target, {
        "schema": 1, "utc": datetime.now(timezone.utc).isoformat(),
        "supersedes": "v1 numerical results unchanged; corrected two static disassembly start addresses to actual function boundaries, and trimmed ranges to whole instructions",
        "scope": __doc__, "producer_sha256": sha(Path(__file__).read_bytes()),
        "original_sha256": sha(h.P.b), "v14_sha256": sha(h.V.b),
        "rows": rows, "machines": len(rows)*2, "pins": pins,
        "disassembly": disassembly,
        "limits": [
            "Explicit lab World/Actor/Record allocations and current words; not observed Berlin values.",
            "Actual refresh executes only through its first name virtual entry, which stops.",
            "Static disassembly ranges are byte-equal in both images; this probe does not execute whole load/init/step.",
            "Clock lifetime/caller diagnosis is separate source audit; no Rust build or runtime execution.",
        ],
    })
    print(json.dumps({"passed": True, "paired_rows": len(rows), "machines": len(rows)*2,
        "pins": len(pins), "executed_pins": sum(p["kind"] == "executed" for p in pins),
        "fixture_sha256": sha((HERE / target).read_bytes())}))


if __name__ == "__main__":
    main()
