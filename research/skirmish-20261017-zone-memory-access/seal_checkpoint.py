"""Seal this bounded successor; never rewrites earlier source or fixtures."""
from pathlib import Path
from hashlib import sha256
import json
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent

def digest(path):
    return sha256(path.read_bytes()).hexdigest()

def pin(relative):
    p = ROOT / relative
    return {"path": relative, "sha256": digest(p), "bytes": p.stat().st_size}

def write(name, value):
    p = OUT / name
    if p.exists():
        raise RuntimeError(f"refusing to overwrite sealed {p}")
    p.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

refresh = ROOT / "crates/aa-runtime/src/load_store_memory_refresh.rs"
before = OUT / "before-load-store-memory-refresh.rs"
def kernel(p):
    text = p.read_text(encoding="utf-8")
    return text[text.index("trait World {"):text.index("struct Owned<")].replace("\r\n", "\n")
assert kernel(refresh) == kernel(before)
assert "Finished `release` profile [optimized] target(s) in 31.94s" in (OUT / "check02.log").read_text()
assert "3 passed; 0 failed" in (OUT / "exact-kernel-replay.log").read_text()

now = datetime.now(timezone.utc).isoformat()
write("findings.json", {
    "created_utc": now,
    "scope": "4EC4A0 same-owner Access extraction for an active Zone loan; unchanged native field kernel",
    "production": pin("crates/aa-runtime/src/load_store_memory_refresh.rs"),
    "predecessor": pin("research/skirmish-20261017-zone-memory-access/before-load-store-memory-refresh.rs"),
    "kernel_old_new_equal": True,
    "kernel_sha256": sha256(kernel(refresh).encode()).hexdigest(),
    "changes": [
        "Owned uses Access read/object_mut/placement over original ObjectFields, components and current placement",
        "refresh and refresh_with_placement retain their old signatures and delegate through StoreAccess to refresh_access",
        "RecordId resolves the original Object.memory record on every reached write; no detached record or ActorStore reconstruction",
        "Original actual-input recorder preserved with Access field reads"
    ],
    "validation": [
        {"kind": "current aa-runtime production check", "result": "PASS", "seconds": 31.94, "log": pin("research/skirmish-20261017-zone-memory-access/check02.log")},
        {"kind": "exact unchanged kernel with original fixture tests", "result": "3/3 PASS", "log": pin("research/skirmish-20261017-zone-memory-access/exact-kernel-replay.log"), "binary": pin("research/skirmish-20261017-zone-memory-access/exact-memory-kernel-tests.exe")},
        {"kind": "exact shared reveal coordinator existing117-row fixture", "result": "1/1 PASS", "log": pin("research/skirmish-20261017-zone-memory-access/exact-reveal-replay.log"), "binary": pin("research/skirmish-20261017-zone-memory-access/exact-reveal-kernel-tests.exe")}
    ],
    "limits": [
        "Exact kernel tests execute pure field/callback ordering, not the new Access owner adapter",
        "No new native process executions, cold unit link, natural startup completion or battle readiness claim",
        "check01 preserves five in-flight foreign source errors; coherent check02 passed after owner fixes"
    ]
})

actor_path = ROOT / "crates/aa-runtime/src/actor_world.rs"
actor = actor_path.read_text(encoding="utf-8")
zone = actor[actor.index("///Synchronous disjoint zone loan."):]
zone_snapshot = OUT / "reviewed-zone-fields-access.rs"
if zone_snapshot.exists():
    raise RuntimeError("review source already exists")
zone_snapshot.write_text(zone, encoding="utf-8")
write("independent-reveal-source-review.json", {
    "created_utc": now,
    "reviewer": "/root/world_startup_gap",
    "result": "ACCEPTED bounded source-only review; no concrete blocker found",
    "independence": "Reviewer authored the adjacent generic4EC4A0 extraction; this review covers State author's new stored_actor_reveal and ZoneMemoryAccess composition only",
    "reviewed": [pin("crates/aa-runtime/src/stored_actor_reveal.rs"), pin("crates/aa-runtime/src/stored_memory_access.rs"), pin("research/skirmish-20261017-zone-memory-access/reviewed-zone-fields-access.rs")],
    "whole_actor_world_at_review": pin("crates/aa-runtime/src/actor_world.rs"),
    "checks": [
        "Current and prior actors resolve through actual generation/key and original Entity fields; active Zone remains separately borrowed",
        "Access exposes no mutable identity/pose/Zone callbacks; Object memory mutations retain the sole original arena",
        "Published placement checks ObjectKey, index reference and reverse ActorId mapping before reading actual placement",
        "Reveal reuses existing117-row coordinator, fresh Kingdom190 before18C reads and immediate flag prefixes before memory",
        "Recursive container lookup uses actual component parent; no outward mutating callback between lookup and recursive call; cycles explicitly refuse",
        "Error returns retain earlier stores and remove only local recursion marker; first-sighting remains reached required"
    ],
    "qualifications": [
        "Source/placement preflights are portable binding guards, not native invalid-pointer/fault equivalence",
        "No panic/unwind parity claim",
        "Reviewer native executions0 and new owner-adapter runtime executions0",
        "Shared pure kernel tests and combined current production check are separate author validation, not independent owner execution",
        "No natural41 or whole battle readiness claim"
    ]
})
print(json.dumps({name: digest(OUT / name) for name in ("findings.json", "independent-reveal-source-review.json")}, indent=2))
