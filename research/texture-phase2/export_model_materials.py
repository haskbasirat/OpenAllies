"""Audit original native tags against the existing static converted mesh inventory.

Reads original/recovered files only. The sidecar changes no geometry or texture file.
ProcessObject (879A90) resets inherited RepColor/Sg at a non-NiGeometry node,
then normalizes its own string extras (8790B0/877C50), and checks tag existence
(871CC0). These bounded semantics are deliberately not arbitrary ancestor OR.
The old converter's traversal is reproduced only to identify existing mesh rows;
it is not proposed as a complete native scene/LOD/animation implementation.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ProcessPoolExecutor
import hashlib
import json
from pathlib import Path, PurePosixPath
import sys
import time
import uuid

GAME = Path(__file__).resolve().parents[3]
ROOT = GAME / "rust-rebuild"
sys.path.insert(0, str(GAME / "reconstruction/tools"))
from convert_models import NifFormat, ORIGINAL, string, transformed_values
from convert_models_animations import read_nif

CONVERTED = GAME / "reconstruction/assets/converted/models"
MANIFEST = CONVERTED / "manifest.json"
HERE = Path(__file__).resolve().parent
OUTPUT = ROOT / "assets-cache/model-materials.json"
TARGETS = [
    "Buildings/GBR/GBRCorps/GBRCorps.nif",
    "Buildings/GBR/GBRCorps/GBRCorpsPack.nif",
    "Buildings/GBR/GBRDepotOil/GBRDepotOil.nif",
]
TAG_TABLE = ("Mod", "Eq", "Ambient_RepColor", "EmRC", "RepColor", "FastPath", "tred", "Sg", "Sg2", "mkf")
MATERIAL_TAGS = ("RepColor", "Sg", "Sg2")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def checked_path(root: Path, relative: str) -> Path:
    parts = PurePosixPath(relative).parts
    if not parts or "\\" in relative or ":" in relative or relative.startswith("/") or any(x in ("", ".", "..") for x in relative.split("/")):
        raise ValueError(f"Unsafe source path: {relative!r}")
    path = root.joinpath(*parts)
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"Source escapes root: {relative}")
    return path


def extras(node) -> list[dict]:
    result = []
    seen = set()
    items = list(getattr(node, "extra_data_list", []))
    linked = getattr(node, "extra_data", None)
    while linked is not None and id(linked) not in seen:
        seen.add(id(linked))
        items.append(linked)
        linked = getattr(linked, "next_extra_data", None)
    seen.clear()
    for extra in items:
        if extra is None or id(extra) in seen:
            continue
        seen.add(id(extra))
        result.append({
            "extra_name": string(getattr(extra, "name", "")),
            "value": string(getattr(extra, "string_data", "")),
            "is_string": isinstance(extra, NifFormat.NiStringExtraData),
        })
    return result


def tag_evidence(items: list[dict], hierarchy: list[str]) -> dict[str, dict]:
    """Native named-presence, normalized CRLF lines, then whole-string fallback.

    877C50 does exact length + ASCII-insensitive name compare, with no trimming.
    An '=' value does not turn a present tag off (871CC0 never reads its value).
    Bare LF is not a native line delimiter. Embedded NUL ends native strings.
    """
    normalized = []
    table = {tag.lower(): tag for tag in TAG_TABLE}
    for item in items:
        name = item["extra_name"].split("\0", 1)[0]
        value = item["value"].split("\0", 1)[0]
        tags, leftover = [], []
        if item["is_string"]:
            lines = value.split("\r\n")
            for index, line in enumerate(lines):
                # The scan retains the last '=' and does not flush a final '='.
                if index == len(lines) - 1 and line.endswith("="):
                    if tags:
                        raise ValueError("Trailing '=' after normalized tags requires a native mutable-string audit")
                    leftover.append(line)
                    continue
                keyword = line.rsplit("=", 1)[0]
                tag = table.get(keyword.lower()) if keyword.isascii() else None
                if tag:
                    tags.append(tag)
                elif keyword:
                    leftover.append(keyword)
        # When all lines were consumed, 87926F..879277 removes the old extra;
        # 8792B0..8792B7 later attaches the canonical named tag objects.
        removed = bool(tags) and not leftover
        normalized.append((item, "" if removed else name, value, tags, removed))
    # Removing an earlier extra can shift native array iteration. Bare value
    # fallback gives the same result for the observed corpus's UserPropBuffers;
    # conflicting named strings with multiple normalizations need a full audit.
    if sum(bool(row[3]) for row in normalized) > 1:
        for item, _, value, tags, removed in normalized:
            if removed and item["extra_name"] in MATERIAL_TAGS and item["extra_name"] not in tags:
                raise ValueError("Conflicting named extras require a native extra-array normalization audit")
            if any(tag in MATERIAL_TAGS for tag in tags) and value.lower() not in {tag.lower() for tag in MATERIAL_TAGS}:
                raise ValueError("Multiple normalized extras with non-bare material tags require a native extra-array audit")
    found = {}
    for tag in MATERIAL_TAGS:
        for item, name, value, tags, removed in normalized:
            method = None
            if name == tag:
                method = "named_extra_presence"
            elif item["is_string"]:
                if tag in tags:
                    method = "native_string_line_normalization"
                if method is None and not removed and value.isascii() and value.lower() == tag.lower():
                    method = "native_string_value"
            if method:
                found[tag] = {"path": hierarchy, "tag": tag, "extra_name": item["extra_name"],
                              "value": item["value"], "method": method}
                break
    return found


def next_context(is_geometry: bool, inherited: dict, own: dict) -> dict:
    context = dict(inherited) if is_geometry else {}
    for tag in ("Sg", "Sg2"):
        if tag in own:
            context["shadow"] = own[tag]
    if "RepColor" in own and "shadow" not in context:
        context["team"] = own["RepColor"]
    return context


def audit_model(relative: str) -> dict:
    source_path = checked_path(ORIGINAL, relative)
    converted_path = checked_path(CONVERTED, str(PurePosixPath(relative).with_suffix(".json")))
    source_bytes = source_path.read_bytes()
    converted_bytes = converted_path.read_bytes()
    converted = json.loads(converted_bytes)
    if converted["source"] != relative or converted.get("format") != "aa-nif-mesh-v1":
        raise ValueError(f"Converted identity mismatch: {relative}")
    data = read_nif(source_path)
    # Bind the bytes actually parsed even if an outside actor changed a file.
    if source_path.read_bytes() != source_bytes:
        raise ValueError(f"Original changed during audit: {relative}")
    expected = []
    seen = set()

    def walk(node, hierarchy=(), context=None, properties=None, parent_transform=None):
        if node is None or not isinstance(node, NifFormat.NiAVObject) or id(node) in seen:
            return
        ordinal = len(seen)
        seen.add(id(node))
        name = string(node.name)
        hierarchy = hierarchy + (name,)
        transform = node.get_transform()
        if parent_transform is not None:
            transform *= parent_transform
        items = extras(node)
        own = tag_evidence(items, list(hierarchy))
        state = next_context(isinstance(node, NifFormat.NiGeometry), context or {}, own)
        props = dict(properties or {})
        for prop in node.properties:
            if prop is not None:
                props[type(prop).__name__] = prop
        # Reproduce historical export eligibility solely for exact row matching.
        legacy_names = [string(x.name) for x in getattr(node, "extra_data_list", []) if x]
        legacy_shadow = "Sg" in legacy_names or "Sgv" in legacy_names or " sg" in name.lower()
        selection = name.lower().startswith(("selectionbox", "selection box", "collision"))
        if isinstance(node, NifFormat.NiTriBasedGeom) and node.data:
            if bool(node.flags & 1) or legacy_shadow or selection:
                return
            vertices = len(node.data.vertices)
            triangles = len(list(node.data.get_triangles()))
            if vertices and triangles:
                skip = "shadow" in state
                evidence = [state[k] for k in ("shadow", "team") if k in state]
                alpha = props.get("NiAlphaProperty")
                depth = props.get("NiZBufferProperty")
                # Native default NiAlphaProperty is 0xEC (913B58); explicit
                # per-object RepColor suppression remains in the Rust mapper.
                render_state = {
                    "alpha_flags": int(alpha.flags) if alpha else 0xEC,
                    "alpha_threshold": int(alpha.threshold) if alpha else 0,
                    "zbuffer_flags": int(depth.flags) if depth else 3,
                    "zbuffer_function": int(depth.function) if depth else 3,
                }
                expected.append({
                    "mesh_index": len(expected), "source_node_ordinal": ordinal,
                    "hierarchy": list(hierarchy), "name": name,
                    "vertex_count": vertices, "triangle_count": triangles,
                    "skip": skip, "skip_reason": "native_shadow_tag" if skip else None,
                    "team_color_mask": not skip and "team" in state,
                    "tag_evidence": evidence,
                    "render_state": render_state,
                    # Preserve the authored sphere center rather than inventing
                    # an AABB midpoint. Use the same static transform/basis and
                    # decimal precision as the currently rendered mesh export.
                    "sort_center": transformed_values([node.data.center], transform),
                })
        children = list(getattr(node, "children", []))
        if isinstance(node, NifFormat.NiLODNode) and children:
            children = children[:1]
        for child in children:
            walk(child, hierarchy, state, props, transform)

    for node in data.roots:
        walk(node)
    meshes = converted["meshes"]
    if len(expected) != len(meshes):
        raise ValueError(f"Mesh count mismatch {relative}: {len(expected)} != {len(meshes)}")
    for row, mesh in zip(expected, meshes):
        actual = (mesh["name"], mesh["hierarchy"], len(mesh["positions"]) // 3, len(mesh["indices"]) // 3)
        wanted = (row["name"], row["hierarchy"], row["vertex_count"], row["triangle_count"])
        if actual != wanted or len(mesh["positions"]) % 3 or len(mesh["indices"]) % 3:
            raise ValueError(f"Mesh identity mismatch {relative} #{row['mesh_index']}: {actual[:2]} vs {wanted[:2]}")
        row["previous_team_color_mask"] = bool(mesh["material"].get("teamColorMask", False))
    return {"source_sha256": digest(source_bytes), "converted_sha256": digest(converted_bytes),
            "mesh_count": len(meshes), "corrections": expected}


def write_json(path: Path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.is_symlink():
        raise ValueError(f"Refusing linked output: {path}")
    temporary = path.with_name(path.name + "." + uuid.uuid4().hex + ".tmp")
    with temporary.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, separators=(",", ":"), ensure_ascii=False)
        stream.write("\n")
    temporary.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--targeted", action="store_true")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--output", type=Path, default=None)
    args = parser.parse_args()
    manifest_bytes = MANIFEST.read_bytes()
    manifest = json.loads(manifest_bytes)
    sources = [row["source"] for row in manifest["models"]]
    if len(sources) != 1847 or len(set(x.lower() for x in sources)) != len(sources):
        raise ValueError("Expected the unique 1847-model recovered manifest")
    selected = TARGETS if args.targeted else sources
    models = {}
    started = time.perf_counter()
    if not 1 <= args.workers <= 8:
        raise ValueError("Audit worker count must be 1..8")
    if any(relative not in sources for relative in selected):
        raise ValueError("Selected model absent from manifest")
    # Results remain authored manifest order, independent of worker completion.
    with ProcessPoolExecutor(max_workers=args.workers) as pool:
        for i, (relative, result) in enumerate(zip(selected, pool.map(audit_model, selected))):
            models[relative] = result
            if (i + 1) % 100 == 0:
                print(f"Audited {i+1}/{len(selected)} ({time.perf_counter()-started:.1f}s)", flush=True)
    if MANIFEST.read_bytes() != manifest_bytes:
        raise ValueError("Converted manifest changed during audit")
    all_rows = [row for model in models.values() for row in model["corrections"]]
    result = {
        "schema": 1, "complete": not args.targeted, "model_count": len(sources),
        "audited_models": len(models), "source_root": "assets/original/Data",
        "converted_root": "assets/converted/models", "manifest_sha256": digest(manifest_bytes),
        "exporter_sha256": digest(Path(__file__).read_bytes()),
        "semantics": "Native ProcessObject context reset + own named/normalized string tags; existing static mesh inventory only",
        "counts": {"meshes": len(all_rows), "shadow_helpers": sum(r["skip"] for r in all_rows),
                   "team_color_meshes": sum(r["team_color_mask"] for r in all_rows),
                   "team_flag_changes": sum(r["team_color_mask"] != r["previous_team_color_mask"] for r in all_rows)},
        "models": models,
    }
    output = args.output or (HERE / "targeted-model-materials.json" if args.targeted else OUTPUT)
    write_json(output, result)
    print(json.dumps({"path": str(output), "sha256": digest(output.read_bytes()), "counts": result["counts"], "seconds": time.perf_counter()-started}))


if __name__ == "__main__":
    main()
