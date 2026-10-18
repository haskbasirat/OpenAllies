"""Checkpoint only animation-task source changes against sealed HF02.

Root supplies exact predecessor paths; hash verification forbids substituting
HF01/older variants or reconstructing source. No source/release file is edited.
"""
import argparse
from pathlib import Path
import package_animation01 as p


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--predecessor", action="append", default=[], metavar="SOURCE=EXACT_HF02_COPY")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    baseline_name = "release/Development-10-Hotfix-02/evidence/build-record.json"
    baseline = p.read(p.ROOT / baseline_name)
    old = {row["path"]: row for row in baseline["source_files"]}
    current_rows = p.source_rows()
    current = {row["path"]: row for row in current_rows}
    p.need(not old.keys() - current.keys(), "Removed HF02 sources require explicit review")
    changed = {name for name in old.keys() & current.keys() if old[name] != current[name]}
    provided = {}
    for value in args.predecessor:
        source, separator, copy = value.partition("=")
        p.need(separator and source in changed and source not in provided, "Bad/duplicate/unneeded predecessor mapping")
        provided[source] = p.inside(p.ROOT, copy)
    p.need(set(provided) == changed, "Supply an exact HF02 copy for every changed existing source")
    checkpoint = p.ROOT / "backups/source-checkpoint-20261017-animations01"
    output = Path(args.output).resolve()
    p.need(output.is_relative_to(checkpoint.resolve()) and output != checkpoint.resolve(), "Index must stay in the animation source checkpoint")
    files = []
    for name in sorted(changed):
        source = provided[name]
        expected = old[name]
        p.need(source.stat().st_size == expected["bytes"] and p.sha(source) == expected["sha256"], "Predecessor is not exact sealed HF02 source")
        target = checkpoint / "pre-animation-source" / p.relative(name)
        if target.exists():
            p.need(target.is_file() and target.stat().st_size == expected["bytes"] and p.sha(target) == expected["sha256"], "Existing backup differs; never replace it")
        else:
            p.copy_new(source, target)
        files.append(dict(source=name, backup=p.record(target), sealed_predecessor=expected, recovered_from=p.record(source)))
    index = dict(schema=1, scope="Only source changes since sealed HF02; its earlier 29 Hotfix02 changes are baseline, not this task's delta.",
        baseline=p.record(p.ROOT / baseline_name), files=files,
        new_files=sorted(current.keys() - old.keys()), current_source_files=current_rows,
        counts=dict(changed_existing=len(changed), new_files=len(current.keys() - old.keys()), current_source_files=len(current)))
    p.write_new(output, index)
    print(p.record(output))


if __name__ == "__main__":
    main()
