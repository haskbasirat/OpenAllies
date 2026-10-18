Independent review of ActorData+2D8 source binding -- 2026-10-16

Accepted within the documented source-projection domain; no blocking findings.
No production file was edited by this review.

Fresh replay of the frozen author native harness reproduced all 72 rows:
12 cases x original/v14 x PC24/53/64. The 100 byte/address pins contain
92 executed instruction addresses and 8 intercepted native leaf entries.
This is a replay of the reviewed author oracle, not a second oracle design.
The typed getters, Property Find, allocation, CString helpers and exceptions
are explicit seams. Constructor/call-site evidence is static inspection.

The release/locked/offline aa-rules native_member_property_data test target
passed all 4 tests, including the actual-source test. It wrote only the new
actual-source-replay.json in this review directory. A separate structural
comparison found it identical to the frozen author actual-source-data.json:
1,084 canonical ActorData definitions per precision, zero nonempty +2D8 lists.
The synthetic rows exercise nonempty lists and partial failures. The real
source census is not a new native parser run or an observed playable battle.

Reviewed source facts:
- 4C4CF0 reads property_IDS, calls Property Find immediately, and prepends only
  after a successful lookup; prior prepends survive a late throw. Duplicates
  remain shared handles and the original tail is retained.
- The source producer supplies the canonical owner's source-file index and
  byte offset to the existing NativeLayoutData registry. Later registration
  and Translate roots are therefore invisible to this immediate lookup.
- The actor root is selected by native registration provenance, avoiding a
  second Thing translation. Other classes/unordered catalogs are refused.
- Catalog::member_definition_properties supplies its actual cached layout
  arena at the explicit precision; the pointer owner guard prevents replacing
  the ActorCatalog after construction. PropertyRef handle identity is shared
  with the existing property consumers.

Scope/preconditions retained:
- The public aa-rules API documents that the caller must supply the same
  catalog's layout arena; it does not independently authenticate a foreign
  NativeLayoutData argument. The reviewed runtime path supplies the owned one.
- The existing runtime canonical-identity guard can refuse an identity that
  a later Thing Translate shadows; it does not silently translate twice.
- Native source parsing/registry internals and unrelated ActorData loading
  were not newly proved here. Malformed template projections may be refused
  before this small list operation. Successful allocation is the proven domain.
- The completed API returns an error, rather than certifying a partial list;
  the public load_list core exposes native partial prefix effects explicitly.
- No property effect/application, saved battle completion, visual improvement
  or playable initialization claim follows from this review.

See independent-review.json for counts, source/artifact SHA256 pins and
commands. All reviewer commands have completed.
