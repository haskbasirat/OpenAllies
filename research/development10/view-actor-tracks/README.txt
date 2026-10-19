VActor Track array (451F90 + 42D7B0 + 42D830) and the 464BD0 TrackInfo entries
================================================================================

Native functions (AA.exe image base 0x400000; every executed byte equal in the
original and the accepted v14 image):

  451F90  VActor Init. After 457A40 and the pose copies, when ActorData vf38
          (4F0500, returns 1) and 45E700 (ActorData+260) > 0:
            +148 = 45E700 (second call), stored BEFORE the allocation;
            6F7810(count*0x74 + 4), leading dword = count;
            96D18D eh_vector_constructor_iterator(arr+4, 0x74, count,
                   42D7B0, 4B2830);
            +144 = arr+4, or NULL when 6F7810 returned NULL;
            for i < +148 (re-read each pass, +144 re-read each pass):
                42D830(this = +144 + i*0x74, owner = this VActor,
                       info = 45E710(i) = ActorData+25C + i*0x24);
            then 85D840(UIWorld+D4) into +14C (unchanged, ported before).
  42D7B0  Track constructor (NOT in the Ghidra export; decoded from bytes):
          +00 +04 = 0, byte +08 = 0, +0C..+30 = 0, byte +34 = 0,
          +54/+58/+5C = B43CF8/B43CFC/B43D00, +60/+64/+68 = the same three
          words read again, +6C = +38 = BF800000 (-1.0f), +70 = 0.
          +09..+0B, +35..+37 and +3C..+53 are never written.
  4B2830  empty RET (the iterator's destructor).
  42D830  Track Init(owner, info): +00 = owner, +04 = info+00, byte +08 =
          info byte +04, +0C = info+08, +10 = info+0C, +20/+24 = info+18/+1C,
          +30 = info+20, +14/+18 = info+10/+14, +1C = 0, then
          fld +20 * 9FBDB8 (0.5f), fld +24 * 0.5f, fstp +28, fstp +2C,
          byte +34 = 1.
  45E700  return ActorData+260.     45E710  return ActorData+25C + i*0x24.
  85D840  UIWorld+D4 LCG (unchanged).
  464BD0  ActorData::LoadData MoverComponent branch 465DA4..465F74, one entry
          per TrackInfo child, appended (+260++, 45E880 growth) before reads:
            5B54D0 track_type, 5B54D0 track_texture,
            405D60(format=track_texture, ActorData+B0 resource_prefix),
            4763C0 (GameData-relative), +00 = 497220 lookup (-1 if absent),
            +0C = 5A77E0(path, 0), +04 byte = 5B5380 track_texture_mirrored,
            +08 = 5B4AE0 Float track_placement_pcg, +10/+14 = 5B4D80
            track_offset, +18/+1C = 5B4D80 track_size, +20 = 5B4AE0 Float
            fade_sec, then +00 < 0 -> 410C40 "Unknown track type '%s' on
            object '%s'" (throws). So +08 is track_placement_pcg and +20 is
            fade_sec. Entry bytes +05..+07 are never written.
          4624B0 ctor: +25C = one 4BDDB0 entry, +260 = 0, +264 = 1.
  4BDFF0  static init 9CD190 -> 4BDFF0(this=B3B53C, 1) registers DISCRETE=0
          then CONTINUOUS=1 in the B3B54C table (4BDEE0, 7 buckets).
  497220  405030 hash (folds a-z) + _wcsicmp; the only setlocale calls
          (7F1965/7F1A39) change LC_NUMERIC, so LC_CTYPE stays C (A-Z fold).

Oracles (Unicorn, research/phase8-ai/native_machine.py, page-0 guarded):

  native_actor_tracks.py -> native-actor-tracks.json (18 rows = 6 cases x
    PC24/53/64). Seams: 457A40 (al=1), 6F7810 (fresh fill-patterned block or
    NULL). Each case runs with two fill patterns; differing bytes = unwritten.
    Cases: 1, 2, 3 and 7 Tracks, special binary32 sizes (subnormal rounding
    to even, -0, inf, quiet and signalling NaN), and the NULL allocation
    (native fault: 42D834 writes [0]). Results are identical at all PCs.
    python native_actor_tracks.py           (verify; --write regenerates)
  native_track_info.py -> native-track-info.json (21 rows = 7 cases x 3 PCs).
    Runs 465DA2..465F79 with real 45E880/96D18D/96BE33/4BDDB0/4BDDA0,
    405030, 497220 and _wcsicmp over the B3B54C table laid out as 4BDFF0
    builds it; getter/CString/path/texture/allocation/410C40 and _getptd are
    recorded seams. Cases: biped pair, five case-folded types (growth 1->4->8),
    unknown type after all stores, empty and trailing-space types, %s prefix
    format with a NULL texture, no TrackInfo.
    python native_track_info.py             (verify; --write regenerates)

Rust (replaying every fixture row):

  crates/aa-runtime/src/data_actor_track_info.rs (+ _tests.rs):
    TrackInfo<T>, read_entry (464BD0 order), track_type_497220, the
    production reader (5A77E0 through Resources in the pre-World source
    prepass) and State::prepare_actor_track_infos / actor_view_track_info_45e710.
  crates/aa-client/src/view_actor_tracks.rs (+ _tests.rs):
    Track<T> (42D7B0 construct, 42D830 initialize, half_9fbdb8), Array<T>
    (allocation + iterator with retained prefix on failure).
  Tests:
    cargo test -p aa-runtime --release --locked --offline -- data_actor_track_info
    cargo test -p aa-client --release --locked --offline -- view_actor_tracks
    AA_GAME_ROOT="E:\Games\Axis and Alies" cargo test -p aa-runtime --release
      --locked --offline -- --ignored actual_v14_trackinfo_entries_prepare_with_their_textures

Removed caps: none of gameplay. The 32-bit size computation count*0x74+4
(6F7810) and the 45E880 capacity doubling are replaced by growable Vecs.

Not reproduced / explicit errors:
  * 6F7810 returning NULL: native stores +144 = NULL and 42D830 then writes
    through NULL (fault at 42D834); Rust allocation failure is an error.
  * 5A77E0 with an empty TrackInfo texture name (unproven): error.
  * A '%' in track_texture with no View resource_prefix source: error.
  * Non-finite track_placement_pcg / fade_sec getter values: error.
  * A track_type with an embedded NUL: error.
