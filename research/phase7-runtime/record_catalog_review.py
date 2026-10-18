"""Record the bounded independent integration review after its test run."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
files = [
    "crates/aa-runtime/src/catalog.rs",
    "crates/aa-runtime/src/definitions.rs",
    "crates/aa-runtime/src/localization.rs",
    "crates/aa-runtime/src/restoration.rs",
    "crates/aa-runtime/tests/catalog_restoration.rs",
    "crates/aa-runtime/tests/localization.rs",
    "crates/aa-mapworld/src/bootstrap.rs",
    "research/phase7-runtime/catalog-independent-output/checks.json",
    "research/phase7-runtime/catalog-independent-output/tests.log",
    "research/phase7-runtime/settings-override-independent-review.json",
    "research/phase7-definitions/registry-order-evidence.json",
    "research/phase7-definitions/native-faction-constructors.json",
]
checks = json.loads((root / files[7]).read_text())
assert all(c["exit_code"] == 0 for c in checks)
report = {
    "schema": 1,
    "reviewer": "terrain_roads independent integration review",
    "passed": True,
    "scope": "Actual source Catalog, portable definition Bank, bounded localization and partial tutorial restoration. Economy refresh is excluded. This does not certify complete map restoration or live gameplay.",
    "files": {f: hashlib.sha256((root / f).read_bytes()).hexdigest() for f in files},
    "checks": checks,
    "rust_tests_passed": 6,
    "findings": [],
    "resolved_findings": [{
        "issue": "An uppercase-first Actor request previously preserved caller spelling in cached DefinitionProof and CharacterDefinitions instead of the authored canonical identity.",
        "resolution": "Catalog canonicalizes through ActorCatalog.find before building; bootstrap now emits source.id. The actual five-infantry regression starts with uppercase requests and verifies canonical proof and Character actor IDs.",
    }],
    "reviewed_boundaries": [
        "Completed Performer counts come from the private validated LoadedPerformer list; no authored-list count or mesh-derived count injection.",
        "Body/Mover/Organization/Detection source fields retain typed original provenance. Organization constructor parameters are per logical group, independently of expanded member slots.",
        "Character animation-set-dependent fields remain absent until caller supplies the actual selected native set; no guessed mesh or saved variant substitution.",
        "Portable Bank tokens are domain-qualified case-folded identities and are not native pointers, saved IDs, registry ordinals or simulation traversal order.",
        "Resource indices use the separately proved original seven-entry registry order. Canonical named definition IDs are preserved.",
        "Commander main/local ability handles are constructor-only and require_usable rejects them; max-range override is recorded without authorizing incomplete special-operation execution.",
        "Localization verifies supplied source hashes and supports a stated literal/ASCII/default-language subset. Ambiguity, missing keys and unproved escapes remain explicit errors.",
        "Tutorial01 and tutorial02 each restore their first Kingdom through concrete Commander state and register two saved Updatable identities before stopping at the unresolved suffix. Publication remains false; no fabricated completed actors or live world are exposed.",
        "Attempt uses decoded native clock/RNG and the source-backed sixteen-tick queue plan; restoration stops at the first Load error and retains its pending-state report.",
    ],
    "limits": [
        "Only the six selected integration/localization/settings tests were rerun here; independent lower-level native equivalence proofs are referenced, not represented as newly rerun.",
        "The source catalog still conservatively rejects unresolved duplicate/template ambiguities. This is not a complete native index or temporal template loader.",
        "Shared special-operation effects, full Kingdom suffixes, actor publication and GUI gameplay are outside the passing scope.",
    ],
}
output = root / "research/phase7-runtime/catalog-independent-review.json"
output.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({"passed": report["passed"], "tests": 6, "sha256": hashlib.sha256(output.read_bytes()).hexdigest()}))
