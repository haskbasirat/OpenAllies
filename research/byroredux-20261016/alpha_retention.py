"""Validate raw-alpha retention in memory; never write converted/game assets."""
from pathlib import Path
import hashlib
import importlib.util
import json
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
GAME = HERE.parents[1].parent
TOOLS = GAME / "reconstruction/tools"
sys.path.insert(0, str(TOOLS))
import convert_models as current


def load_before():
    spec = importlib.util.spec_from_file_location("before_alpha_retention", HERE / "production-backups/converter.py")
    old = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(old)
    old.ROOT, old.ORIGINAL, old.OUTPUT = current.ROOT, current.ORIGINAL, current.OUTPUT
    return old


def main():
    proof_path = HERE / "corpus/alpha-cross-check.json"
    proof = json.loads(proof_path.read_text())
    before = load_before()
    converter, legacy = current.Converter(), before.Converter()
    materials, outputs = [], []
    for case in proof["cases"]:
        path = current.ORIGINAL / case["path"]
        source_bytes = path.read_bytes()
        assert hashlib.sha256(source_bytes).hexdigest() == case["sha256"]
        data = current.NifFormat.Data()
        with path.open("rb") as stream:
            data.read(stream)
            assert stream.tell() == len(source_bytes)
        for alpha in case["alpha_properties"]:
            prop = data.blocks[alpha["block"]]
            assert isinstance(prop, current.NifFormat.NiAlphaProperty)
            expected = {"flags": alpha["flags"], "threshold": alpha["threshold"]}
            assert {"flags": int(prop.flags), "threshold": int(prop.threshold)} == expected
            material, _ = converter.material(path, [prop])
            old_material, _ = legacy.material(path, [prop])
            assert material["alphaPropertyState"] == expected
            stripped = dict(material)
            del stripped["alphaPropertyState"]
            assert stripped == old_material
            materials.append(dict(source=case["path"], source_sha256=case["sha256"],
                                  block=alpha["block"], material=material,
                                  legacy_material=old_material, expected=expected))
        try:
            result = converter.convert(path, data=data, write=False)
            old_result = legacy.convert(path, data=data, write=False)
        except ValueError as error:
            assert str(error) == "No visible triangle geometry"
            outputs.append(dict(source=case["path"], status="no_visible_triangle_geometry"))
        else:
            alpha_meshes = 0
            for mesh in result["meshes"]:
                if "alphaPropertyState" in mesh["material"]:
                    alpha_meshes += 1
                    state = mesh["material"]["alphaPropertyState"]
                    assert state in [{"flags": p["flags"], "threshold": p["threshold"]}
                                     for p in case["alpha_properties"]]
            stripped = json.loads(json.dumps(result))
            for mesh in stripped["meshes"]:
                mesh["material"].pop("alphaPropertyState", None)
            assert stripped == old_result
            outputs.append(dict(source=case["path"], status="converted_in_memory",
                                mesh_count=len(result["meshes"]), alpha_mesh_count=alpha_meshes,
                                all_previous_output_fields_identical=True))
        assert hashlib.sha256(path.read_bytes()).hexdigest() == case["sha256"]
    absent, _ = converter.material(current.ORIGINAL / proof["cases"][0]["path"], [])
    legacy_absent, _ = legacy.material(current.ORIGINAL / proof["cases"][0]["path"], [])
    assert absent == legacy_absent and "alphaPropertyState" not in absent
    out = dict(schema=1, upstream_commit=proof["upstream_commit"],
               alpha_cross_check_sha256=hashlib.sha256(proof_path.read_bytes()).hexdigest(),
               scope="Byro-validated authored alpha properties through current converter, old/new comparison in memory only",
               property_cases=len(materials), cases=materials, absent_material=absent,
               conversion_outputs=outputs, assets_written=False, original_inputs_unchanged=True)
    (HERE / "alpha-retention-fixtures.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(dict(property_cases=len(materials), models=len(outputs),
                          converted=sum(r["status"] == "converted_in_memory" for r in outputs),
                          unchanged=True, assets_written=False)))


if __name__ == "__main__":
    main()
