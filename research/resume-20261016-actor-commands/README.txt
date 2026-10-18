ActorCommandData source fields for saved Mind commands (2026-10-16)

API and implementation
----------------------
aa_rules::native::actor_command::predefined_actor_command(canonical_id)
  -> native::Result<ActorCommandDefinition>
ActorCommandDefinition is Copy and contains:
  id: &'static str       Canonical original command IDS.
  mind_type: u32         Definition+0C, native command type.
  component_slot: u32    Definition+18; Mind is slot4.
  behavior_flags: u32    Definition+2C, consumed by native Mind52C320.
PREDEFINED_ACTOR_COMMANDS exposes all39 records in registration order.

These definitions are code-predefined, independent of authored UI roots.
The API does not require an ActorCatalog, does not infer native type from
the authored data order, and does not apply Translate aliases again.
The parent owns resolving saved references and live command interning.
Unknown or empty IDs are errors. Saved null references remain the saved
reader's separate responsibility. No Stop/default fallback exists.

Native producer evidence
------------------------
1. CRT initializer9CD7E0 calls enum4DA9D0(B3CBA0,1), then registers its
   destructor9ED2F0 through atexit96CAB0. The original enum constructor
   calls4DA2F0 ->4DA1E0 and fills the39 names/type IDs. The oracle runs
   this actual CRT initializer; atexit is a recorded shutdown-only seam.
2. DataManager47F7C0(39,2) constructs the registry at GameData+274 and
   stores that address in B3AA58. Mode2 accepts only predefined objects.
3. GameData475870 allocates39 objects, passes hardcoded(type, component,
   flags) to4740F0, and registers each through4B4E60.
4. Constructor4740F0 stores type+0C, component+18, flags+2C; it obtains the
   name from the original enum.4B4E60 verifies creation order and registers
   the object. The registered +0C values are verified, not inferred.
5. Saved-reference reader4DAB60 calls manager Find463FA0. Native lookup
   tests prove ASCII case-insensitive built-in names, no whitespace trim,
   and null for unknown/empty input. The Rust API represents invalid input
   as an explicit error instead of a missing runtime callback/default.

Examples(type, component, flags):
  kill = (0,4,0x2130); move = (5,4,0x130); stop = (6,4,0)
  build_structure = (13,14,0x90010); research = (35,21,0x1000000)
  group_behavior_enable = (37,0xFFFFFFFF,0)
Non-Mind component slots are preserved; Mind dispatch can reject them as
the original does. No actor subtype handler is implemented by this table.

TGI UI flags are separate fields
-------------------------------
ActorCommand LoadData48CD00 reads non_group_command/apply_group_offset/
aggressive_command into bytes+3C/+3D/+3E, then hot-command flags at+3F/+40,
high_importance at+41 and preferred_position+44. These are not the +2C
behavior word. Its flag stage48CF0A..48CF97 reads +2C bit0 to force the
non-group byte for actor-data commands; it also forces that byte for
native types11/12/34. The byte changes do not change +2C.

The oracle executes that exact flag stage on all39 original authored
command roots at PC24/53/64 after parsing the actual template and8 source
files with original5BD080. It observes the UI bytes changing while the
three dispatch words remain identical before/after. Earlier localized
names, icon/cursor resources and deferred VFX fields, and later CString
cleanup are outside this bounded fragment; full UI-load parity is not
claimed. Those are not used to invent the three dispatch fields.

Proof and validation
--------------------
native_actor_commands.py runs ORIGINAL instructions using the established
layout/definition emulator harness. Heap/CString/mutex/line-source support
and page-zero guards are reused. The command type, component and flag
values are never supplied by the harness. All executed original byte
windows are compared against v14 and recorded with hashes in the fixture.
The first successful prototype invoked enum4DA9D0 with indexed mode0;
the final oracle invokes actual CRT9CD7E0, which passes growable mode1.
Both yielded the same records, but only the latter is the final evidence.

native-actor-commands.json:
  117 constructed+registered records (39 at PC24,PC53,PC64)
  30 real native lookup cases
  117 authored UI flag-stage loads (39 at each precision)
  9 pinned input source hashes (template plus8 authored files)

Four dedicated Rust tests pass with --include-ignored; see tests.log:
  every_builtin_matches_original_registration_at_all_precisions
  native_lookup_cases_and_invalid_ids
  authored_ui_flag_reads_preserve_native_dispatch_fields
  v14_authored_commands_match_pinned_native_inputs
The real-data test reads through V14Sources and confirms all9 input hashes
and all39 authored command IDs. This installation currently has UI roots
for all39; existence without those roots is established by the native
registration-before-TGI oracle and the API's lack of a catalog dependency.
source-census.json lists the table and authored IDs. findings.json records
full executable, source, generator, fixture and census hashes.

Reproduce from rust-rebuild in PowerShell:
  $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
  python research/resume-20261016-actor-commands/native_actor_commands.py
  $env:CARGO_TARGET_DIR='target/s-layouts'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  $env:AA_ACTOR_COMMAND_CENSUS='E:\Games\Axis and Alies\rust-rebuild\research\resume-20261016-actor-commands\source-census.json'
  cargo test -p aa-rules --release --locked --offline -j4 --test native_actor_command -- --include-ignored --nocapture
  python research/resume-20261016-actor-commands/record_findings.py

No generic parser/loader or runtime code was changed. No external
dependencies, artificial runtime caps, gameplay defaults, release files,
original files or saves were changed. The39 known command definitions are
game semantics, not a maximum number of live commands. Broad combined
aa-rules/aa-metagame compatibility checks belong to the parent task.
