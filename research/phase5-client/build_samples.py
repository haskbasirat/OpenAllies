"""Verify unmodified native-oracle selections and bundle asset-free diagnostics."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
AREAS = ("actors", "navigation", "orders", "spatial")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


bundle = {"schema": 1, "sources": {}, "components": {}}
for area in AREAS:
    folder = ROOT / f"research/phase5-{area}"
    selection_path = folder / "client-samples.json"
    selection = load(selection_path)
    if area == "actors":
        source_path = ROOT / selection["oracle_path"]
        expected_sha = selection["oracle_sha256"]
    elif area == "navigation":
        source_path = ROOT / selection["source_fixture"]
        expected_sha = selection["source_fixture_sha256"]
    else:
        source_path = ROOT / selection["source"]
        expected_sha = selection["source_sha256"]
    assert sha(source_path) == expected_sha, f"{area}: source changed"
    source = load(source_path)
    if area == "actors":
        for row in selection["samples"]:
            index = row["source_case_index"]
            without_index = {k: v for k, v in row.items() if k != "source_case_index"}
            assert without_index == source["cases"][index], (area, index)
        count = len(selection["samples"])
    elif area == "navigation":
        cases = {row["input"]["name"]: row for row in source["fixtures"]}
        for row in selection["fixtures"]:
            assert row == cases[row["input"]["name"]], area
        count = len(selection["fixtures"])
    else:
        for row in selection["samples"]:
            assert row["fixture"] == source[selection["source_field"]][row["source_index"]], area
        count = len(selection["samples"])
    assert count == 6, f"{area}: expected six bounded samples"
    original = source["original_sha256"]
    accepted = source["v14_sha256"]
    if "original_sha256" in bundle:
        assert original == bundle["original_sha256"]
        assert accepted == bundle["accepted_sha256"]
    bundle["original_sha256"] = original
    bundle["accepted_sha256"] = accepted
    bundle["components"][area] = selection
    bundle["sources"][area] = {
        "oracle": source_path.relative_to(ROOT).as_posix(),
        "oracle_sha256": expected_sha,
        "selected_samples": count,
        "selection": selection_path.relative_to(ROOT).as_posix(),
        "selection_sha256": sha(selection_path),
        "adapter_source": (folder / "client_samples.rs").relative_to(ROOT).as_posix(),
        "adapter_sha256": sha(folder / "client_samples.rs"),
    }

output = HERE / "world-samples.json"
output.write_text(json.dumps(bundle, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"samples": 24, "sha256": sha(output), "bytes": output.stat().st_size}))
