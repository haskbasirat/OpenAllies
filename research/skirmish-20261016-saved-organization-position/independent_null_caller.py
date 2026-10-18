"""Independent native selected-null probes; never a Rust caller parity claim."""
from pathlib import Path
import copy
import importlib.util
import json

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("saved_position_independent_probe", HERE / "native_saved_position.py")
M = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(M)


def main():
    seed = json.loads(M.INPUT.read_text())[0]["input"]
    rows = []
    for image in ["original", "v14"]:
        for precision, cw in M.MODES:
            for name, mode, flag, mind, expected in [
                ("selected_null_mode6", 6, True, False, "ok"),
                ("selected_null_no_mind", 1, False, False, "ok"),
                ("selected_null_owner_flag30", 1, True, False, "guarded_null_read"),
                ("selected_null_present_mind", 1, False, True, "mind_64"),
            ]:
                data = copy.deepcopy(seed)
                data["world_mode"] = mode
                data["selected"] = None
                data["member"] = 0
                for actor in data["actors"]:
                    if actor["id"] == data["owner"]:
                        actor["flags"] = actor["flags"] | (1 << 30) if flag else actor["flags"] & ~(1 << 30)
                        actor["mind"] = mind
                machine = M.Probe(image, cw, data)
                try:
                    output = machine.run(0)
                    actual = output["outcome"]
                except AssertionError as error:
                    assert machine.null_accesses and "null-pointer access" in str(error), str(error)
                    actual = "guarded_null_read"
                    output = {"outcome": actual, "null_accesses": machine.null_accesses, "message": str(error)}
                assert actual == expected, (name, image, precision, output)
                rows.append({"image": image, "precision": precision, "name": name, "output": output})
    report = {
        "scope": "Independent isolated native null-caller probes only; no Rust malformed-caller parity claimed. No required service returns success.",
        "cases": rows,
    }
    (HERE / "independent-null-caller.log").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({
        "cases": len(rows),
        "outcomes": {key: sum(r["output"]["outcome"] == key for r in rows) for key in ["ok", "guarded_null_read", "mind_64"]},
    }))


if __name__ == "__main__":
    main()
