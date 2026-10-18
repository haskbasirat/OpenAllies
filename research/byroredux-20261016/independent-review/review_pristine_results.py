"""Independently validate frozen actual-parser sample replay and alpha evidence."""
from pathlib import Path
from collections import Counter
import hashlib
import json

HERE = Path(__file__).resolve().parent
CORPUS = HERE.parent / "corpus"


def pin(path):
    raw = path.read_bytes()
    return {"path": str(path), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


original_path = CORPUS / "pyffi-baseline-sample.json"
published_path = CORPUS / "byro-pristine-sample.json"
replay_path = HERE / "byro-pristine-sample-replay.json"
fixture_path = CORPUS / "alpha-cross-check.json"
old = json.loads(original_path.read_text(encoding="utf-8"))
published = json.loads(published_path.read_text(encoding="utf-8"))
replayed = json.loads(replay_path.read_text(encoding="utf-8"))
fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
assert published == replayed, "Independent sample replay differs from frozen author output."
assert published["exe_sha256"] == "8db52279138af30502d8ca52a5799ba7189fbcf4d984cc3de12de20fd18e0399"
before = {row["path"]: row for row in old["records"]}
assert len(before) == len(old["records"]) == len(published["records"]) == 16
assert all(row["status"] == "clean" and row["unparsed_tail_bytes"] == 0 for row in before.values())
assert {row["path"] for row in published["records"]} == set(before)
fixture_rows = {row["path"]: row for row in fixture["cases"]}
assert len(fixture_rows) == len(fixture["cases"])
count = Counter()
details = []
for row in published["records"]:
    source = before[row["path"]]
    actual = row.get("result", {})
    assert row["status"] == "completed_child"
    assert source["sha256"] == row["sha256"] == actual["sha256"]
    if source["version"] >= 0x05000001:
        assert actual["header"]["block_order"] == source["block_order"]
    else:
        assert actual["header"]["block_order"] == []
    complete = actual.get("parser_telemetry_clean") is True
    diagnostic = actual["strict_diagnostic"]
    count[actual["status"]] += 1
    record = {"path": row["path"], "version": source["version"], "status": actual["status"]}
    if complete:
        assert actual["parsed_blocks"] == source["parsed_blocks"] == source["declared_block_count"]
        footer_measured = source["version"] >= 0x05000001
        if footer_measured:
            assert diagnostic["status"] == "all_blocks_and_root_footer_consumed"
            assert diagnostic["consumed_bytes"] == source["bytes"] == actual["bytes"]
            assert diagnostic["unparsed_tail_bytes"] == 0
            assert diagnostic["root_footer"] and all(index < actual["parsed_blocks"] for index in diagnostic["root_footer"])
            count["strict_footer_eof_measured_models"] += 1
        else:
            assert diagnostic["status"] == "not_run_inline_legacy"
            assert diagnostic["consumed_bytes"] is None and diagnostic["unparsed_tail_bytes"] is None
            count["strict_footer_eof_unmeasured_legacy_models"] += 1
        assert source["alpha"] == actual["alpha"]
        assert fixture_rows[row["path"]]["alpha_properties"] == source["alpha"]
        assert fixture_rows[row["path"]]["sha256"] == source["sha256"]
        count["exact_alpha_blocks"] += len(source["alpha"])
        count["alpha_absent_models"] += not source["alpha"]
        count["complete_models"] += 1
        old_geometry = {g["block"]: g for g in source["geometries"] if g["type"] in {"NiTriShapeData", "NiTriStripsData"}}
        new_geometry = {g["block"]: g for g in actual["geometries"]}
        assert old_geometry.keys() == new_geometry.keys()
        for index, geometry in old_geometry.items():
            other = new_geometry[index]
            for key in ("vertices", "uv_sets", "uv_lengths", "vertices_f32_bits_sha256", "uv_f32_bits_sha256"):
                assert geometry[key] == other[key], (row["path"], index, key)
            count["geometry_arrays_positions_uv_exact"] += 1
            count["triangle_digest_equal"] += geometry["triangle_indices_sha256"] == other["triangle_indices_sha256"]
            count["triangle_digest_different"] += geometry["triangle_indices_sha256"] != other["triangle_indices_sha256"]
        old_nodes = {n["block"]: n for n in source["nodes"]}
        new_nodes = {n["block"]: n for n in actual["nodes"]}
        for index in old_nodes.keys() & new_nodes.keys():
            a, b = old_nodes[index]["rotation"], new_nodes[index]["rotation"]
            if a != b:
                count["raw_matrix_different"] += 1
                count["raw_matrix_diff_is_exact_transpose"] += [list(v) for v in zip(*a)] == b
        record["strict_footer_eof_measured_by_separate_harness"] = footer_measured
        record["exact_alpha_properties"] = len(source["alpha"])
    else:
        assert row["path"] not in fixture_rows
        assert row["path"] in fixture["incomplete_excluded"]
        assert actual["truncated"] is True and actual["dropped_block_count"] > 0
        assert diagnostic["status"] == "block_error"
        record["first_strict_error"] = diagnostic["error"]
    details.append(record)
assert count["complete_models"] == 15 and count["partial_truncated"] == 1
assert count["exact_alpha_blocks"] == 95 and count["alpha_absent_models"] == 4
assert len(fixture_rows) == 15
report = {
    "schema": "byroredux_independent_pristine_sample_review_v1",
    "scope": "Frozen upstream parser with reduced manifests, actual original asset bytes, and separately reviewed strict diagnostic; no production importer/native rendering proof.",
    "pins": [pin(path) for path in (original_path, published_path, replay_path, fixture_path, CORPUS / "run_guarded.py", CORPUS / "harness/src/main.rs", CORPUS / "harness/Cargo.toml", CORPUS / "harness/Cargo.lock")],
    "independent_replay_report_exact": True,
    "sample_count": 16,
    "counts": dict(count),
    "sample_is_not_full_1900_file_parse_sweep": True,
    "records": details,
    "limitations": [
        "The source corpus inventory covers 1900 assets; this actual parser replay covers the selected 16 only.",
        "Public parse_nif infers the first qualifying NiNode subclass as root and does not certify root/footer/EOF. The separate harness measures the type-table file footer and EOF for complete samples.",
        "Raw f32 rotation transpose differences are representation evidence, not an AA rendering fix; original multiplication/basis authority remains separate.",
        "Triangle digest differences must not be discarded without separately checking winding and cyclic representation; positions and UV equality alone are insufficient.",
        "Source texture bindings/filenames, normal/color arrays, child links, controller keys, animation playback and native AA renderer behavior are not fully compared.",
        "The truncated NiVisData case is excluded from absence or full semantic claims.",
        "The 512 MiB process limit bounds research children only; it is not a game memory limit or game performance measurement.",
    ],
    "review_result": "accepted_for_bounded_parser_and_raw_alpha_evaluation_only",
}
path = HERE / "pristine-sample-review.json"
with path.open("x", encoding="utf-8", newline="\n") as output:
    json.dump(report, output, indent=2, ensure_ascii=True)
    output.write("\n")
print(json.dumps({"counts": dict(count), "review_sha256": pin(path)["sha256"]}))
