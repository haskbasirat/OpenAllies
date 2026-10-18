PHASE7 SAVED WORLD RESTORATION (IN PROGRESS)

Runtime API:
 restore::restore_records defaults to StopOnError. It resolves actual definitions,
 constructs base load shells, registers saved IDs without reseeding, then runs
 derived Load stages. A failure keeps earlier effects and stops later objects.
 restore_records_with_mode(...,InspectPartial) is diagnostic only.
 No result currently claims runtime_publication_ready: true.

RuntimeDefinition carries source DefinitionProof, final flags/native class,
 Object scale/variation inputs and shared aa_actors::owned::Definitions.
 RestoredComponent is the shared aa_actors::owned::Component<String> alias.
 Runtime and fresh actors do not get separate copies of authoritative Body,
 Mover, Properties, Element, Economy, Experience or Detection states.

Component stages follow original4D919C: Construction15, numeric0..30 excluding
15/4, Mind4 last. Missing defaults are constructed in place in that sequence.
 Original4D8290 predicates are matched across248 native cases. Obsolete saved
 Body/XP are loaded then removed; missing Properties may be constructed even
 when its original default bit was set. Unsupported factories/codecs stop.
 Positive saved Updatable IDs call the required host registration callback
 immediately after component insertion and before remaining saved fields.
 Failed callbacks leave constructor state and stop; successful trace entries
 do not replace the host's actual Updatable ID table.

Proof and tests:
 native_object_load.py ->144 complete original Object Load cases.
 native_saved_components.py ->170 original component-reader cases. Versions:
 Body0..4, Mover0..8, Character0..2 (animation0), Construction0..1,
 Performer5..9, Supply0..2, Denizen0..4 (no command objects), Structure0..13
 (empty queues/legacy refs), Element0..3 including negative history count,
 Experience0, MobileDeployment0/1/3, shared Updatable-only leaves.
 native_component_actions.py ->248 complete original4D8290 branch cases with
 explicit ctor/Load/delete observation leaves, stopping at definition errors.
 native_component_order.py ->31 original caller dispatches,8 mask bytes.
 native_special_constructors.py ->21 whole original Team/Kingdom saved ctors,
 including base constructors, Resource-sized vectors, embedded recruitment.
 native_kingdom_actor_load.py ->656 full original Actor-layer returns for all
328 source Kingdoms, both numbering flag settings, with actual bit29 no-spatial
 path/null owner/empty Actor name. Prior Thing/Object body is a phase6 seam;
 constructor15C baseline remainsFFFFFFFF. Broad Actor spatial/name branches
 are not claimed by this bounded path.
 native_kingdom_prefix.py ->60 original Kingdom prefix/resource cases, including
 duplicate source assignments, finite/nonfinite/signed-zero values and caps.
 native_kingdom_ai.py ->25 native tactical/strategic constructor and bounded
 saved cases. Tactical nonempty definition/handle tables remain unsupported.
 Strategic v0 mapping preserves the native untouched final word; v1 loads66.
 native_kingdom_suffix.py ->69 original loads in33 sequences, versions0..20,
 including duplicate-definition insertion/rehash, signed badge-list bounds,
 missing definitions, raw final Resource words, repeated loads and byte order.
 Definition tokens carry equality only, never guessed native pointer hashes.
 Cross-key hashed iteration is unavailable; per-key native lookup order is kept.
 native_dynamic_doodad.py ->512 whole derived saved-load cases plus3 whole
 original constructors. Actual two-slot factories and component readers run;
 prior GObjectLoad is the separately proven seam. Updatable publication is
 immediate, and world+48=0 resets Breakable state after consuming its payload.
 native_start_location.py ->8 whole ordinary Actor returns from the2 actual
 tutorial01 start_loc records, with real registered42-state descriptor groups,
 original Economy factory/ctor/requirement/Load, ownable/null-owner variants.
 Only the actual bit29/empty-reference/name/unnumbered branch is complete;
 ordinary Actor spatial/name/numbering services remain explicit dependencies.
 native_map_objects.py ->90 complete derived Area/Camera readers plus6 whole
 constructors. Both derive Object; classes6/7 now project that actual base
 layer in Inventory instead of retaining it inside the opaque subtype tail.
 Area Updatable publication, shape words and head-inserted saved entries are
 restored. Camera v0 conversion remains unsupported; source corpus usesv1/v2.
 native_construction.py ->224 whole Construction constructors and saved loads
 with a proven empty predecessor chain and actual null-modifier resource costs.
 Noneditor Load resets primary costs to the definition while retaining the
 second saved vector. Nonempty upgrade identity and saved arithmetic NaN cases
 remain explicit dependencies. The separate unicorn-construction-nan-observation
 records the emulator limitation; it is not accepted physical-x87 parity proof.
 native_denizen.py ->90 whole empty-Inventory Denizen constructors and180 loads,
 preserving version gates, discarded excess occupants, raw bytes and repeated
 head insertion of saved handles. Nonempty Inventory/commands are unsupported.
 native_ground_requirements.py ->2048 whole4CFB00 dependency walks for all subsets
 of11 supported concrete ground types, using original vtables and virtual leaves.
 Construction needs Body; Experience needs Properties; other9 have no dependency.

Commands:
 cargo test -p aa-mapworld --test restore --test restore_components --test saved_components --test actor_payload --test special_state --test kingdom_actor_restore --test kingdom_ai --test kingdom_suffix --test commander_restore --test dynamic_doodad --test restore_stop -- --include-ignored
 cargo clippy -p aa-mapworld --lib --no-deps -- -D warnings

Coverage audit currently decodes6177/11701 complete Actor tails. This is wire
coverage, not gameplay completion. Organization/Mind and several building
components remain unresolved. Native component constructors need actual
source/selected-view inputs; empty or guessed placeholders are not supplied.
 Saved Character must resolve the actual saved Object variant and native view.

World entities:
 Team base +derived Load is implemented with required localization callback.
 Kingdom common Actor/Updatable is applied, then typed prefix through color,
 diplomacy vector and Resource mapping/clamp. Team saved identity is looked up
 in the one registry; missing/stale identity becomes null as native code.
 Commander uses the shared native-proven aa-combat constructor/load with exact
 source callback inputs; count-mismatched saved abilities are discarded as the
 original does and the diagnostic remains explicit. Nation identities, bounded
 tactical/strategic saved state, definition counts, Nation Badge texture indices
 and final resources restore into owned state. Only fully covered records reach
 AwaitingPostLoad; that state does not mean live-world initialization is done.
 Immutable source payloads remain alongside partial state, with errors and
 explicit completion stages; they are not exposed as finished live objects.

 Dynamic ownership now binds from the current generation-checked GTID registry.
 Source owner slots remain diagnostic metadata, not evidence of registration.
 The ordinary Actor Load requirement check runs before reading its saved tail.
 Native classes6/7 retain owned Area/Camera state in LoadedEntity.special_object.
 Construction and Denizen are canonical shared owned::Component boxes, so the
 runtime arena consumes their actual saved state without reinitialization.

Independent common-code review passed by navigation. It found and fixed:
 -Mover v8 speed-error ordering: only10-vector/20/24 precede speed getter/math.
 -Element v3 negative history count uses signed skip, and1C/time24 reset.
 Regression tests exercise each failure boundary as well as successful values.

No original game executable, recovered source asset, save or previous release
has been modified by these restoration tools. Research uses mapped copies of
original x86 instructions under Unicorn; it does not drive a live game.
