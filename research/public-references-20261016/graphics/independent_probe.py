"""Read original descriptors and compare converter outputs without writing assets."""
from pathlib import Path
import copy
import hashlib
import importlib.util
import json
import struct
import sys
import types

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TOOLS = ROOT.parent / "reconstruction/tools"
sys.path.insert(0, str(TOOLS))
import convert_models as current


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    result_file = HERE / "independent-source-probe.json"
    assert not result_file.exists()
    fixture = json.loads((HERE / "material-retention-fixtures.json").read_text())
    previous = types.ModuleType("previous_converter")
    previous.__file__ = str(HERE / "before/convert_models.py")
    exec(compile(Path(previous.__file__).read_text(), previous.__file__, "exec"), previous.__dict__)
    previous.ORIGINAL = current.ORIGINAL
    old_converter = previous.Converter()
    converter = current.Converter()
    before_hashes = {}
    descriptor_reads = {}
    active_bytes = b""
    descriptor_read = current.NifFormat.TexDesc.read

    def observe_descriptor(descriptor, stream, data):
        start = stream.tell()
        result = descriptor_read(descriptor, stream, data)
        # AA's checked fixtures are exactly NIF 10.1.0.0. Ref, enum32,
        # enum32, uint32 are the first sixteen bytes of its TexDesc.
        assert data.version == 0x0A010000
        clamp, filtering, uv = struct.unpack_from("<III", active_bytes, start + 4)
        assert (clamp, filtering, uv) == (
            int(descriptor.clamp_mode), int(descriptor.filter_mode), int(descriptor.uv_set)
        )
        descriptor_reads[id(descriptor)] = {
            "offset": start,
            "bytes": active_bytes[start + 4:start + 16].hex(),
            "state": {"clampMode": clamp, "filterMode": filtering, "uvSet": uv},
        }
        return result

    current.NifFormat.TexDesc.read = observe_descriptor
    rows = []
    full_outputs = []
    capture = []
    material = converter.material

    def observe_material(model, properties):
        properties = list(properties)
        expected = None
        for prop in properties:
            if isinstance(prop, current.NifFormat.NiTexturingProperty) and prop.has_base_texture:
                expected = descriptor_reads[id(prop.base_texture)]
        capture.append(expected)
        return material(model, properties)

    converter.material = observe_material
    for source in dict.fromkeys(case["source"] for case in fixture["cases"]):
        path = current.ORIGINAL / source
        active_bytes = path.read_bytes()
        before_hashes[str(path)] = sha(path)
        descriptor_reads.clear()
        data = current.NifFormat.Data()
        with path.open("rb") as stream:
            data.read(stream)
        capture.clear()
        new = converter.convert(path, data=data, write=False)
        expected = list(capture)
        old = old_converter.convert(path, data=data, write=False)
        scrubbed = copy.deepcopy(new)
        for index, (mesh, raw_descriptor) in enumerate(zip(new["meshes"], expected)):
            actual = mesh["material"].get("baseTextureState")
            assert actual == (raw_descriptor["state"] if raw_descriptor else None)
            matching = [case for case in fixture["cases"] if case["source"] == source and case["mesh_index"] == index]
            assert len(matching) == 1
            assert matching[0]["material"] == mesh["material"]
            assert matching[0]["source_sha256"] == before_hashes[str(path)]
            scrubbed["meshes"][index]["material"].pop("baseTextureState", None)
            rows.append({"source": source, "mesh_index": index, "raw_descriptor": raw_descriptor})
        assert scrubbed == old, source
        full_outputs.append({"source": source, "entire_converted_output_except_metadata_unchanged": True})
        assert path.read_bytes() == active_bytes
    result = {
        "schema": "independent-aa-base-texture-retention-probe-v1",
        "original_nifs": len(full_outputs),
        "material_rows": len(rows),
        "direct_raw_descriptor_words_verified": sum(row["raw_descriptor"] is not None for row in rows),
        "checks": rows,
        "old_new_full_outputs": full_outputs,
        "source_hashes_unchanged": before_hashes,
        "assets_written": False,
        "native_renderer_executed": False,
        "visual_parity_claim": False,
    }
    result_file.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"passed": True, "rows": len(rows), "report_sha256": sha(result_file)}))


if __name__ == "__main__":
    main()
