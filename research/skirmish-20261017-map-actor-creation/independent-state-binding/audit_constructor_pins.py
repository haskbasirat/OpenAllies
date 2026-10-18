"""Read-only independent constructor fixture replay with seam pin classification."""
from pathlib import Path
import importlib.util
import json
import sys

HERE = Path(__file__).resolve().parent
AUTHOR = HERE.parent
spec = importlib.util.spec_from_file_location("independent_constructor", AUTHOR / "native_constructor.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
machine = module.ActorMachine
original = machine._hook
intercepted = set()

def hook(self, u, pc, size, data):
    if module.P.base <= pc < module.P.base + module.P.image_size and pc in self.seams:
        intercepted.add(pc)
    return original(self, u, pc, size, data)

machine._hook = hook
module.main()
pins = set(module.am.VISITED)
assert intercepted <= pins
report = {
    "fixture_address_byte_pins": len(pins),
    "executed_image_instruction_addresses": len(pins - intercepted),
    "intercepted_leaf_entry_addresses": [hex(p) for p in sorted(intercepted)],
    "intercepted_leaf_entries": len(intercepted),
    "scope": "same immutable constructor replay; classification accounts for inherited hook recording before seams",
}
print(json.dumps(report, indent=2))
if "--report" in sys.argv:
    (HERE / "constructor-pin-classification.json").write_text(json.dumps(report, indent=2) + "\n")
