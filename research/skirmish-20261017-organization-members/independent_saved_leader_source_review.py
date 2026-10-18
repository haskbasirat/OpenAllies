"""Read-only freeze verification; writes only the separate source-review report."""
from pathlib import Path
import hashlib, json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
freeze = HERE / "freeze.json"
assert sha(freeze) == "8a954a5c1a8cab397866e4676d85c1690540ecb47e332f415d9c027d4b0e1eb4"
manifest = json.loads(freeze.read_text())
verified = {}
for path, expected in manifest["source_snapshots"].items():
    actual = sha(ROOT / path)
    assert actual == expected, path
    verified[path] = actual
report = {
    "schema": 1,
    "reviewer": "saved_leader_adapter",
    "status": "no blocking finding in bounded independent source review",
    "scope": "Frozen v1 adapter and field-loan integration; source review only",
    "freeze_sha256": sha(freeze),
    "reviewer_executions": 0,
    "reviewer_builds": 0,
    "production_edits": False,
    "source_snapshots_verified": verified,
    "observations": [
        "57CBA0 reuses the single add_member_fields coordinator. The current Organization remains installed and each short component loan ends before outward callbacks.",
        "Member Element, Body, Experience, Mover and Mind paths use current typed component storage and owner checks. Same staged ActorStore, moved Bank, Arena, scheduler owners and World services are borrowed rather than reconstructed.",
        "State effects commit the current bit prefix before leave/enter/actions; they reread current state and placement on both callback success and failure before final writes.",
        "5784C0 propagation captures an identity-only receipt for the actual Properties allocation, rereads actual count/slots and uses the sole instance State. Reached supporting 4CEF10 targets the Organization and preserves member as source.",
        "524B80 loans the same Arena with ActorAccess. Base24/28 stores are retained before later required children. Position/status predicates and Mind114/118 dispatch keep kind-specific branches explicit.",
        "The adapter preserves required errors for positive rank, active movement, 522890, arrival allocation, healthy home relations, nested propagation, relocation, interpolation and Zone4 callbacks. No always-successful replacement was found."
    ],
    "findings": [],
    "limitations": [
        "This review does not execute the live 57CBA0 adapter, validate a whole fresh Regiment spawn, or independently replay the native/portable fixtures.",
        "Author native and exact-kernel results remain separately attributed to freeze.json; compiled-only whole-adapter scope remains unchanged.",
        "Root's later stored_organization_member_mind real522890 successor is outside these frozen snapshots and this review."
    ],
}
out = HERE / "independent-saved-leader-source-review.json"
out.write_text(json.dumps(report, indent=2) + "\n")
print(sha(out))
