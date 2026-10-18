"""Summarize previously parsed AA controller metadata without reparsing originals.

No original NIF/KF parser or SDK/runtime tools are executed. Writes one derived
JSON report in this research directory; source inventory/catalog stay unchanged.
"""
from pathlib import Path
from collections import Counter
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CATALOG = ROOT / "research/phase3-animation/animation-catalog.json"
INVENTORY = ROOT / "research/phase3-animation/source-inventory.json"
cat_raw, inv_raw = CATALOG.read_bytes(), INVENTORY.read_bytes()
catalog, inventory = json.loads(cat_raw), json.loads(inv_raw)
assert catalog["source_inventory_sha256"] == hashlib.sha256(inv_raw).hexdigest()
total = Counter()
files = Counter()
pack_total = Counter()
pack_files = Counter()
features = Counter()
pack = []
coverage = {k: [] for k in ["NiVisController", "NiAlphaController",
                          "NiTextureTransformController", "NiMaterialColorController",
                          "NiFlipController"]}
cat_text, inv_text = cat_raw.decode(), inv_raw.decode()


def line_of(text, source):
    pos = text.index('"source": "' + source + '"')
    return text.count("\n", 0, pos) + 1


for row in catalog["assets"]:
    counts = row["controller_counts"]
    total.update(counts)
    files.update(counts.keys())
    short = {"source": row["source"], "source_sha256": row["sha256"],
             "catalog_row_line": line_of(cat_text, row["source"]),
             "controller_counts": counts,
             "known_unsupported_features": row["known_unsupported_features"]}
    for cls in coverage:
        if cls in counts:
            coverage[cls].append(short)
    if row["embedded_building_pack_unpack"]:
        pack.append(short)
        pack_total.update(counts)
        pack_files.update(counts.keys())
        features.update(row["known_unsupported_features"])
vis = []
for row in inventory["assets"]:
    controls = [c for c in row["controllers"] if c["class"] == "NiVisController"]
    if controls:
        vis.append({"source": row["source"], "source_sha256": row["sha256"],
                    "inventory_row_line": line_of(inv_text, row["source"]),
                    "controllers": controls})
result = {
    "format": "aa-existing-controller-coverage-review-v1", "date": "2026-10-16",
    "basis": "Counts from existing parsed phase3 AA inventory/catalog; original assets were not freshly parsed or replayed.",
    "catalog": {"path": str(CATALOG.relative_to(ROOT)),
                "sha256": hashlib.sha256(cat_raw).hexdigest()},
    "inventory": {"path": str(INVENTORY.relative_to(ROOT)),
                  "sha256": hashlib.sha256(inv_raw).hexdigest(),
                  "source_file_count": inventory["source_file_count"],
                  "parsed_count": inventory["parsed_count"], "errors": inventory["errors"]},
    "all_controller_instances": dict(total), "files_with_each_controller": dict(files),
    "selected_non_transform_classes": coverage,
    "building_pack_unpack": {"file_count": len(pack), "controller_instances": dict(pack_total),
                              "files_with_each_controller": dict(pack_files),
                              "files_with_each_unsupported_feature": dict(features), "files": pack},
    "visibility_controller_details": vis,
    "visibility_values_not_retained_by_existing_inventory": True,
    "current_pose_bridge_scope": "Position/normal updates only; no visibility/material/UV/texture controller execution.",
}
(HERE / "controller-coverage-audit.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"files": inventory["parsed_count"],
                  "selected_classes": {k: {"instances": total[k], "files": files[k]} for k in coverage},
                  "pack_files": len(pack), "pack_controllers": dict(pack_total),
                  "pack_feature_files": dict(features)}, indent=2))
