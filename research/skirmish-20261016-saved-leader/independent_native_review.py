"""Read-only independent native replay. Never regenerate author fixtures."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("kind", choices=("core", "saved"))
    args = parser.parse_args()
    if args.kind == "core":
        folder = HERE.parent / "skirmish-20261016-borrowed-leader"
        fixture = folder / "native-borrowed-leader.json"
        generator = folder / "native_borrowed_leader.py"
    else:
        folder = HERE
        fixture = folder / "native-saved-leader.json"
        generator = folder / "native_saved_leader.py"
    before = {str(p.relative_to(ROOT)): sha(p) for p in (fixture, generator)}
    module = load("independent_" + args.kind, generator)
    document = json.loads(fixture.read_text())
    rows = document["cases"]
    outputs = []
    for row in rows:
        if args.kind == "core":
            output = module.Borrowed(row, row["image"]).execute()
        else:
            output = module.SavedLeader(row, row["image"]).run()
        assert output == row["output"], (row.get("name", row.get("input")), row["image"], row["precision"])
        outputs.append(output)
    for index in range(0, len(rows), 2):
        assert rows[index]["image"] == "original"
        assert rows[index + 1]["image"] == "v14"
        assert outputs[index] == outputs[index + 1]
    assert before == {str(p.relative_to(ROOT)): sha(p) for p in (fixture, generator)}
    report = {
        "schema": "independent-read-only-leader-native-review-v1",
        "kind": args.kind,
        "rows_reexecuted": len(rows),
        "precisions": sorted({row["precision"] for row in rows}),
        "images": sorted({row["image"] for row in rows}),
        "every_saved_expected_output_matches": True,
        "original_v14_pairs_match": True,
        "author_fixture_and_generator_preserved": before,
        "scope": "Read-only replay of unchanged native instructions and explicit author seams; no complete world or Mind implementation claim.",
    }
    target = HERE / ("independent-" + args.kind + "-native-replay.json")
    assert not target.exists(), target
    target.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": True, "rows": len(rows), "report": str(target), "sha256": sha(target)}))


if __name__ == "__main__":
    main()
