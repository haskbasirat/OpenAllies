NameList compatibility follow-up, 2026-10-16

Scope
This note covers the NameList input/loader repair, not all actor creation.
The earlier native-actor-creation/component/object fixtures are preserved.

Pre-existing failure
The complete aa-rules --include-ignored run failed in the old
dump_real_actor_creation_definitions diagnostic on ukraine_city01, whose
name_list_ids is namelist_ukraine. The 12-actor loop had recently added two
British truck/trailer actors and this Ukrainian city, but the preserved
real-definitions.json contains only the original nine actors. Generic
children("Name") rejected the 48 repeated, non-template Name children with
"multiple children Name lack a native list declaration". The old string-only
field getter would also have rejected their quoted implicit localized values.
The initial failure log remains qa-output/resume-20261016/rules-compatibility.log.

Native evidence
GameData construction 48A8E0 -> 484D90 creates the NameList manager at +59C.
Its vf1C is 484EF0 -> 484FB0 -> NameList constructor 4C1630 (vtable A22F64).
LoadData 4C19D0 calls base 47D960, then 5B4220("Name") and iterator 42BE10
for ordinals 0..count. Parser 5B5C80 renames duplicate plain Name siblings to
?0000000_Name, ?0000001_Name, etc., and 42BE10 retrieves that same order.
No explicit [list Name] declaration is required by this path.
For each child, 5B5620("name") supplies the localized text and 4C1910 appends
it to +18/+1C. 5B5620 strips a leading '#' before 4B9F40 and supplies the
native empty CString default for missing keys. Untyped plain identifiers,
untyped null, and declared plain-string values fail the localized getter;
quoted values and declared localized null work. One leading '@' and quotes
are handled by native parse-time logic, not by the NameList loader.

native_name_lists.py runs the ORIGINAL whole parser 5BD080, constructor
4C1630, loader 4C19D0, iterator, getter and vector appends under Unicorn.
It uses the existing shared definition/parser machine. Seams are byte-line
input, allocation/pools/mutex and throw handling, plus a declared dictionary
4B9F40 localization seam. This seam has synthetic text values; the fixture
does not claim original localized city-name spelling or the full localization
manager is exercised. The 15 cases include the exact original Ukraine file,
48 names, duplicates/case, interspersed non-Name children, template lists,
localized keys/missing keys, quote/null rules, and byte-to-UTF16 widening.
Every executed image instruction is checked equal in original and v14.
native-name-lists.json pins source bytes, producer/dependency hashes, both
executables, native instruction windows and observed outputs.

Implementation
ResolvedNode has two crate-private NameList-only helpers. Generic children()
and field() guards are unchanged. Native untemplated ordinary Name children
can repeat; other unverified Name declaration/inheritance forms still error.
The explicit APIs are:
  ActorCatalog::native_name_list_with_lookup(ids, pc, lookup)
  ActorCatalog::native_actor_creation_inputs_with_lookup(id, count, pc, lookup)
lookup uses native::display_name::Lookup (UTF16 key -> optional UTF16 text).
The older no-callback entry points reject a localized #key when no text-table
owner is supplied. They never pass an untranslated #key as a runtime name.
These APIs produce inputs; they do not install a live NameList or reproduce
its partially appended vector after a later load error. Glyph/escape expansion,
unpaired UTF16 output and unsupported Name declaration forms remain explicit
unsupported inputs. No gameplay constants, native files or saves changed.

Diagnostic/export safety
real_actor_creation_source_projection_matches_preserved_baseline is read-only
by default. It compares all nine existing baseline actor rows and exercises
the three additional actors. Its explicit lookup echoes #keys only for a
clearly labeled source projection, and its Ukraine order is compared against
the 48 lookup keys observed by the original parser/loader on hash-pinned bytes.
This projection is not a loaded/localized runtime NameList.
Only AA_EXPORT_ACTOR_CREATION_SOURCE=<new output file> permits export. The
writer uses create_new: it cannot overwrite the existing fixture or any other
existing file. real-definitions.json stays at SHA256
ac65c09452940b31066163f59cdab6ce69ed332778ceec6b5996e40f1d87b4ec.

Reproduce
Set PYTHONPATH to reconstruction/tools/python-packages, then:
  python research/phase8-actor-creation/native_name_lists.py
The default compares its existing fixture; --write explicitly regenerates it.
For Rust, set AA_GAME_ROOT=E:\Games\Axis and Alies and
CARGO_TARGET_DIR=target/s-actorcreate, then:
  cargo test -p aa-rules --release --locked --offline -j 4 -- --include-ignored
  cargo test -p aa-metagame --release --locked --offline -j 4 -- --include-ignored
Test outcomes are recorded in name-list-findings.json and the linked logs.
