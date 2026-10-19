Aspect extra (loaded-Model) textures: 46AE20, 46AF70, 5A79F0, 469BB0/469C50 +48

Gap (package 9 of gap-plan): data_aspect.rs request_extra refused 46AE20,
data_aspect_release.rs refused the 469BB0/469C50 +48 walk, 46AF70 had no
Rust site, and the 5A79F0 search-path miss branch was an explicit stop.

Native facts (disassembly + oracle):
- 46AEC0 (already ported) calls 46AE20 when Model28 flags+8 bit0 is set,
  with (+54 ? delta : +18 + delta, mode).
- 46AE20(delta, mode): n = 5A4ED0 = Model+7C. If n != +4C, 46ADD0 resizes
  the +48 {data, +4C count, +50 capacity} vector and EVERY element i < n is
  resolved: +0 = 46A180(5A5590(i) = Model+80[i].full, flag 0), +4 =
  5A55A0(i) raw name. Then vf10(delta, mode) on +0 for every i < n (also
  when n == +4C, where nothing is re-resolved). +54 = 1 last, also for n 0.
  A NULL +0 (empty Model texture name) faults at vf10.
- 46ADD0(n): cap*2 < n -> 46A450(n); cap < n -> 46A450(2*cap); no data ->
  46A450(cap); +4C = n. 46A450 raises an existing vector to at least 4,
  copies old +0 verbatim and assigns +4; 469CF0 constructs only +4, so new
  +0 words stay operator-new garbage. Constructors: data 0, count 0, cap 1.
- 46A180 flag 0: renames (+3C list) then 4763C0 with GameData+4. 48D660 is
  the only writer of GameData+4 (85A550 of every 48D970 file at its start;
  static scans of all [0xB36530] users and GameData methods found no other
  store), so in play it is the directory of the last index file:
  Data-root translate.tgi -> "". A non-empty folder would be prefixed
  (oracle row "Weapons").
- 5A79F0(name, owner): 5A77E0(name,1); if a miss and owner non-NULL, for
  each non-empty path (ObjectData+B4 list, 425E80 appends in source order):
  '.'-relative -> 85ABB0(85ABB0(85A550(name), path), 859B70(name)) + 85A030;
  otherwise 85ABB0(path, 859B70(name)); 5A77E0(candidate,1). Finally
  5A77E0(name, 0).
- 46AF70: Model vf18 when flags+8 bit0 clear; 46AE20(+18, 1) when +54 is 0;
  855D30 "m_extra_textures_requested" (AspectData.cpp 0x1C9) when the B3A747
  latch is 0 and +54 still 0 (unreachable after a returning 46AE20).
- 469BB0/469C50 (kernels already ported, research/skirmish-20261019-data-
  release-ready) read Model+7C and +48[i]+0. Model+7C is written only by
  5A7DB0 (0), the 5A99C0 load from the Model's immutable source and the
  uncalled operator= 5A91E0; unload vf40 5A6560 / vf54 5A7CF0 keep it.

Oracle: native_aspect_extra_textures.py -> native-aspect-extra-textures.json
(--write authors; default replays and asserts equality). Original and v14 x
Single24/Double53/Extended64, 204 machines, all equal, 1657 pinned bytes.
- search: 13 real 5A79F0 rows (direct hit, first/second path hit, all miss
  -> flags 0, empty path skipped, NULL owner, empty list, no directory,
  backslash, trailing slash, drive colon, '.'-relative, final-only hit).
- extra: 9 sequences of real 46AE20 on one Aspect (gbr_brigade_engineer
  names, same-count repeat, 469FE0-style reset then 5/1/0 textures, growth
  1 -> 4 -> 9, first allocations 2 and 3, zero first, count change without
  reset, non-empty GameData+4, rename list, NULL Texture vf10 fault).
- block: 12 real 46AF70 cases (loaded x +54 x latch x seam sets +54).
Seams: CString manager (rename oracle helper), heap (poisoned), 5A77E0,
85A030 identity, 5A79F0 (46AE20 rows), Texture vf10, Model vf18, 46AE20 and
855D30 (46AF70 rows), 405D60 %s/%% and 424C10 trim for 4763C0.

Rust: crates/aa-runtime/src/data_aspect_texture.rs (5A79F0, 85A550/859B70
split, GameData+4, Extras vector with 46ADD0/46A450, 46AE20, 46AF70),
data_aspect.rs (Row +48/+50 and the read Model+7C, skin/glow now use the
full 5A79F0, Bound::request_extra = 46AE20, request_aspect_with_texture_
policy, block_aspect_textures_46af70), data_aspect_release.rs (extra_count,
extra). Tests: data_aspect_texture_tests.rs replays every row, plus an
ignored AA_GAME_ROOT lab (actual Aspect, GameData+4, actual Texture lookups).

Not proven / open:
- '.'-relative TextureSearchPath (85A030) stops explicitly; shipped data
  has only '/'-absolute paths.
- 5A79F0 with an empty name stays refused (texture_request_name).
- 469BB0/469C50 use the Model+7C value 46AE20 read for the SAME Model, not a
  live read: ResourceRelease (data_release_kernels.rs) exposes no Model
  field access. A live read needs that trait to gain a Model+7C accessor.
- request_aspect keeps the required Texture policy; callers with an actual
  policy use request_aspect_with_texture_policy. 45A780 (package 3) must
  call block_aspect_textures_46af70.
- B3A747 latch is not stored; the assertion path is an explicit error.
- The real-data lab uses the two texture names of the shipped
  GBRBrigadeEngineer.nif (byte scan); it does not decode the NIF.
