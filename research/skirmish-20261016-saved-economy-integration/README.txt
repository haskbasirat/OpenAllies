Saved Economy integration checkpoint, 2026-10-16

Production changes
------------------
saved_economy_refresh.rs now connects native4CF0F0/570020 to the actual
restored actor's owned Economy. It validates the real header owner, preserves
the initial virtual+108 result, and takes/reinserts the same Box on every
Result path. Earlier writes are retained on failure. No gameplay component,
Kingdom array, modifier, or command is cloned to supply a callback.

Each value callback receives the actual focused Economy and borrowed held
ancestor Economies. A recursive parent call takes that parent's real Box;
the current child remains available through the held references. Cycles are
reported as invalid unsupported runtime state, not replaced with success.
Definitions/source descriptors may be shared immutably; this is distinct
from mutable gameplay ownership.

Flow callbacks use actual Kingdom SpecialShell vectors1A0/1AC/1B8 through
saved_kingdom_flows and aa_economy::borrowed_flows. No reset, resize, synthetic
empty Economy, or rollback occurs in this adapter. The lazily read provider
cap remains after the original raw-production store. The separate proven
ResourceCatalog is explicitly bound; legacy callers with no metadata fail
when that dependency is reached.

The two Organization observer callbacks now delegate to Catalog's actual
managed-load owner. Default standalone loading still has Unknown lifecycle.
Only the separately proven fresh managed inactive token supplies the native
present-but-inactive branch. This change does not initialize Kingdoms or
activate SAI.

Observed natural record diagnostics
-----------------------------------
The test reloads tutorial02 with actual sources and explicit native mode1.
It is a partial record diagnostic; required immediate Init/PostCreate and
complete world publication are omitted and are not claimed as implemented.
Of46 actor records,17 have no Economy,29 have Economy and non-null owner.
27 stop at actual Kingdom1B8[0], whose constructor-only vector is empty.
Two linked headquarters,4100 usa_hq_infantry and8131 usa_hq_airborne, complete
the native gated refresh: upkeep is blocked, production is ineligible, Cost
is written, then their actual parent has no Economy and4CF0F0 returns. This
is callback progress, not a playable map or a complete load.

All46 cases retain the same Economy allocation where present, and all actor
word tables remain unchanged. Before/after component and Kingdom values are
saved in actual-natural-refresh.json. The final independently generated
observation has the same SHA256.

Native evidence
---------------
native_saved_refresh.py reuses the established native570020 harness with a
guarded page0: only real FS-prefixed SEH accesses may touch bytes0..3. It
loads original and v14 independently; visited instruction bytes agree.

* Six complete570020 executions use actual usa_corps4097 source/actor facts
  at PC24/53/64 in both images. This is an explicitly initialized-storage
  fixture: the test invokes only the already-proven Init flow-reset fragment
  on existing Kingdom storage. Production refresh never invokes that reset.
  Captured source Cost/Upkeep/Production values are independently executed
  from original code before being used as coordinator seams. Final component
  flags/vectors and actual Kingdom flow/stockpile vectors match Rust exactly.
* Twelve natural HQ executions retain empty Kingdom arrays. They execute
  actual56F9A0's linked early return and actual4CF0F0's absent-Economy return.
  Their gate and Cost facts are independently evaluated over the captured
  real saved/source graph. No Kingdom flow function is reached.
*54 additional source-getter executions support those18 coordinator runs.
  The source graph and getter implementation are independently covered by
  the saved-economy-values checkpoint; this integration does not claim that
  the complete getter call stack executes inside the coordinator oracle.
* The original entry probe36 cases and prior null-owner3 cases are retained.
  The test-only historical entry helper intentionally ends at its former
  boundary; production now calls the full coordinator instead.

The established coordinator oracle also covers159 native refresh scenarios
and450 equality cases. The borrowed flow provider separately covers876
canonical storage cases and count/cap identity. Those are reused dependencies,
not newly counted executions of this integration.

Ownership/error-only fixture
---------------------------
The test temporarily links existing HQ4100's Element to existing corps4097
and clears an existing Kingdom consumption array to force a later parent
failure. Both real Economy Box addresses are retained; child Cost is already
written; parent Upkeep assignment survives; the parent flag is not set and
its later Cost write is not reached. This deliberate in-memory graph is not
a natural saved relationship or a recursive gameplay parity claim. Separate
View tests verify actual focused/held member-cache writes.

Verification and remaining boundary
-----------------------------------
Four focused Rust tests pass, including original-output recompare and the
nested error fixture. final-native-recompare-tests.log is the final run.
No broad map census, UI/gameplay test, networking test, or release packaging
was performed by this subtask; root coordinates those separately.

The next natural dependency is real per-record Kingdom Init/storage setup,
not an arbitrary missing callback. This adapter deliberately does not fill
those missing arrays. Source evaluation still inherits established valid
constructed-source preflight limitations; active observer notification and
existing-world destruction remain outside the managed inactive lifecycle.

Reproduce from rust-rebuild (PowerShell)
--------------------------------------
$env:AA_GAME_ROOT='E:/Games/Axis and Alies'
$env:CARGO_TARGET_DIR='target/s-saved-economy-integration'
cargo test -p aa-runtime --lib saved_economy_refresh::tests --release --locked --offline -j4 -- --include-ignored --nocapture
python research/skirmish-20261016-saved-economy-integration/native_saved_refresh.py

Source freeze audit
-------------------
The census started with full-file hashab202df10e54b1a15b1385ca6caae57a0afe8dba0105fed8f1d8f3f02e30c478.
That exact file was reconstructed by reversing only recorded test-only patch
blocks/formatting into census-start-source-reconstructed.rs. Its hash matches
the prior independent census pin. Its production prefix before the final
#[cfg(test)] mod tests is byte-identical to the final source, hash
7d71e807afe71fd768909f254c1416399b148f02f1b8a1af8e7f29ac0dc71060.
The final source hash is recorded in findings.json. Later test additions do
not retroactively claim full-file immutability during the census.
