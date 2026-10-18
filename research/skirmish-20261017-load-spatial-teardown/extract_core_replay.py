"""Lift the exact frozen pure fixture test/snapshot for a small Rust link.

This is supplemental kernel-only proof, never a substitute for runtime binding
or actual registered-shell acceptance in the root's shared test cohort.
"""
from pathlib import Path
import hashlib, json
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
path = ROOT / "crates/aa-runtime/src/load_spatial_teardown_tests.rs"
source = path.read_text()

def function(name):
    start = source.index("fn " + name + "(")
    opening = source.index("{", start)
    level, end = 1, opening + 1
    while level:
        level += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]

native = function("original_v14_native_removal_and_unindexed_descriptor_skip")
snapshot = function("snapshot")
header = "use aa_numeric::Precision;\nuse aa_spatial::index::{Member,SpatialIndex};\nuse serde_json::{Value,json};\n"
#Preserve function text verbatim; include paths keep their original source
#location through this temporary child folder at the same relative depth.
outdir = HERE / "core-link"
outdir.mkdir(exist_ok=True)
out = outdir / "core_replay.rs"
text = header + "#[test]\n" + native + "\n\n" + snapshot + "\n"
#Original tests are one directory shallower than this file; use copied fixture
#layout at the precise relative include path without editing the lifted body.
out.write_text(text)
H = lambda b: hashlib.sha256(b).hexdigest()
(HERE / "core-extraction.json").write_text(json.dumps(dict(
    source=str(path.relative_to(ROOT)), source_sha256=H(path.read_bytes()),
    native_test_text_sha256=H(native.encode()), snapshot_text_sha256=H(snapshot.encode()),
    extraction_sha256=H(out.read_bytes()),
    scope="verbatim test/kernel snapshot only; not Runtime Source/provenance/Binding/GTID acceptance"
), indent=2)+"\n")
print(out)
