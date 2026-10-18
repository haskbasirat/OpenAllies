"""Independent in-memory alpha metadata and complete prior-output comparison."""
from pathlib import Path
from collections import Counter
import hashlib
import importlib.util
import json
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TOOLS = ROOT.parent / "reconstruction/tools"
sys.path.insert(0, str(TOOLS))
import convert_models as current


def hash_bytes(raw):
    return hashlib.sha256(raw).hexdigest()


spec = importlib.util.spec_from_file_location("independent_pre_alpha_converter", HERE / "before-alpha/convert_models.py")
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
previous.ROOT, previous.ORIGINAL, previous.OUTPUT = current.ROOT, current.ORIGINAL, current.OUTPUT
proof_path = HERE.parent / "corpus/alpha-cross-check.json"
proof = json.loads(proof_path.read_text(encoding="utf-8"))
assert hash_bytes(proof_path.read_bytes()) == "4f25f07a4560bdd39dfecc1fccfa6afab3417c39e4939547b6711e449d8288d5"
counts = Counter()
outputs = []
new_reader, old_reader = current.Converter(), previous.Converter()
for case in proof["cases"]:
    path = current.ORIGINAL / case["path"]
    raw = path.read_bytes()
    assert hash_bytes(raw) == case["sha256"]
    data = current.NifFormat.Data()
    with path.open("rb") as stream:
        data.read(stream)
        assert stream.tell() == len(raw)
    for alpha in case["alpha_properties"]:
        prop = data.blocks[alpha["block"]]
        assert isinstance(prop, current.NifFormat.NiAlphaProperty)
        expected = {"flags": alpha["flags"], "threshold": alpha["threshold"]}
        assert {"flags": int(prop.flags), "threshold": int(prop.threshold)} == expected
        new_material, new_uv = new_reader.material(path, [prop])
        old_material, old_uv = old_reader.material(path, [prop])
        assert new_material.pop("alphaPropertyState") == expected
        assert new_material == old_material and new_uv == old_uv
        counts["exact_raw_alpha_properties"] += 1
    results = []
    for reader in (old_reader, new_reader):
        try:
            model = reader.convert(path, data=data, write=False)
        except ValueError as error:
            assert str(error) == "No visible triangle geometry"
            results.append(("no_visible_triangle_geometry", None))
        else:
            results.append(("converted_in_memory", model))
    assert results[0][0] == results[1][0]
    if results[0][1] is not None:
        stripped = json.loads(json.dumps(results[1][1]))
        for mesh in stripped["meshes"]:
            state = mesh["material"].pop("alphaPropertyState", None)
            if state is not None:
                assert state in [{"flags": p["flags"], "threshold": p["threshold"]} for p in case["alpha_properties"]]
                counts["inherited_alpha_meshes_checked"] += 1
        assert stripped == results[0][1]
    assert path.read_bytes() == raw
    counts[results[0][0]] += 1
    outputs.append({"path": case["path"], "source_sha256": case["sha256"], "old_new_status": results[0][0], "prior_output_identical_after_removing_new_metadata": True})
empty_new, empty_uv = new_reader.material(current.ORIGINAL / proof["cases"][0]["path"], [])
empty_old, old_uv = old_reader.material(current.ORIGINAL / proof["cases"][0]["path"], [])
assert empty_new == empty_old and empty_uv == old_uv and "alphaPropertyState" not in empty_new
assert counts["exact_raw_alpha_properties"] == 95 and len(outputs) == 15
report = {
    "schema": "byroredux_independent_alpha_retention_replay_v1",
    "proof_sha256": hash_bytes(proof_path.read_bytes()),
    "current_converter_sha256": hash_bytes((TOOLS / "convert_models.py").read_bytes()),
    "previous_converter_sha256": hash_bytes((HERE / "before-alpha/convert_models.py").read_bytes()),
    "models": 15,
    "counts": dict(counts),
    "records": outputs,
    "absent_alpha_material_unchanged": True,
    "asset_writes": False,
    "source_bytes_rechecked_unchanged": True,
    "scope": "Actual original NIF data through existing converter in memory; new raw metadata retained, every previous emitted field identical. No game renderer/visual claim.",
}
path = HERE / "alpha-retention-replay.json"
with path.open("x", encoding="utf-8", newline="\n") as out:
    json.dump(report, out, indent=2, ensure_ascii=True)
    out.write("\n")
print(json.dumps({"models": 15, "counts": dict(counts), "sha256": hash_bytes(path.read_bytes())}))
