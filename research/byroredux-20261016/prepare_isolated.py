"""Copy reviewed selected source into a small evaluation workspace.

The pristine download remains byte-identical. Only the isolated workspace's
manifests are reduced: no engine members, examples, integration tests, bsa dev
dependency, toolchain installer, setup scripts, or renderer build scripts.
"""
from pathlib import Path
import json
import shutil
import tomllib

HERE = Path(__file__).resolve().parent
REF = HERE.parents[1].parent / "reconstruction/references/byroredux-20261016"
SOURCE, OUT = REF / "source", REF / "isolated"


def value(v):
    if isinstance(v, str): return json.dumps(v)
    if isinstance(v, bool): return str(v).lower()
    if isinstance(v, list): return "[" + ", ".join(value(x) for x in v) + "]"
    if isinstance(v, dict): return "{ " + ", ".join(k + " = " + value(x) for k, x in v.items()) + " }"
    return str(v)


def section(name, entries):
    return "[" + name + "]\n" + "\n".join(k + " = " + value(v) for k, v in entries.items()) + "\n\n"


if __name__ == "__main__":
    assert not OUT.exists(), "Do not overwrite an existing isolated checkout."
    OUT.mkdir()
    root = tomllib.loads((SOURCE / "Cargo.toml").read_text())
    nif = tomllib.loads((SOURCE / "crates/nif/Cargo.toml").read_text())
    core = tomllib.loads((SOURCE / "crates/core/Cargo.toml").read_text())
    for name in ("core", "nif"):
        shutil.copytree(SOURCE / "crates" / name / "src", OUT / "crates" / name / "src")
    shutil.copyfile(SOURCE / "crates/core/Cargo.toml", OUT / "crates/core/Cargo.toml")
    selected = set(core["dependencies"]) | set(nif["dependencies"])
    deps = {k: v for k, v in root["workspace"]["dependencies"].items() if k in selected}
    text = section("workspace", dict(members=["crates/core", "crates/nif"], resolver="2"))
    text += section("workspace.package", root["workspace"]["package"])
    text += section("workspace.dependencies", deps)
    (OUT / "Cargo.toml").write_text(text)
    text = section("package", nif["package"]) + section("dependencies", nif["dependencies"])
    text += section("features", nif["features"])
    (OUT / "crates/nif/Cargo.toml").write_text(text)
    shutil.copyfile(SOURCE / "Cargo.lock", OUT / "Cargo.lock")
    (OUT / "EVALUATION-NOTES.txt").write_text(
        "Isolated AA parser evaluation of ByroRedux fa7591ca28b935ae411ecb1d1378843abdb77a5e.\n"
        "Source .rs bytes are copied unchanged from ../source; original manifests retained there.\n"
        "Only workspace/core/nif are built; root upstream Cargo/toolchain/scripts are not run.\n"
        "NIF manifest excludes dev-dependencies, integration targets and examples.\n"
        "Root Cargo.lock is the starting pin; Cargo may prune unrelated workspace entries.\n"
        "Root original Cargo/README declare MIT; no root LICENSE file is in the source tree.\n"
        "No production source, workspace Cargo.lock, renderer, assets, or runtime data modified.\n")
    print(OUT)
