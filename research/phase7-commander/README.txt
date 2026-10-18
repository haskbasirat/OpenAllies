Commander/Faction owned construction and save application

Implemented: kingdom replacement4E0F00, Commander constructor4CE1D0, owned
cleanup4CE310, save application4CE0B0 and Ability4C8B40. Canonical FactionData
resolution and its full authored loader are separate from this state module.
Definition contains actual ordered LocalAbility inputs, never an injected count.
Every instance is created by the original4C8650 field semantics; Commander sets
recharge_bypass=true. Native+0C XP is the same balance consumed by4CDD90 payment.
Runtime must share this balance instead of creating a second payment copy.

Replacing a Commander first destroys old owned action/list storage, then creates
the new Commander. Dropping summoned-ID lists does not destroy summoned actors.
A null new definition leaves no Commander. Memory allocation addresses/capacity
and hardware exhaustion behavior are not portable state claims.

Save loading copies XP before the count check. If the saved action count differs
from the constructed definition table, the original emits a diagnostic, reads
and discards EVERY saved action, and keeps all constructed actions at defaults.
LoadOutcome reports this condition; the runtime must surface it as a diagnostic
and must not say discarded saved abilities were restored. Matching counts apply
saved version0/1 actions in order. use_count==0 resets last_use to -FLT_MAX,
regardless of the incoming word. Byte enabled is interpreted as native nonzero.
Version1 GTIDs are prepended in read order (thus reversed ahead of existing IDs);
version0 leaves the existing ID list untouched. Definition/power/bypass survive.
The physical+28 intrusive-list tail is represented by Vec ownership; Ability's
legacy word28 zero placeholder is not represented as a native address or a save
writer. The native oracle checks real list tails independently.

Input SavedState/SavedAbility records are a complete existing-codec payload,
not a new wire format. Invalid later records return an error with earlier field
writes retained. Raw float words are copied, not numerically reinterpreted.
No summed-ID actors are fabricated or resolved without the real registry.

40 original sequences /280 transitions pass. Native constructors, destructors,
list linkers and readers run unchanged from original/v14. Only allocation/free,
stream/version transport and mismatch diagnostic leaves are hooks. The oracle
normalizes native definition/actor pointers to fixture identities. One matching
Rust test and one partial-failure test pass; strict scoped Clippy passes.
This proof does not claim a complete tutorial/world restoration or Commander
SUMMON/supply-drop gameplay, which require real definition/runtime bindings.

A separate read-only discovery is pinned: FactionData loader48BAE0 calls4BF990
with actor=null, appends ordered Ability nodes unconditionally, and forces their
resolved shared definition range to FLT_MAX. The authored binding owner ports
that source boundary separately; this module does not self-claim that loader.

Commands:
python research/phase7-commander/native_commander.py
cargo test -p aa-combat --test native_commander
cargo clippy -p aa-combat --all-targets --no-deps -- -D warnings
