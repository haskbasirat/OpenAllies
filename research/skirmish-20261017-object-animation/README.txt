Sequence and Object/Actor animation source projection â€” 2026-10-17

Production
  aa-runtime/resource_sequence.rs: actual NiSequenceResource5A5FB0 constructor
  and typed5A80D0 factory through the existing one Resource/Task manager.
  object_animation_set.rs/object_animation_core.rs: same-Data Set49BF20,
  ordered49DA40/49D900 fields/appends and49B7E0 request loop.
  actor_animation_source.rs/actor_animation_core.rs: same-Data/Layout source
  sets, base then explicit aspect then reparsed fallback,463B50/463440/461F70
  fields,461190 finalization,45EA60 requests. Each Animation owns one shared
  FX compiler Session, moved once into Graph. No timing DTO substitution.

Native evidence (original and accepted v14, all PC24/53/64)
  native_sequence.py:18 constructors,24 typed lookup cases,209 executed pins.
  9D0710 registration is raw metadata only; its registry initializer was NOT
  executed. Typed Find5AD430/5AEA90 is an explicit transport returning the
  same actually constructed allocation. No wrong-class decoder is asserted.
  native_object_animation.py:120 source cases,54 request cases,1169 executed
  pins. Actual49BF20/49C020/49DA40/49D900/497220/405D60/49B7E0 bodies run.
  Typed node getters, source-relative path and Sequence factory are explicit
  children. Positive CreateVFX/PlaySFX cut off at their source-count read.
  native_actor_animation.py:300 source cases +24 seed calls,126 finalizer
  cases,72 request cases,2472 executed pins. Actual499FF0 registers seven
  canonical prefixes and12 aliases. Actual463B50/463440/4632D0/461F70,
  461190/45EA60 run. Empty fixed FX lists use real42BF10/428A30. Positive
  timed FX stops at actual42B990. This source fixture does not certify the
  concrete positive Graph compiler, Sound resource or whole ActorData loader.

Notable exact behavior
  Object duplicate IDS stops before KF/factory. A NULL Sequence skips Object
  Animation allocation. Object appends occur AFTER field/child loading.
  Actor NULL Sequence still produces an Animation. Actor overall/typed list
  appends occur BEFORE461F70. Inherited matching IDS skips all later reads;
  ordinary duplicate IDS is retained. Fallback reparses source and creates
  distinct allocations. Canonical type lookup uses prefix before alias.
  Actor timed cues prepend before reading time; +/-zero time diagnoses,
  while negative nonzero time reaches the following FX compiler.
  Required kinds are idle/fidget/die. Move thresholds follow authored order;
  they are written before the final death-threshold diagnostic.
  Object requests reread current B282F0 per Resource. Actor requests pass
  literal priority/mode0 and visit fixed then linked timed FX lists.

Verification
  Fresh read-only replays of all three native producers PASS.
  exact_core_tests.rs includes the exact two production cores;5/5 tests PASS.
  No copied kernel. Generic test services are explicit typed fixture inputs.
  Client production check08 PASS40.58s includes concrete source and shared
  FX Session. Final runtime production check09 PASS22.11s includes the common
  memory0C/10/Sound constructor additions and readonly source Graph visitor.
  The latest Actor-global client factory loan is a separate successor edit. Sequence runtime unit constructor test is saved but NOT
  run: root runtime50 captured zero such tests, explicitly not a pass.
  Concrete source owner/provenance guards are compiled; a whole actual source
  Data preparation/ActorData load replay has NOT been claimed.

Bounds/open work
  The native lookup/allocator/CString/TGI/formatter seams are in each producer.
  FP status, SEH/allocator failure/unwind behavior and nonfinite source fields
  are outside this finite source projection. ASCII strings/ordinary printf
  %s/%% are admitted; other formatter forms remain required errors.
  Additional runtime alias writers, positive death filter registry lookup,
  Object CreateVFX/PlaySFX children, Sequence5A8190 decode and actual positive
  FX request effects remain explicit services/errors. Source loads must run
  in the actual pre-World source interval; late preparation is not certified.
  Whole Data/source completion, renderer upload, animation playback, VWorldAD,
  resource task barrier and playable battle readiness are NOT inferred.

Removed allocation policies
  Object overall initial16 and eight typed initial2 capacities; Actor initial
  overall14 and seven typed initial2 capacities use growing Vec storage.
  The seven/eight animation types remain native registry rules, not caps.

Related independent owners
  SavedLeader owns fx_graph_animation_source and prepared Aspect Layout;
  State owns Resource/Task/Texture/Model backend; Film owns Sound constructor.
  Those files' current hashes are context, not blanket independent acceptance.

Small replay
  rustc --edition=2024 --test exact_core_tests.rs with cached aa_numeric and
  serde_json rlibs in target/s-builder-source-review/release/deps.
  Native producers default to immutable comparison; --write is author-only.
