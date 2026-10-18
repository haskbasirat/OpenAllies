"""Read original NIF material metadata; write only this review's aggregate JSON.

Uses the pre-existing local PyFFI reader, never SDK code or SDK executables.
The phase6 sidecar defines the audited exported-mesh population and hashes.
Counts are authored-data observations, not original-renderer parity evidence.
"""
from __future__ import annotations

from collections import Counter, defaultdict
from concurrent.futures import ProcessPoolExecutor
import hashlib
import json
from pathlib import Path
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / "reconstruction/tools"))
from convert_models import NifFormat, ORIGINAL, string
from convert_models_animations import read_nif

SIDECAR = ROOT / "research/phase6-material/model-materials.json"
OUTPUT = HERE / "original-material-census.json"
SLOTS = [
    ("base", "base_texture"), ("dark", "dark_texture"),
    ("detail", "detail_texture"), ("gloss", "gloss_texture"),
    ("glow", "glow_texture"), ("bump", "bump_map_texture"),
    *((f"decal{i}", f"decal_{i}_texture") for i in range(4)),
]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def color(value):
    return [float(value.r), float(value.g), float(value.b)]


def inspect_model(args):
    relative, metadata = args
    source = ORIGINAL / relative
    raw = source.read_bytes()
    if digest(raw) != metadata["source_sha256"]:
        raise ValueError(f"Original hash differs from audited sidecar: {relative}")
    data = read_nif(source)
    if source.read_bytes() != raw:
        raise ValueError(f"Original changed during inventory: {relative}")
    expected = {row["source_node_ordinal"]: row for row in metadata["corrections"]}
    rows = []
    seen = set()

    def walk(node, hierarchy=(), inherited=None):
        if node is None or not isinstance(node, NifFormat.NiAVObject) or id(node) in seen:
            return
        ordinal = len(seen)
        seen.add(id(node))
        name = string(node.name)
        hierarchy = hierarchy + (name,)
        props = dict(inherited or {})
        for prop in node.properties:
            if prop is not None:
                props[type(prop).__name__] = prop
        if ordinal in expected:
            row = expected[ordinal]
            if list(hierarchy) != row["hierarchy"] or name != row["name"]:
                raise ValueError(f"Mesh identity differs: {relative} #{ordinal}")
            if not isinstance(node, NifFormat.NiTriBasedGeom) or node.data is None:
                raise ValueError(f"Expected ordinary geometry: {relative} #{ordinal}")
            geom = node.data
            if len(geom.vertices) != row["vertex_count"] or len(list(geom.get_triangles())) != row["triangle_count"]:
                raise ValueError(f"Mesh size differs: {relative} #{ordinal}")
            facts = {}
            def fact(key, value):
                facts[key] = json.dumps(value, sort_keys=True, separators=(",", ":"))
            fact("authored_vertex_colors", bool(geom.has_vertex_colors))
            fact("uv_set_count", len(geom.uv_sets))
            for kind in ["NiTexturingProperty", "NiVertexColorProperty", "NiShadeProperty", "NiStencilProperty", "NiMaterialProperty"]:
                fact("property_present." + kind, kind in props)
            vertex = props.get("NiVertexColorProperty")
            fact("vertex_source_mode", int(vertex.vertex_mode) if vertex is not None else "absent")
            fact("vertex_lighting_mode", int(vertex.lighting_mode) if vertex is not None else "absent")
            shade = props.get("NiShadeProperty")
            fact("shade_flags", int(shade.flags) if shade is not None else "absent")
            stencil = props.get("NiStencilProperty")
            fact("stencil_draw_mode", int(stencil.draw_mode) if stencil is not None else "absent")
            fact("stencil_enabled", bool(stencil.stencil_enabled) if stencil is not None else "absent")
            material = props.get("NiMaterialProperty")
            if material is not None:
                fact("material_diffuse_nonwhite", color(material.diffuse_color) != [1.0]*3)
                fact("material_ambient_nonwhite", color(material.ambient_color) != [1.0]*3)
                fact("material_specular_nonzero", color(material.specular_color) != [0.0]*3)
                fact("material_emissive_nonzero", color(material.emissive_color) != [0.0]*3)
                fact("material_alpha", float(material.alpha))
                fact("material_glossiness", float(material.glossiness))
            texture = props.get("NiTexturingProperty")
            fact("apply_mode", int(texture.apply_mode) if texture is not None else "absent")
            present = []
            transforms = []
            if texture is not None:
                for slot, attr in SLOTS:
                    if getattr(texture, "has_" + attr, False):
                        present.append(slot)
                        desc = getattr(texture, attr)
                        fact("map." + slot + ".uv_set", int(desc.uv_set))
                        fact("map." + slot + ".clamp", int(desc.clamp_mode))
                        fact("map." + slot + ".filter", int(desc.filter_mode))
                        if getattr(desc, "has_texture_transform", False):
                            transforms.append(slot)
                            transform = {"translation": [float(desc.translation.u), float(desc.translation.v)],
                                "tiling": [float(desc.tiling.u), float(desc.tiling.v)],
                                "rotation": float(desc.w_rotation), "method": int(desc.transform_type),
                                "center": [float(desc.center_offset.u), float(desc.center_offset.v)]}
                            fact("map." + slot + ".transform_components", transform)
                            fact("map." + slot + ".transform_is_identity_components",
                                 transform["translation"] == [0.0, 0.0] and transform["tiling"] == [1.0, 1.0]
                                 and transform["rotation"] == 0.0)
                        fact("map." + slot + ".source_present", desc.source is not None)
                fact("shader_map_count", int(getattr(texture, "num_shader_textures", 0)))
            fact("map_slots", present)
            fact("texture_transform_slots", transforms)
            rows.append({"source_node_ordinal": ordinal, "mesh_index": row["mesh_index"], "hierarchy": list(hierarchy),
                         "skip": row["skip"], "team_color_mask": row["team_color_mask"], "facts": facts})
        # Match the audited sidecar's historical export traversal exactly.
        extras = [string(x.name) for x in getattr(node, "extra_data_list", []) if x]
        shadow = "Sg" in extras or "Sgv" in extras or " sg" in name.lower()
        selection = name.lower().startswith(("selectionbox", "selection box", "collision"))
        if isinstance(node, NifFormat.NiTriBasedGeom) and node.data and (bool(node.flags & 1) or shadow or selection):
            return
        children = list(getattr(node, "children", []))
        if isinstance(node, NifFormat.NiLODNode) and children:
            children = children[:1]
        for child in children:
            walk(child, hierarchy, props)

    for node in data.roots:
        walk(node)
    if len(rows) != len(expected):
        raise ValueError(f"Missing source rows: {relative}, {len(rows)} != {len(expected)}")
    return relative, metadata["source_sha256"], rows


