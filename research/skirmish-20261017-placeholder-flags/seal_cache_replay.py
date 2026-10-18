"""Retain final current-cache unit replay without changing historical evidence."""
from pathlib import Path
import json, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
H = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
old = HERE / "freeze-before-cache-replay.json"
doc = json.loads(old.read_text())
for pin in doc["pins"]:
    assert H(ROOT / pin["path"]) == pin["sha256"], pin["path"]
catalog = (ROOT / "crates/aa-runtime/src/catalog.rs").read_text()
methods = []
for method in ["verified_placeholder_flags", "verified_placeholder_flags_in_bank"]:
    start = catalog.index("    pub(crate) fn " + method + "(")
    opening = catalog.index("{", start)
    depth, end = 1, opening + 1
    while depth:
        depth += (catalog[end] == "{") - (catalog[end] == "}")
        end += 1
    methods.append(catalog[start:end])
snapshot = HERE / "source-snapshot/catalog-placeholder-methods.rs"
snapshot.write_text("\n\n".join(methods) + "\n")
binary = HERE / "frozen-cache-tests.exe"
log = HERE / "runtime-cache-unit-replay.log"
assert "2 passed; 0 failed" in log.read_text()
for p in [binary, log, snapshot]:
    doc["pins"].append(dict(path=str(p.relative_to(ROOT)).replace("\\", "/"), bytes=p.stat().st_size, sha256=H(p)))
doc["status"] = "frozen own production/external/native and actual Catalog cache/moved-Bank unit replay passed"
doc["historical_before_cache_replay_sha256"] = H(old)
doc["cache_unit_replay"] = {"passed": 2, "failed": 0, "seconds": 13.41,
    "binary": str(binary.relative_to(ROOT)).replace("\\", "/"), "binary_sha256": H(binary),
    "command": "AA_GAME_ROOT=<actual>; frozen-cache-tests.exe replacement_flags_source::tests --include-ignored --nocapture",
    "build_overrides": {"CARGO_PROFILE_RELEASE_LTO": "false", "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "16"},
    "scope": "same actual Bank/cache, warm foreign precision and active Bank loan refusal, unchanged native rows; same Bank moved without reinstall",
    "shared_source_scope": "only the two Catalog cache methods are snapshotted; shared files continue evolving separately"}
(HERE / "freeze.json").write_text(json.dumps(doc, indent=2) + "\n")
findings = json.loads((HERE / "findings.json").read_text())
findings["rust"]["runtime_unit_native_and_cache_move_tests"] = dict(passed=2, failed=0, seconds=13.41, binary_sha256=H(binary))
findings["freeze_sha256"] = H(HERE / "freeze.json")
findings["status"] = "bounded unchanged-flags source/cache checkpoint passed"
(HERE / "findings.json").write_text(json.dumps(findings, indent=2) + "\n")
print(json.dumps(dict(freeze_sha256=H(HERE / "freeze.json"), findings_sha256=H(HERE / "findings.json"), binary_sha256=H(binary))))
