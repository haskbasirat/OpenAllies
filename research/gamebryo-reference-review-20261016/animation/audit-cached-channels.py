"""Read current manifest-bound animation caches; write counts only beside script.

No exporters, native routines, SDK tools, Rust code, or caches are executed or
modified. Counts describe current exports, not native in-memory stream fixups.
"""
from pathlib import Path
from collections import Counter
import hashlib
import itertools
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CACHE = ROOT / "assets-cache/animation"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def present(track):
    result = set()
    for name in ("translation", "scale"):
        if track[name]["keys"]:
            result.add((track["node"], name))
    r = track["rotation"]
    if r["kind"] == "constant_bind" or r.get("channel", {}).get("keys"):
        result.add((track["node"], "rotation"))
    return result


groups = []
for manifest_name, key, scope in [
    ("manifest.json", "assets", "current_live_five_infantry"),
    ("families-manifest.json", "exports", "separate_inspector_family_exports"),
]:
    manifest_raw = (CACHE / manifest_name).read_bytes()
    manifest = json.loads(manifest_raw)
    assets = []
    for item in manifest[key]:
        raw = (CACHE / item["file"]).read_bytes()
        assert sha(raw) == item["sha256"], item["file"]
        rig = json.loads(raw)
        clips = []
        presence = []
        for clip in rig["clips"]:
            channels = set().union(*(present(t) for t in clip["tracks"]))
            presence.append(channels)
            absent = Counter()
            single = Counter()
            endpoints = []
            populated = []
            for t in clip["tracks"]:
                for name in ("translation", "scale", "rotation"):
                    if name == "rotation":
                        if t[name]["kind"] == "constant_bind":
                            continue
                        keys = t[name]["channel"]["keys"]
                    else:
                        keys = t[name]["keys"]
                    if not keys:
                        absent[name] += 1
                        continue
                    populated.append((t["target"], name))
                    if len(keys) == 1:
                        single[name] += 1
                    if keys[0]["time"] != t["start"] or keys[-1]["time"] != t["stop"]:
                        endpoints.append({"target": t["target"], "channel": name,
                                          "keys": len(keys), "controller_start": t["start"],
                                          "controller_stop": t["stop"],
                                          "first": keys[0]["time"], "last": keys[-1]["time"]})
            clips.append({"name": clip["name"], "source": clip["source"],
                          "source_sha256": clip["source_sha256"],
                          "track_count": len(clip["tracks"]),
                          "nodes_without_track": [n["name"] for i, n in enumerate(rig["nodes"])
                                                  if i not in {t["node"] for t in clip["tracks"]}],
                          "empty_channels_on_existing_tracks": dict(absent),
                          "single_key_channels": dict(single),
                          "populated_channels": len(channels),
                          "channels_without_both_controller_boundary_keys": endpoints})
        changes = []
        for a, b in itertools.permutations(range(len(clips)), 2):
            lost = presence[a] - presence[b]
            if lost:
                changes.append({"from": clips[a]["source"], "to": clips[b]["source"],
                                "channels_present_only_in_from":
                                [{"target": rig["nodes"][node]["name"], "channel": name}
                                 for node, name in sorted(lost)]})
        assets.append({"file": item["file"], "sha256": sha(raw), "source": rig["source"],
                       "node_count": len(rig["nodes"]), "clips": clips,
                       "ordered_clip_switches_with_dropped_channels": changes})
    groups.append({"scope": scope, "manifest": manifest_name,
                   "manifest_sha256": sha(manifest_raw), "assets": assets})

result = {"format": "aa-current-cached-animation-channel-audit-v1", "date": "2026-10-16",
          "basis": "Read-only, manifest-hash-verified cached exports; not execution of original loader/interpolator or sequence transitions.",
          "groups": groups}
summary = []
for group in groups:
    clips = [c for a in group["assets"] for c in a["clips"]]
    missing = Counter()
    singles = Counter()
    for c in clips:
        missing.update(c["empty_channels_on_existing_tracks"])
        singles.update(c["single_key_channels"])
    s = {"scope": group["scope"], "assets": len(group["assets"]), "clip_bindings": len(clips),
         "distinct_source_kfs": len({c["source"] for c in clips}),
         "tracks": sum(c["track_count"] for c in clips), "empty_channels": dict(missing),
         "single_key_channels": dict(singles),
         "ordered_switches_dropping_channels": sum(len(a["ordered_clip_switches_with_dropped_channels"])
                                                   for a in group["assets"]),
         "assets_with_dropped_channel_switch": [a["file"] for a in group["assets"]
                                               if a["ordered_clip_switches_with_dropped_channels"]],
         "boundary_key_mismatches": sum(len(c["channels_without_both_controller_boundary_keys"]) for c in clips),
         "boundary_key_mismatches_with_multiple_keys": sum(
             e["keys"] > 1 for c in clips for e in c["channels_without_both_controller_boundary_keys"])}
    group["summary"] = s
    summary.append(s)
(HERE / "cached-channel-audit.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(json.dumps(summary, indent=2))
