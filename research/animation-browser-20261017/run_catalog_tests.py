"""Run the real catalog module's two unit tests with an unused cache locator seam.

The cache_dir seam returns None; these tests exercise search and lexical path
rejection only. Actual installed discovery/load is checked by the release EXE.
"""
import subprocess
import package_animation01 as p


def main():
    out = p.QA / "catalog-unit03"
    out.mkdir(parents=True, exist_ok=False)
    source = p.ROOT / "crates/aa-client/src/animation_catalog.rs"
    before = p.record(source)
    harness = out / "catalog_tests.rs"
    harness.write_text('mod animation_preview { pub fn cache_dir(_: &aa_assets::AssetStore) -> Option<std::path::PathBuf> { None } }\n'
        + '#[path = "' + source.as_posix() + '"]\nmod animation_catalog;\n', encoding="utf-8")
    deps = p.ROOT / "target/x86_64-pc-windows-msvc/release/deps"
    command = ["rustc", "--test", "--edition", "2024", "--target", "x86_64-pc-windows-msvc", "-C", "lto=thin", "-C", "opt-level=2", "-C", "codegen-units=1", str(harness), "-L", "dependency=" + str(deps), "-L", "dependency=" + str(p.ROOT / "target/release/deps")]
    libraries = []
    for name in ("aa_assets", "anyhow", "serde", "serde_json", "sha2"):
        library = max(deps.glob("lib" + name + "-*.rlib"), key=lambda path: path.stat().st_mtime)
        libraries.append(p.record(library))
        command += ["--extern", name + "=" + str(library)]
    exe = out / "catalog-tests.exe"
    command += ["-o", str(exe)]
    with (out / "compile.log").open("x", encoding="utf-8") as log:
        result = subprocess.run(command, cwd=p.ROOT, stdout=log, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError((out / "compile.log").read_text())
    test = [str(exe), "--nocapture"]
    with (out / "test.log").open("x", encoding="utf-8") as log:
        result = subprocess.run(test, cwd=p.ROOT, stdout=log, stderr=subprocess.STDOUT)
    p.need(result.returncode == 0 and before == p.record(source), "Catalog tests failed or source changed")
    p.write_new(out / "receipt.json", dict(passed=True, scope=__doc__, command=command, test_command=test,
        tested_source_files=[before], test_binary=p.record(exe), libraries=libraries,
        evidence_files=[p.record(harness), p.record(out / "compile.log"), p.record(out / "test.log")]))
    print((out / "test.log").read_text())


if __name__ == "__main__":
    main()
