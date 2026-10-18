# ByroRedux and Kohan reference continuation, 2026-10-16

This source checkpoint follows `SKIRMISH-BUILD-OPTIONS-20261016.txt`. Skirmish
remains first, multiplayer second, campaign last. It does not update
Development-09 or establish a complete battle, release, completion percentage,
or delivery date. Final checks and source hashes are in
`qa-output/byro-kohan-20261016/acceptance.json`.

## Actual ByroRedux use

[ByroRedux](https://github.com/matiaszanolli/ByroRedux) was pinned at
`fa7591ca28b935ae411ecb1d1378843abdb77a5e`. Selected original source, notices,
URLs, Git blob IDs and SHA-256 hashes are under
`../reconstruction/references/byroredux-20261016`. Its raw Rust NIF parser and
core crate were built in a separate minimal workspace on this Windows PC;
all 298 selected Rust source files remain unchanged. Main workspace manifests
and lockfile were not changed. The upstream engine, renderer, setup scripts,
Bethesda gameplay and import defaults were not adopted.

We actually ran that parser on 16 representative original AA files, including
the reported British corps and oil-depot models, pack/unpack/death variants,
infantry, weather, water and effects. All original inputs remain unchanged.
Fifteen models finish with clean parser telemetry. Fourteen also pass the
separate strict block/root-footer/EOF diagnostic; the legacy 4.2.2 water file
does not run that strict footer diagnostic. `AirplaneDeath.NIF` stops at
`NiVisData`, parsing only 16 of 231 blocks. This is not a whole-corpus success:
the header/hash inventory covers 1,900 files but only these 16 were parsed.

The two readers agree exactly on all 95 authored alpha-property flags and
thresholds in the 15 completed models. This is the concrete ByroRedux-guided
production change: newly converted materials retain optional
`alphaPropertyState` with the original flags and byte threshold. A supplied
value that disagrees with the audited native-render-state sidecar is refused.
Old converted JSON and models without an authored alpha property remain
supported. No existing assets were regenerated. The native render-state
sidecar remains authoritative; this change does not change shaders, rendering
defaults or prove that a reported transparency problem is visibly fixed.

The broader semantic comparison retains its raw disagreements. The 174 matrix
differences are storage transposes between the two readers; 53 triangle-strip
differences are cyclic vertex ordering with the same oriented triangles and
order. Both classifications were checked against actual serialized data.
They are representation differences, not proof of incorrect AA transforms or
native renderer parity. Full normal/color arrays, texture binding structure,
controller keys and animation playback were not compared. Scene-root inference
and upstream transform sanitization remain adapter concerns.

The project declares MIT in its README and root Cargo metadata; its full pinned
tree has no standalone root LICENSE file. Those declarations and third-party
notices are preserved. This is a factual provenance boundary, not a claim of
verified vendor-SDK rights or a reason to stop private compatibility work.
No vendor Gamebryo implementation was copied into the rewrite. Production
metadata uses the existing reader; no ByroRedux decoder was copied into the
main runtime. The isolated parser is now a reproducible independent diagnostic.

The full asset test suite passes 19 tests with one pre-existing ignored test.
Both client alpha-conflict tests pass in the full release client test build.
Independent review also replays the 15-model parser sample, three new asset
tests and in-memory conversion of the same original model inputs.

See `research/byroredux-20261016/`, especially the corpus report, alpha
cross-check, representation-difference analysis, retention evidence and
independent review. The research subprocess memory/time bounds protect this
test harness and are not game memory or bandwidth limits.

## Kohan II leads

The exact [official modernization announcement](https://store.steampowered.com/news/app/97130/view/672876020537229477)
says the update was rebuilt from original source. This supports a source-access
lead; it does not supply a public TimeGate engine repository, SDK, AA rights or
reuse license. No public original game-source release was found in the bounded
official-site and GitHub searches. A precise local inquiry is saved as
`research/kohan-leads-20261016/official/UNSENT-SOURCE-REQUEST.txt`; nothing was sent.

The CC0 [rwd_edit](https://github.com/csernib/rwd_edit) source independently
corroborates archive layout. Its author tested Kohan II, not AA. Our independent
read-only audit matches all 14,748 metadata rows in ten installed AA archives
against the existing Rust reader, with unchanged archive hashes. Eleven RWD
tests pass. A public footer-field naming trap is documented; no production
archive-reader change is justified and the upstream C++ was not executed.

[Battleborn](https://github.com/Ensrick/kohan2-battleborn) contains mod data and
research rather than original engine implementation. Its animation-startup
lead was checked against native AA. Kohan-specific rules were not imported.

The MIT [Kohan widescreen patch](https://github.com/Ensrick/kohan2-widescreen)
led to a unique AA instruction signature. We then ported the actual AA camera
projection subroutine, not the Kohan patch behavior. The bounded coordinator
preserves its explicit precision, temporary rounding, ordered shared writes,
required renderer flag and cache reads. It passes 1,536 original/v14 native
cases across three precision modes and independent review. It stops before
actual D3D device publication and does not change client FOV, zoom, DPI, picking
or camera defaults. The downloaded patch DLL was not used.

## Battle source progress and remaining gates

The saved owner action rebuild now passes the actual held Organization and
Actor into the existing native coordinator. Ninety native original/v14 cases
across all precision modes preserve mask clears, contributor order and guarded
required publication errors. The tutorial checkpoint advances from owner
`4D1350` to required member-added callback `5784C0`, still actor 4099 at source
ordinal 434. The eight historical membership-prefix checks remain intact.

Final root checks pass the all-target workspace release check, six Actor
storage/Organization tests and 55 selected runtime tests. The 80-map managed
mode1/PC53/diagnostic-clock0 census still has 20 record-clean maps and 796
failed records: 577 require completed `ActorData+1E4`, while 219 now reach
`5784C0`. There are zero map-load errors and zero complete immediate Init,
publication or battle-ready maps. The census elapsed time is not a loading or
performance benchmark. Definition parsers and metagame code did not change;
their preceding full compatibility results remain historical, not rerun here.

The new borrowed combat-mean coordinator preserves actual manager/list reads,
lazy getters, progressive mean writes, assertion state, logging and failures.
It passes 210 native cases, including an explicitly limited ten-definition
source subset. It does not create completed means for all 1,028 definitions,
invent a source FP/profile context, bind the concrete shared source graph or
publish completed `ActorData+1E4`.

The next gates remain actual member/property effects, completed shared combat
source values, real placement/world services, immediate per-record Init and
PostCreate, registration/publication, mission/SAI activation and ordered live
ticks. Record-clean maps are still diagnostic. Multiplayer and real two-PC,
separate-Internet acceptance remain untested and unavailable on this machine.
Source checks are not a performance, visual-parity or original-save gameplay
acceptance test. No reliable whole-game percentage or finish date is established.