def main():
    started = time.time()
    sidecar_bytes = SIDECAR.read_bytes()
    metadata = json.loads(sidecar_bytes)
    models = metadata["models"]
    groups = {name: {"mesh_rows": 0, "counts": defaultdict(Counter), "examples": defaultdict(dict)}
              for name in ["all_sidecar_meshes", "retained_meshes", "retained_non_team_meshes"]}
    sources = []
    with ProcessPoolExecutor(max_workers=4) as pool:
        for index, (relative, source_hash, rows) in enumerate(pool.map(inspect_model, sorted(models.items())), 1):
            sources.append({"path": relative, "sha256": source_hash, "mesh_rows": len(rows)})
            for row in rows:
                names = ["all_sidecar_meshes"]
                if not row["skip"]:
                    names.append("retained_meshes")
                    if not row["team_color_mask"]:
                        names.append("retained_non_team_meshes")
                for name in names:
                    group = groups[name]
                    group["mesh_rows"] += 1
                    for key, value in row["facts"].items():
                        group["counts"][key][value] += 1
                        group["examples"][key].setdefault(value, {"model": relative, "source_sha256": source_hash,
                            "mesh_index": row["mesh_index"], "source_node_ordinal": row["source_node_ordinal"],
                            "hierarchy": row["hierarchy"]})
            if index % 200 == 0:
                print(f"Read {index}/{len(models)} models", flush=True)
    result = {"schema": 1, "review_date": "2026-10-16", "population_sidecar": str(SIDECAR),
        "population_sidecar_sha256": digest(sidecar_bytes), "models": len(models), "groups": groups,
        "source_models": sources, "elapsed_seconds": round(time.time()-started, 3),
        "limits": ["Authored data census only; no original renderer or SDK tool executed.",
            "Uses existing PyFFI parser. Does not prove parser correctness or native default states.",
            "Every source hash and source mesh ordinal/name/hierarchy/size matches the existing audited sidecar.",
            "Counts effective properties under historical static exported-mesh traversal, including nearest LOD only.",
            "An absent property is recorded as absent, not replaced with an inferred SDK/native default.",
            "Not all original NIFs, animation state, particles, discarded LODs, dynamic override state or live native traversal.",
            "Each examples entry is the first matching mesh in sorted model order, not a rendering regression proof."]}
    OUTPUT.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(OUTPUT), "models": len(models), "rows": {k:v["mesh_rows"] for k,v in groups.items()},
                      "elapsed_seconds": result["elapsed_seconds"]}), flush=True)


if __name__ == "__main__":
    main()
