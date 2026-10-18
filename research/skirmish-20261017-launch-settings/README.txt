LaunchSettings native reconstruction checkpoint

Addresses and scope
-76E540 successful constructor ordering through the entry to76B420.
-514000 Blank,5141F0 Load,514050 Random,5141A0 Preset,944BC0 Metagame;
 shared513EB0/5137C0 initialization remains a single implementation.
-76D6C0 full diagnostic traversal,59F010 Header,5128F0 common Creator,
 5129A0/512F90/513010/5129E0 derived wrappers,5118A0/5116A0/5114D0
 rules,50E820/50E660/50E6B0/50E3E0 parameters,505EA0 RMC settings,
 784560 AppLaunchState.76D6C0 does NOT log its incoming label.
-50F4C0 constructs KingdomParameter+30 closed=0.510580 file loading
 retains it (unlike the distinct network50FF20 path);4A4C00 copies it.
-5A1A3F..5A1A5A calls49AD70 into Header+20/+28. Actual stored text
 companions and objective pairs are retained/copied, never re-localized
 during diagnostics. Missing fallback stays required; no empty substitution.
-5A19F7..5A1A1E checks nonnull Resource then511860 saved predicate
 before47EA40. SourceResource::load has a real nonnull owned Resource.
-47EA40 begins by478D20 replacing the map_strings object. The selected
 source and subsequent worker each take this path for nonsaved Headers.

Ownership
The same retained launch owner stores four distinct derived siblings and
its original Load. No Load retagging, raw constructor, Default or Clone is
exposed for the new siblings. RMC vectors are growable; native empty active
counts are retained without adding a capacity1 gameplay limit.
The new diagnostic reader borrows current Header68, Load, siblings and
AppLaunchState. SyncSink borrows actual SyncA and evaluates its gate at
each logger only after enabled/suspend. Disabled logging never skips source
reads. All sink failures retain the exact preceding output prefix.
Current closed30 and68 companions belong to each actual parameter owner.
Immutable source/Header/Creator and the mutable Session68 remain distinct.
Current localized names/objectives belong to their same parameter arrays.

MAPSTRINGS replacement
Only add_map_strings can mark a Text as a member of map_strings; a path
spelling cannot confer membership. Replacement removes those prior map
rows and their escape-index companions before reading the new source.
Global Text objects, allocation identities, the table identity and surviving
entry references are retained. Read/parse refusal retains the removal
prefix. The new source uses the already existing Text parser/escape policy.
The raw decoded Chunk is returned unchanged for the later TRIGGERS decoder.
No duplicate suppression or replacement of the immutable definition text
is performed. Whole host launch attempt03 passed after this producer was
bound; its worker then exposed the separate second-load append, now fixed.
Those live-flow results are owned/reported by the parent and Globals.

Native evidence (original/v14;PC24/53/64)
native-launch-constructors.json:120 actual derived constructor executions.
native-launch-order.json:6 constructor-publication prefixes;76B420 remains
 an explicit observed cutoff, not a claim that the full Reset executed.
native-launch-dump.json:84 paired rows/168 executions, including full typed
 logger sequences and service-stop prefixes.
native-app-dump.json:36 paired rows/72 executions, all11 logger cutoffs.
native-closed30.json:30 paired rows/60 executions; actual file-load retention
 for Header and appended Creator, plus real4A4C00 copy with noncanonical bytes.
native-header-pairs.json:15 paired rows/30 executions, empty/unique/missing
 key paths and first/second-pair partial failures.
native-source-map-strings-guard.json:42 paired rows/84 executions, both
 Resource pointer branches and all seven actual4981A0 enum flags.
native-map-strings-replacement.json:12 paired rows/24 executions, real old
 TextData destruction/new constructor/registration/Find. Global entry pointer
 survives; old map entries disappear; source-read cutoff retains this prefix.

Evidence qualifications
Raw fixture objects, verified source inputs and named transports are explicit.
Logger leaf calls in the traversal fixture are observed seams; their bodies
are covered by the existing synchronizer proof, not counted as executed here.
Native snapshots include setup instructions; instruction/basic-block counts
are not percentages of game completion. Earlier failed logs and narrower
pre-guard artifacts remain labeled alongside corrected final artifacts.
Header source readers are a value binding with input preflight, not a claim
of complete malformed-RMP allocator/SEH/I/O behavior. The source replacement
covers embedded MAPSTRINGS; unavailable external string files remain errors.
No full76B420, arbitrary callback reentrancy, missing-text fallback, general
language/duplicate Text registration, saved-header name override or every
GameSettings editor writer is certified. Diagnostic definition ordinals require
canonical mode0/1 source registration, matching current source identities and
no unresolved reloads. Positive additional-Actor diagnostic pointer ordinals
remain explicitly required. No game readiness, renderer, active battle,
network connection, audio output or release acceptance is conferred here.

Rust validation
All4 focused launch tests PASS. The three actual source maps start from the
baseline global text only and retain all constructor-nonnull siblings. Their
captured typed diagnostic events match18 fresh native executions (both PEs,
all three PCs;2269 executed instruction pins), then the SAME frozen Rust test
binary replayed every native result successfully. Seven existing creator,
labels and GameSettings unit regressions also PASS. No extra runtime relink
was used for these replays.

The immutable test executable path/hash and exact source/fixture/log receipts
are recorded in findings.json. It is a test-only optimized binary with LTO
explicitly disabled; it is not a new playable game or release acceptance.
The separate Stance test's now-corrected View field path still awaits the
parent's next coherent cohort. External game_settings_value_source and
resource_map_cache_source fixture-only baseline setup changes also await that
cohort. Failed/transient logs remain preserved and are not counted as passes.

Backup qualification
The before directory holds the captured pre-edit source files, including the
narrow restoration site. No contemporaneous pre-edit capture was made for the
creator_labels objective companion and load_world_rules read-only getters;
no reconstruction is misrepresented as an original backup. No original game,
save, profile, release or external account file was modified in this scope.
