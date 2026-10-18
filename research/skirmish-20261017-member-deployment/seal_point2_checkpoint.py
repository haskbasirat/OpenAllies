"""Seal the bounded point2 successor without changing predecessor evidence."""
from pathlib import Path
import hashlib, json, re, shutil

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source_paths = [
    "crates/aa-rules/src/native.rs",
    "crates/aa-rules/src/resolved.rs",
    "crates/aa-rules/tests/native_point2_decimal_prefix.rs",
]
sources = []
for name in source_paths:
    original = ROOT / name
    snapshot = HERE / "point2-source-checkpoint" / name
    snapshot.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(original, snapshot)
    sources.append({"path": name, "snapshot": str(snapshot.relative_to(HERE)), "sha256": sha(snapshot)})

native = json.loads((HERE / "native-point2-decimal-prefix.json").read_text())
assert len(native["rows"]) == 63 and len(native["instructions"]) == 5262
assert sha(HERE / "native-point2-decimal-prefix.json") == "c2def62a0ab530d1b5fba99eeb85e292234c25f147ab78cca6698a062d93085c"
census = json.loads((HERE / "actual-source-census-point2.json").read_text())
for pc in census["precisions"]:
    assert pc["present"] == 40 and pc["absent"] == 988
    assert all(row["error"] is None for row in pc["rows"])
    usa = next(row for row in pc["rows"] if row["id"] == "usa_hq_infantry")
    assert usa["words"][:2] == [0x40200000, 0x3fc00000]

regressions = []
for name in ["point2-aa-rules-full.log", "point2-aa-metagame-full.log"]:
    log = (HERE / name).read_text(encoding="utf-8-sig")
    results = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored", log)
    assert results and all(failed == "0" and ignored == "0" for _, failed, ignored in results), name
    assert "test result: FAILED" not in log
    regressions.append({"log": name, "suites": len(results), "passed": sum(int(v[0]) for v in results), "failed": 0, "ignored": 0, "sha256": sha(HERE / name)})

binary = sorted((ROOT / "target/s-owner-recruitment/release/deps").glob("native_point2_decimal_prefix-*.exe"), key=lambda p: p.stat().st_mtime)[-1]
shutil.copyfile(binary, HERE / "frozen-point2-tests.exe")
files = [
    "native_point2_decimal_prefix.py", "native-point2-decimal-prefix.json",
    "native-point2-write.log", "native-point2-immutable.log",
    "actual-source-census-point2.rs", "actual-source-census-point2.exe",
    "actual-source-census-point2.json", "actual-source-census-point2.log",
    "frozen-point2-tests.exe", "point2-aa-rules-full.log", "point2-aa-metagame-full.log",
    "POINT2-README.md", "seal_point2_checkpoint.py",
]
report = {
    "schema": 1,
    "status": "author bounded native/source checkpoint; independent review pending",
    "scope": "Point2-only finite ASCII digits/dots decimal-prefix subset of whole5BD080/5BB7F0/GetPoint25B4D80; scalar and point3 remain strict",
    "native": {"rows": 63, "original_v14_executions": 126, "executed_address_byte_pins": 5262, "immutable_replay": "PASS", "actual_source_line": native["actual_source_line"], "actual_source_sha256": native["actual_source_sha256"]},
    "rust": {"tests": "3/3 including real USA HQ source; part of full aa-rules regression", "source": sources, "required_regressions": regressions, "new_runtime_or_client_link": False},
    "actual_source_census": {"canonical_actor_definitions": 1028, "precisions": "three explicit lab PCs; no runtime/startup authority", "deployment_present": 40, "deployment_absent": 988, "typed_errors": 0, "usa_creation_words": [0x40200000, 0x3fc00000]},
    "unchanged_predecessor": "findings.json and actual-source-census.json retain the earlier strict-parser refusal; this successor does not overwrite them",
    "not_claimed": [
        "General CRT string-to-number equivalence or arbitrary ignored suffixes",
        "New tolerance in scalar fields or point3",
        "Native accepted no-digit prefix '..5' or repeated-dot suffix f/% (still explicit errors)",
        "Source expression/reference evaluation or nonfinite arithmetic",
        "Native malformed-pointer/allocator/Win32/unwind equivalence",
        "Natural playable mission deployment, initialized Staged World or actual positive4F9C10/Mind198 child success",
    ],
    "files": {name: sha(HERE / name) for name in files},
}
out = HERE / "point2-findings.json"
out.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({"report_sha256": sha(out), "regressions": regressions, "binary_sha256": sha(HERE / "frozen-point2-tests.exe")}))
