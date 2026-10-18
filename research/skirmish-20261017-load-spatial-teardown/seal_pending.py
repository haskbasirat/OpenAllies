"""Pin stable sources and honest pending shared-cohort validation."""
from pathlib import Path
import hashlib, json
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
H = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
files = [ROOT/"crates/aa-runtime/src/load_spatial.rs", ROOT/"crates/aa-runtime/src/load_spatial_teardown_tests.rs",
    HERE/"native_teardown.py", HERE/"native-teardown.json", HERE/"native-immutable.log", HERE/"check.log", HERE/"README.txt"]
snap = HERE/"source-frozen"
snap.mkdir(exist_ok=True)
pins = []
for path in files:
    row = dict(path=str(path.relative_to(ROOT)).replace("\\","/"), sha256=H(path))
    if path.suffix == ".rs":
        target = snap/path.name
        target.write_bytes(path.read_bytes())
        row["snapshot"] = str(target.relative_to(ROOT)).replace("\\","/")
    pins.append(row)
doc = dict(schema=1, status="source_native_frozen_runtime_root_cohort_pending", pins=pins,
    native=dict(rows=18,runs=36,address_byte_pins=291,executed=289,intercepted=2,global_retirement_executed=False),
    production_check=dict(passed=True,seconds=50.08),
    rust_tests=dict(status="pending",filters=["load_spatial::teardown_tests"],include_ignored=True,actual_game_root_required=True),
    interrupted_own_build="rust-tests.log deliberately stopped per root request to avoid duplicate runtime links; no passing Rust test claim",
    scope="spatial prefix only, same actual registered shell/index and sourcePC; current-only rescan boundary explicit")
(HERE/"freeze.json").write_text(json.dumps(doc,indent=2)+"\n")
print(json.dumps(dict(freeze_sha256=H(HERE/"freeze.json"),source_sha256=H(files[0]),test_sha256=H(files[1]))))
