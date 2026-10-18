"""Recheck three original NIFs without writing assets or converted models.

Uses the existing local PyFFI converter, never downloaded public code or SDK
binaries. Output must be a new JSON file in this evidence directory.
"""
from pathlib import Path
import argparse
import hashlib
import json
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT.parent / "reconstruction/tools"))
from convert_models import Converter, NifFormat, ORIGINAL


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default="material-retention-recheck.json")
    args = parser.parse_args()
    output = (HERE / args.output).resolve()
    if output.parent != HERE or output.exists():
        raise ValueError("Output must be a new file directly in this evidence directory")
    before = json.loads((HERE / "material-retention-before.json").read_text())
    converter = Converter()
    material_method = converter.material
    captured = []

    def capture(model, properties):
        properties = list(properties)
        expected = None
        for prop in properties:
            if isinstance(prop, NifFormat.NiTexturingProperty) and prop.has_base_texture:
                expected = {
                    "clampMode": int(prop.base_texture.clamp_mode),
                    "filterMode": int(prop.base_texture.filter_mode),
                    "uvSet": int(prop.base_texture.uv_set),
                }
        captured.append(expected)
        return material_method(model, properties)

    converter.material = capture
    rows = []
    for prior in before:
        source = ORIGINAL / prior["source"]
        raw = source.read_bytes()
        assert hashlib.sha256(raw).hexdigest() == prior["source_sha256"]
        data = NifFormat.Data()
        with source.open("rb") as stream:
            data.read(stream)
        captured.clear()
        converted = converter.convert(source, data=data, write=False)
        assert len(captured) == len(converted["meshes"])
        for index, (mesh, expected) in enumerate(zip(converted["meshes"], captured)):
            assert mesh["material"].get("baseTextureState") == expected
            legacy = dict(mesh["material"])
            legacy.pop("baseTextureState", None)
            assert legacy == prior["converted_materials_before"][index]["material"]
            rows.append({"source": prior["source"], "mesh_index": index,
                         "expected": expected, "legacy_material_unchanged": True})
        assert source.read_bytes() == raw
    descriptor = NifFormat.NiTexturingProperty()
    descriptor.has_base_texture = True
    descriptor.base_texture.source = None
    descriptor.base_texture.clamp_mode = 1
    descriptor.base_texture.filter_mode = 5
    descriptor.base_texture.uv_set = 7
    material, preview_uv = converter.material(ORIGINAL / "descriptor-only.nif", [descriptor])
    assert material["baseTextureState"] == {"clampMode": 1, "filterMode": 5, "uvSet": 7}
    assert "texture" not in material and preview_uv == 0
    result = {"original_source_files": len(before), "material_rows": len(rows),
              "checks": rows, "descriptor_without_source_retained": True,
              "native_rendering_claim": False}
    with output.open("x", encoding="utf-8") as stream:
        json.dump(result, stream, indent=2)
        stream.write("\n")
    print(f"Checked {len(rows)} material rows from {len(before)} sources: {output}")


if __name__ == "__main__":
    main()
