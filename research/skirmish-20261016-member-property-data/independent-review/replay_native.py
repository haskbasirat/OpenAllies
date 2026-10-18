"""Fresh reviewer replay of the frozen author oracle; no new oracle semantics."""
from pathlib import Path
import hashlib
import importlib.util
import json

HERE = Path(__file__).resolve().parent
AUTHOR = HERE.parent
script = AUTHOR / "native_member_property_data.py"
spec = importlib.util.spec_from_file_location("reviewed_member_property_oracle", script)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
rows = []
for case in m.cases():
    for image in ["original", "v14"]:
        for precision in m.h.PRECISION:
            rows.append({"image": image, "precision": precision, "input": case,
                         "output": m.Machine(case, image, precision).run()})
doc = {"schema": 1, "scope": m.__doc__,
       "original_sha256": m.h.ORIGINAL_SHA, "v14_sha256": m.h.V14_SHA,
       "producer_sha256": hashlib.sha256(script.read_bytes()).hexdigest(),
       "instructions": [{"va": hex(a), "bytes": b} for a, b in sorted(m.h.VISITED.items())],
       "cases": rows}
expected = json.loads((AUTHOR / "native-member-property-data.json").read_text())
assert doc == expected
out = HERE / "native-replay.json"
with out.open("x", encoding="utf-8", newline="\n") as f:
    f.write(json.dumps(doc, indent=2) + "\n")
# The inherited hook records entry bytes before a seam intercepts execution.
# These are pins of intercepted entry addresses, not executed native bodies.
leaves = {0x5b4220, 0x856370, 0x42be10, 0x5b54d0,
          0x405b80, 0x463fa0, 0x6f7810, 0x410c40}
entries = [hex(a) for a in sorted(m.h.VISITED) if a in leaves]
summary = {"fresh_rows": len(rows), "exact_author_fixture_match": True,
           "visited_image_address_pins": len(m.h.VISITED),
           "intercepted_image_entry_pins": entries,
           "executed_image_instruction_addresses": len(m.h.VISITED) - len(entries),
           "source_sha256": doc["producer_sha256"],
           "fresh_fixture_sha256": hashlib.sha256(out.read_bytes()).hexdigest(),
           "scope": "Fresh replay of the frozen author harness, not an independently constructed oracle. Native list/cursor instructions execute; typed source, CString helpers, property lookup, throwing and successful allocation are explicit seams."}
with (HERE / "native-replay-summary.json").open("x", encoding="utf-8", newline="\n") as f:
    f.write(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary))
