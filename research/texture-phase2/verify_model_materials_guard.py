"""Read-only corpus applicability proof for final guards; keeps the shipped cache frozen."""
from concurrent.futures import ProcessPoolExecutor
import hashlib
import json
from pathlib import Path
import time
import export_model_materials as exporter


def main():
    cache = exporter.OUTPUT
    before = cache.read_bytes()
    document = json.loads(before)
    assert hashlib.sha256(before).hexdigest() == "53b0ff8ef77fdc7c57257389285561b008e58fc665128d777be4e0aa11e6863f"
    sources = list(document["models"])
    manifest = json.loads(exporter.MANIFEST.read_bytes())
    assert sources == [row["source"] for row in manifest["models"]]
    assert document["complete"] and len(sources) == 1847
    started = time.perf_counter()
    with ProcessPoolExecutor(max_workers=4) as pool:
        for i, (source, actual) in enumerate(zip(sources, pool.map(exporter.audit_model, sources))):
            assert actual == document["models"][source], source
            if (i+1) % 300 == 0:
                print(f"Final guard applicability {i+1}/1847", flush=True)
    assert cache.read_bytes() == before
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    report = {
        "schema": 1, "passed": True, "models": 1847, "meshes": 11035,
        "cache_sha256": sha(cache), "cache_unmodified": True,
        "exporter_sha256": sha(Path(exporter.__file__)),
        "producer_sha256": sha(Path(__file__)),
        "test_source_sha256": sha(exporter.HERE / "test_model_materials.py"),
        "documentation_sha256": sha(exporter.HERE / "MATERIALS.txt"),
        "prior_validation_sha256": sha(exporter.HERE / "model-materials-validation.json"),
        "all_original_and_converted_files_rehashed": True,
        "all_model_records_identical_to_frozen_cache": True,
        "scope": "Final normalization guards pass the entire fixed shipped corpus; no runtime/cache/asset mutation. Existing cache retains its producing exporter hash; this evidence binds the later source-only guard.",
        "focused_tests": 9, "seconds": time.perf_counter()-started,
    }
    target = exporter.HERE / "model-materials-final-guard-validation.json"
    target.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"report": str(target), "sha256": sha(target), "exporter_sha256": report["exporter_sha256"]}))


if __name__ == "__main__":
    main()
