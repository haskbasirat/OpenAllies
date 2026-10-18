"""Independent read-only native replay of placement basis and terrain CPU leaves."""
from pathlib import Path
import hashlib
import importlib.util
import json
import math
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "research/phase4-rng"))
from original_rng import Machine, original, modified
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EIP


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def product(left, right):
    return [[sum(left[i][k] * right[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def review_basis():
    source_path = HERE / "native-placement-basis.json"
    source = load(source_path)
    placements = load(ROOT.parent / "reconstruction/game/data/map-actors/Campaign/axis01_crete.json")
    by_id = {row["instanceId"]: row for row in placements["actors"]}
    assert len(source["cases"]) == 144
    assert source["source_sha256"] == hashlib.sha256(original.b).hexdigest()
    assert source["modified_sha256"] == hashlib.sha256(modified.b).hexdigest()
    assert source["producer_sha256"] == sha(HERE / "native_placement_basis.py")
    for window in source["ranges"].values():
        address = int(window["start"], 16)
        size = int(window["end_exclusive"], 16) - address
        assert original.range(address, size) == modified.range(address, size)
        assert hashlib.sha256(original.range(address, size)).hexdigest() == window["sha256"]
    machines = {pc: Machine(cw) for pc, cw in [("Single24", 0x007f), ("Double53", 0x027f), ("Extended64", 0x037f)]}
    conversion = [[1, 0, 0], [0, 0, 1], [0, -1, 0]]
    inverse_conversion = [[1, 0, 0], [0, 0, -1], [0, 1, 0]]
    maximum_error = 0.0
    source_placements = set()
    for case in reversed(source["cases"]):
        if case["instance_id"] is not None:
            placement = by_id[case["instance_id"]]
            assert placement["thingId"] == case["kind"]
            assert placement["orientation"] == case["heading"]
            source_placements.add(case["instance_id"])
        machine = machines[case["precision"]]
        machine.reset(0)
        machine.u.mem_write(machine.STATE, bytes(0x100))
        machine.u.mem_write(machine.STACK + 4, struct.pack("<I3f", machine.STATE, *case["heading"]))
        machine.u.emu_start(0x8715f0, machine.STOP, count=2000)
        assert machine.u.reg_read(UC_X86_REG_EIP) == machine.STOP
        assert machine.u.reg_read(UC_X86_REG_ESP) == machine.STACK + 4
        raw = bytes(machine.u.mem_read(machine.STATE + 0x38, 36))
        assert list(struct.unpack("<9I", raw)) == case["native_matrix_bits"]
        flat = struct.unpack("<9f", raw)
        native = [list(flat[i * 3:i * 3 + 3]) for i in range(3)]
        # Convert the actual native basis, independently of the oracle's native
        # matrix expectation, to the engine's documented (x,z,-y) render space.
        rendered = product(product(conversion, native), inverse_conversion)
        x, y, z = case["heading"]
        assert z == 0
        angle = math.atan2(x, -y)
        c, s = math.cos(angle), math.sin(angle)
        yaw = [[c, 0, s], [0, 1, 0], [-s, 0, c]]
        error = max(abs(rendered[i][j] - yaw[i][j]) for i in range(3) for j in range(3))
        assert error < 0.000001
        maximum_error = max(maximum_error, error)
    store = (ROOT / "crates/aa-assets/src/store.rs").read_text(encoding="utf-8")
    loading = (ROOT / "crates/aa-client/src/loading.rs").read_text(encoding="utf-8")
    picking = (ROOT / "crates/aa-client/src/inspection.rs").read_text(encoding="utf-8")
    gpu = (ROOT / "crates/aa-client/src/gpu.rs").read_text(encoding="utf-8")
    assert "x.atan2(-y)" in store
    assert "map(|_| placement_model_yaw(p.orientation))" in store
    assert "instance.yaw_radians" in loading and "Instance::at(" in loading
    assert "Instance::at(instance.position, instance.yaw_radians, instance.scale)" in picking
    assert "glam::Quat::from_rotation_y(yaw)" in gpu
    files = [source_path, HERE / "native_placement_basis.py", ROOT / "crates/aa-assets/src/store.rs",
             ROOT / "crates/aa-client/src/loading.rs", ROOT / "crates/aa-client/src/inspection.rs",
             ROOT / "crates/aa-client/src/gpu.rs"]
    report = {
        "schema": 1, "reviewer": "world_diagnostic", "passed": True,
        "source_cases_replayed": 144, "source_map_placements": len(source_placements),
        "maximum_render_basis_error": maximum_error,
        "method": "Reverse-order replay of unchanged 8715f0/70d270/436310 with no hooks; exact native f32 matrix bits; independent C*M*C^-1 conversion to renderer Y rotation.",
        "checks": ["Source executable identities, producer, and native code-window hashes verified.",
                   "Actual saved Crete heading vectors matched to original recovered placement records.",
                   "All 144 cases across PC24/53/64 match, including all four cardinal headings.",
                   "Rendering and picking both consume PreparedInstance.yaw_radians through gpu::Instance::at.",
                   "Unresolved/nonfinite extractor yaw retains the identity guard; raw saved assets are unchanged."],
        "limits": ["Planar orientation only; no pitch/roll or interpolation claim.",
                   "Yaw normalization retains at most the stated numerical difference from serialized native direction columns; not bitwise matrix parity.",
                   "No GPU frame, alpha-picking, or full game parity claim; final rendered acceptance is separate."],
        "sha256": {path.relative_to(ROOT).as_posix(): sha(path) for path in files},
    }
    (HERE / "independent-review.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def review_terrain():
    folder = ROOT / "research/phase5-terrain"
    fixture_path = folder / "native-layer-fixtures.json"
    source = load(fixture_path)
    assert source["producer_sha256"] == sha(folder / "native_layers.py")
    assert source["original_sha256"] == hashlib.sha256(original.b).hexdigest()
    assert source["v14_sha256"] == hashlib.sha256(modified.b).hexdigest()
    assert len(source["cases"]) == 1027
    spec = importlib.util.spec_from_file_location("independent_native_terrain", folder / "native_layers.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    machine = module.TerrainMachine()
    assert json.loads(json.dumps(machine.table)) == source["table"]
    assert load(folder / "mask-lookup.json")["table"] == source["table"]
    assert sum(row is not None for row in source["table"]) == 161
    for case in reversed(source["cases"]):
        actual = machine.layers(case["neighbors"], case["priorities"])
        assert actual == case["layers"]
    resources = source["authored_resources"]
    assert len(resources) == 33
    for row in resources:
        path = ROOT.parent / row["source"]
        assert sha(path) == row["sha256"]
        raw = path.read_bytes()
        assert list(struct.unpack_from("<HH", raw, 12)) == [row["width"], row["height"]]
    files = [fixture_path, folder / "native_layers.py", folder / "mask-lookup.json",
             ROOT / "crates/aa-assets/src/terrain.rs", ROOT / "crates/aa-assets/tests/terrain_layers.rs"]
    report = {
        "schema": 1, "reviewer": "world_diagnostic", "passed": True,
        "source_cases_replayed": 1027, "native_lookup_entries": 161, "authored_resources_verified": 33,
        "method": "Reverse-order replay of source original native neighborhoods, signed-priority qsort, gap closure and mask table construction; final resource leaf and allocator remain explicit intercepted seams.",
        "checks": ["Exact layer identities, signed priorities, masks, base masks and transforms match every saved source row.",
                   "512 binary grids, three directional-priority examples, and 512 deterministic random unique-priority grids verified.",
                   "Embedded 256-slot mask catalog exactly equals freshly constructed native table, with 161 populated entries.",
                   "All 33 recorded Global01 authored mask file hashes and TGA dimensions verified.",
                   "Rust source rejects priority ties and conflicting identity priorities; does not assume map palette order is native priority."],
        "limits": ["CPU planner is exported but not used by the current single-texture terrain renderer.",
                   "Only fully resolved 3x3 grids and unique material priorities are supported.",
                   "Final TransitionSet selector/resource resolution, block-border reconstruction and compositor texture-coordinate offsets are not ported.",
                   "Raw mask matrices must not be passed directly to the GPU as complete UV transforms.",
                   "GPU alpha equations, filtering, seams, original frame parity and water remain separate work."],
        "sha256": {path.relative_to(ROOT).as_posix(): sha(path) for path in files},
    }
    (folder / "independent-review.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    for name, action in [("placement_basis", review_basis), ("terrain_layers", review_terrain)]:
        report = action()
        print(json.dumps({"component": name, "passed": report["passed"], "cases": report["source_cases_replayed"]}), flush=True)
