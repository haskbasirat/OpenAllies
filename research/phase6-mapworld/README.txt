PHASE 6: ORIGINAL MAP INPUTS AND LOADED OBJECT PREFIXES

aa-mapworld reads original RMP/TGCK version2 containers directly, not rounded
viewer JSON. Directory extents, duplicate names, overlaps, declared sizes,
decompressed stream termination and source hashes are checked. Chunk decoding
takes an explicit caller allocation budget; it adds no gameplay RAM cap. Raw
chunks and zlib streams are retained under exact source identities. Additional
unimplemented chunks remain available through MapArchive::decode_chunk.

Public typed loader:
  MapArchive::from_bytes(Vec<u8>)
  MapArchive::decode_chunk(name, max_decoded_bytes)
  WorldInventory::decode(&archive, max_chunk_bytes)

WorldInventory currently provides:
- GWORLD version<=28 serialized RNG state and exact raw field/time bits with
  native version gates. Unknown fields use their native object offset names;
  remaining bytes are explicit, not discarded or assigned invented meanings.
- IDMGR version0 serialized cursor, highest slot, capacity65536 and generation
  prefix. Native reads highest_slot u16 values, not highest_slot+1. Missing tail
  generations retain reset-zero. restore_empty_registry delegates to the exact
  aa-actors pre-registration metadata constructor. AddID restoration must not
  reseed generations as normal actor creation does.
- DYNAMIC_THINGS_INDEX version0 count/record lengths, exact source order and
  DYNAMIC_THINGS version0 header word (its second word is NOT assumed count).
- Every bounded record has UTF16 definition ID, class, serialized identity,
  GThing version0..2 raw name/position/facing/flags/word1C, GObject version0..5
  scale multiplier/variant/associated handle/nested-count prefix when applicable.
- All recorded GActor class3 prefixes provide version<=14, owner handle and
  serialized state bits. Native owner resolution uses the saved GTID table only
  when evaluated Thing flags contain bit23; handles are not side/player indices.
  State version<9 ORs only the low word, >=9 replaces both words before subsequent
  unported transient-state clearing and component deserialization.
- Opaque subtype and nested-object tails retain exact bytes and source spans.
  Unsupported bounded records are represented explicitly without corrupting the
  following indexed record. No inspector health text is used as runtime health.
- MAP_HEADER supplies exact world extents, setup rules and authored player metadata.
- TEAMS/KINGDOMS now have complete typed source wire decoding for all original
  maps; see research/phase6-kingdoms/README.txt. Source owner handles are bound to
  exact saved Kingdom IDs, including minor kingdoms. Runtime ownership still
  requires the final definition-derived bit23 gate and actual Kingdom restoration.
- Required-object chunks remain source-bound opaque initialization inputs.
  Unsupported extended subtype schemas retain the full raw chunk plus an error.
  A complete live actor/world is NOT claimed by this reader.

Native evidence:
  4FC340 indexed-record loop consumes original source order and seeks by saved
    record length after a failed/unknown Thing, not an inferred placement count.
  4FAE80 reads definition string/class/saved ID, checks ID generation, constructs
    category-specific loaded object, then virtual+D8 Load. It does not use the
    normal 4EF2A0 new-object/RNG construction path.
  4EF160 GThing::Load: original version helper8589B0, CString, word1C, position,
    facing, serializedflags; merges (old&FC0000FF)|definition|savedLowByte; v1
    consumes an extra legacy word. Raw input decoding does not simulate the
    native >1000-character name cleanup or diagnostics.
  4EC7A0 GObject::Load: base, version<=5, scale, version-gated variants, associated
    handle, legacy16-bit value, nested-count/data. Native scale/variant evaluation
    requires typed definition values/RNG and is separate from this byte reader.
  A273F0+D8=4D8CF0 actor Load; owner/state prefix is executed through4D8E1F.
  4F0230 IDMGR load, including capacity65536 comparison, executes in the oracle.
  4FC6C6..4FC7DF world-prefix field reads execute for versions0..28; earlier UI
    and queue cleanup are explicit outside this fixture segment.

Validation completed:
- 48 original x86 full GThing Load cases, including all supported versions,
  arbitrary raw floating bits, UTF16 units, flags and legacy-word consumption.
- 8 original ID-manager cases (6 accepted including highest65535, 2 rejected).
- 29 original GWORLD prefix cases, covering every version gate.
- 12 original GActor owner/state prefix cases for versions0/8/9/14, owner flag
  gate, null/present owner, and old world-version14 upgrade behavior.
- The complete current aa-mapworld suite passes27 tests, including native header,
  actor payload, recruitment, teams, kingdoms, source ownership and terrain tests.
  Strict all-target aa-mapworld Clippy passes.
- All80 original maps /225851 indexed placements decode without unsupported
  prefixes or generation mismatches. Exact positions, definition IDs, saved IDs
  and record extents match the previous independently extracted source records.
- Native classes: 131043 dynamic doodads9,82799 objects2,11701 actors3,
  196 cameras7,112 areas6. All source GObject nested counts are zero here.
  Crete5633 records include314 actors and exact owner handles0/1/2/12344.

Corpus/report hashes:
  corpus-audit.json
    fd099714db403ef0acdd3f95050cbb1db3aa43bc31760a4532a40b539267a925
  native-load-fixtures.json
    a7b8faf17c45802ce712a06859cc0684b3b2cb67c77242e00c1c4f4e8b03a5de

Commands:
  python research/phase6-mapworld/native_load_oracle.py
  cargo run -p aa-mapworld --example audit_maps
  $env:AA_GAME_ROOT="E:\Games\Axis and Alies"
  cargo test -p aa-mapworld -- --include-ignored
  cargo clippy -p aa-mapworld --all-targets -- -D warnings

Original/recovered assets, native game installation, saves and old releases are
read-only. Header/extents decoding is independently developed in header.rs and
research/phase6-mapheader; actual object constructors/definition evaluation are
owned by aa-actors/aa-rules rather than duplicated in this loader.
