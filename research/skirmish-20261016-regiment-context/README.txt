Regiment saved Mind context and owned restore integration, 2026-10-16

Scope
-----
528C20 saved Regiment versions2..21 now apply against the actual current
Organization LayoutData, real world begin time (GWorld+E0), and source ActorData
registry. The existing contextless v0/v1 API is retained. Contextless v2+ still
returns an explicit missing-context error. This is restoration and staging,
not completion of Regiment state execution or a playable skirmish claim.

Native basis
------------
526260 constructor; 528C20 Load; 521C30 embedded state list; 52C580 base Mind
Load; 4D9C80/4DACE0 owned command reader; 52D2C0 path Load; 4C2D90 signed desired
formation index; 4C2E60/4BDBA0 ActorData lookup; 51FE60 engage insertion; 522E60
join-vector growth. The prior native-regiment.json supplies266 decoder cases,
of which240 successful v2..21 images are compared by the full-context test.

New native_formation_context.py executes120 signed-index/table cases across
PC24/53/64. Negative indices return null without touching a missing layout;
nonnegative indices use the real table, including null and duplicate entries.
Missing layout and out-of-range cases are explicit Rust errors.

New native_source_context.py executes3 additional full-load images, one per
precision. Its synthetic payload uses actual source names (stop/usa_infantry),
nonempty path, tracked/join lists, engage targets, text,2 queued commands and
one current command. The native oracle still has a named-registry pointer seam;
the runtime integration test separately resolves these names in the real v14
data. These bytes are not represented as an existing original saved game.

regiment_context_native replays all243 full-load images plus120 formation
cases. It compares all remaining scalar/byte/state/core words, state-stack
order, reference identity, path contents/counters/dirty word, sorted engage
targets, tracked/join order, text, pending/current command ownership, and
absence of the detached scalar slots. Independent instruction-pin audit is
in independent-pin-audit.json; it checks both original and v14 executable bytes.

Single owners
-------------
aa_mind::regiment_load::apply_with_context consumes Regiment and returns Owned:
tracked_words, OwnedPath, engage_targets, join_words, bombard_text,
desired_formation and construct_definition. Native pointer/count mirrors at
9C/A0..B8, E8,148,1DC/1E0,264/268/26C and2BC/2C0 are removed from Installed
where represented by this owner. Actual untransferred scalar and state fields
stay in Installed/Arena; no default path/list or duplicate mutable owner is
constructed. Reapplication to an already detached Mind is refused.

MindLoadContext borrows the real Organization slot and supplies actual owner
GTID/world time. The component stage temporarily holds the same Mind outside
the slot to borrow Organization, and always reinserts it before returning a
loader error. 528C20 does not reenter the Mind slot. Existing legacy adapter
implementations retain their old method via a default context-forwarding method.

Catalog resolves the current Organization's canonical layout directly; it
does not resolve the ActorData default layout again. +148 preserves CString
presence versus first-NUL lookup and one-step source translation. Unknown or
wrong-category references remain explicit errors. Command values are bound
and retained as distinct allocations, with raw boolean/trailing bytes intact.

RestoredMindParts.mind is now Option<Mind> solely to allow take/reinsert of the
actual core without a clone or dummy replacement. All returned Load/action
errors restore Some with the same core/state allocations. Missing core is an
explicit invalid-state error. The existing shared Ability identity guard in
saved_mind_actions is preserved. Staging validates before consuming records,
then moves Regiment Owned into ActorStore.regiment_owned exactly once alongside
the core, states and actual Commands owners. Duplicate/missing container owners
are rejected. No Staged fields or restoration.rs changes were needed.

Runtime validation
------------------
saved_command_ownership has7 tests, including a real-source test run explicitly
with AA_GAME_ROOT. It exercises all3 precisions, actual source Organization and
ActorData/command definitions, native payload decoding, component Load forwarding,
actual borrowed Organization address, direct after-take formation-range failure,
post-success callback failure, and one-time staging. Stack/pending buffers are
reserved before the failure probe, so address checks distinguish real moves
from cloning empty containers. Nonempty path/tracked/join/text allocations keep
their addresses at staging. Core state-node identities and command owners remain
intact. The surrounding registry record used for staging is explicitly a test
ownership graph; this does not assert an entire source map loaded successfully.

Logs: mind-tests.log, runtime-tests.log, shared-check.log, clippy.log.
The first source-test assertion reversed the two raw command wire byte names;
it was corrected to compare the recorded original native fields directly.
No production logic was changed to satisfy that assertion.

Limits
------
- Application preflights references, payload presence, state bindings and owned
  constructor fields before mutation. Native abort/allocator exception partial
  writes are not reproduced or claimed. A rejected missing source stays an
  explicit rejection, never a successful default.
- This owns successful Load and staging, not all live Regiment state adapters,
  destructor callbacks, original save writing, or complete skirmish startup.
- No full-map census, renderer, multiplayer or internet test was performed by
  this checkpoint. Root performs combined map acceptance separately.
- No original game files, release files, saves or profiles were modified.
- Strict Clippy is not claimed globally: pre-existing aa-rules/aa-actors lints
  block the dependency-inclusive command; see clippy.log. The no-deps command
  separately stops at pre-existing aa-mind lints in sai and the earlier
  regiment_load decoder/basic application; see clippy-no-deps.log.

Backups
-------
backups/regiment-context-before-20261016-171524.zip
backups/regiment-shared-glue-before-20261016-172523.zip
backups/regiment-option-mind-before-20261016-172838.zip
backups/saved_command_ownership-before-regiment-context.rs
