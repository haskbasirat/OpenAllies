"""Compile exact frozen model_fixes.rs as its own test crate, no Cargo changes."""
from pathlib import Path
import hashlib
import json
import os
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
source = ROOT / "crates/aa-client/src/model_fixes.rs"
assert hashlib.sha256(source.read_bytes()).hexdigest() == "7271a9c718112fd70f4cb234665cf0cf30317c85b49a1ddc554ab150e89fceb4"
own = ROOT / "target/s-byroredux-review/release/deps"
author = ROOT / "target/s-graphics-reference/release/deps"
libs = {}
for name, directory in (("aa_assets", own), ("serde", own), ("serde_json", own), ("anyhow", author), ("sha2", author)):
    candidates = list(directory.glob(f"lib{name}-*.rlib"))
    assert len(candidates) == 1, (name, candidates)
    libs[name] = candidates[0]
output = HERE / "model-fixes-alpha-guard-contract.exe"
assert not output.exists()
command = ["rustc", "--test", "--edition=2024", "--crate-name=review_alpha_guards", str(source), "-C", "opt-level=1", "-L", f"dependency={own}", "-L", f"dependency={author}", "-o", str(output)]
for name, path in libs.items():
    command += ["--extern", f"{name}={path}"]
environment = dict(os.environ)
environment["CARGO_MANIFEST_DIR"] = str(ROOT / "crates/aa-client")
completed = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True, encoding="utf-8", timeout=120)
with (HERE / "alpha-guard-contract-build.log").open("x", encoding="utf-8", newline="\n") as stream:
    stream.write(completed.stdout + completed.stderr)
assert completed.returncode == 0, completed.stderr
report = {"scope": "Exact frozen production model_fixes.rs compiled directly as a test crate against reviewed existing dependencies; this is a source contract test, not full aa-client linking or runtime/graphics proof.", "command": command, "returncode": completed.returncode, "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "exe": str(output), "exe_sha256": hashlib.sha256(output.read_bytes()).hexdigest(), "libs": [{"name": name, "path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()} for name, path in libs.items()]}
with (HERE / "alpha-guard-contract-build.json").open("x", encoding="utf-8", newline="\n") as stream:
    json.dump(report, stream, indent=2)
    stream.write("\n")
print(json.dumps({"returncode": completed.returncode, "exe": str(output), "exe_sha256": report["exe_sha256"]}))
