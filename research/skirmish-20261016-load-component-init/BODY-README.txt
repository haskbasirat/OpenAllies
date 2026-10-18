Concrete Body562290 InitExisting prefix
=======================================

NEW aa-runtime::load_body_init_world binds the actual saved Body allocation in
the held Record. It does not fabricate health/FX/linked-body services.

The original562290 instructions:
  - read current owner Actor+100 bit21; set means true immediately;
  - otherwise compare Body health+10 and maximum+14, retaining that comparison;
  - read actual owner Construction+9C;565070 rereads its owner's state bit29
    and returns actual Construction+28 only on that path;
  - a nonnull target uses pure4CF110 (current Properties+1C context) then enters
    required561D40 maximum/modifier/linked-body service;
  - if the earlier comparison differed, reread health/max, store max to health,
    then enter561E50(old_health,token0,notify1,source0);
  - otherwise return true. Body AfterInit virtual+14 is4B2830 (ret).

The authoritative Actor+100 is components.state.low. No second raw actor.words
copy is read. Source/factory checks use actual Catalog Bank source precision and
canonical provenance/category/flags/component mask. The real Body owner and
actual Actor constructor layout are checked; no source geometry is inferred
from a renderer. The same Load shell/Body Box is mutated, without copies kept.

Healthy and bit21-skip paths complete this component step. Active Construction
target stops at required561D40 before any Body mutation. Damaged health stops at
required561E50 AFTER its actual health=max store; that store survives the error.
The caller must not blindly retry the failed operation: the changed state would
otherwise hide the required health callback. None of this certifies whole
Actor Init, object placement, source replacement, world startup or a battle.

Native proof
------------
native_body_prefix.py runs original/v14 separately at all3 PCs on216 explicit
finite field cases:138 complete,18 required561D40,60 required561E50. There are
432 fresh image instances and70 address-byte pins (68 executed addresses and
2 intercepted required-child entry pins). Native565070 and4CF110 execute;
required children never return fabricated success. The fixture checks the exact
health/max state and store prefix. The test host's Body/Construction fields are
explicit valid component probes; they are not a whole loaded native allocation.

Actual saved-state evidence
---------------------------
actual-healthy-body.json is the first eligible actual healthy Body Actor in the
stopped (2v2)Kursk diagnostic, registry order: GTID11542, definition
eastern_europe_city_center02_small, class3 BUILDING, health500/max500,
stateLow0x08000000. It is a neutral building, not a military unit. Source PC is
the actual Bank's Double53; live component calls test all3 PCs. Source and full
body state are captured. Production InitExisting and AfterInit preserve that
same real allocation and complete shell on these healthy paths. This does not
claim that the diagnostic record's entire Load/Init/world lifecycle is ready.

Reproduce (leave AA_BODY_INIT_CAPTURE unset for replay):
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  $env:CARGO_TARGET_DIR='target/s-state-binding-review'
  cargo test -p aa-runtime --lib load_body_init_world::tests --release --locked --offline -j4 -- --include-ignored
  python research/skirmish-20261016-load-component-init/native_body_prefix.py

Native command is immutable by default. --write generates before freeze.
AA_BODY_INIT_CAPTURE is an author-only absolute create-new capture destination.

Open effects
------------
561D40 requires the current modifier context, source45D730 maximum getter and
actual linked Body ownership/effects.561E50 requires actual flags, states, world
health callbacks, components/FX/queues and linked actors. Their absence remains
an explicit reached error. Military-unit/whole-battle integration and other
component InitExisting/AfterInit effects are separate work.

Frozen hashes and validation/review results: body-findings.json.
