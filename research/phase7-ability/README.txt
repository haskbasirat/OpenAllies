PHASE 7: ORIGINAL PERFORMER / ABILITY / PROJECTILE COMPONENTS

Status: component implementations and native differential tests completed.
Independent review and live runtime composition are separate gates. This does
not claim a playable attack loop, authored-unit integration, or a full match.

Rust surfaces
- ability::Ability: action-instance state, original sentinel/default fields.
- constructors::{create,init_create}: owned Performer/action/turret construction,
  original actor notification before mana copy, Properties refresh at InitCreate.
- performer::{start,begin,use_frame,timed_event,finish}: enabled/admission gates,
  target intention, animation choice, exact use/reload/cue scheduler boundaries.
- admission::{target_flags,target_matches,eligible,recharge_remaining,available,
  can_use}: original target classification/masks, source/target ownership, death,
  range/turret/resources, summon uniqueness, mana/recharge, immunity and healing.
- weights::{maximum,maximum_other}: original utility weights, including mixed
  extended and f32 stores. 57E670 is NOT an animation duration or cooldown.
- execution::{dispatch_from_performer,use_ability,use_area}: phase transitions,
  resource payment, source impulse, mana debit, self/target effects, live area
  iteration with native tie sorting, attenuation, projectile and area branches.
  DamageEffect calls the existing exact hit -> Body implementation directly.
- launch::{spawn_position,launch}: original creation-record defaults, facing
  rotation, point-target height adjustment and factory -> projectile handoff.
- projectile::{new,start,advance,impact}: owned embedded Ability, activation and
  scheduling, original speed/lifetime types, target prediction, interpolation,
  strictly-negative arrival, admission and exact effect delivery at impact.
- surface::{shape_projection,ground_position,hit_position}: native target-shape
  projection, preserved extended/store boundaries and mutable actor getters.
- payment::pay: owned aa-economy balances plus faction XP debit. Original signed
  integer XP conversion, stats45/46 ordering and post-stat resource cost copy.
  Costs must match the actual resource registry; no guessed cost-free fallback.
- fx::emit: precise owned effect instance handoff; actual manager/effect rendering
  remains required. See phase7-fx/README.txt for the narrow tested boundary.

Behavior deliberately preserved
- Initial recharge rejection uses <= 0 remaining on this native path.
- Starting consumes the animation selector even when retaining the old clip.
- Completion notification precedes writing Performer stage 4.
- Source impulse reads AbilityDefinition+44 (impact_impulse); it does not switch
  to the separately authored recoil_impulse+48 as an unrequested correction.
- Performer debit occurs after attempted delivery even when native use returns0.
- An active ability records the original attack-start time, not the use-frame
  clock. Embedded projectile hits execute phase2 and do not repeat phase0 cost.
- Unlimited areas use the live iterator. Limited areas prepend matches and use
  the native linked merge ordering: the right half wins equal-distance ties.
- Projectile time exactly zero moves to the endpoint and waits; only a negative
  extended subtraction triggers impact. Z/store/prediction boundaries retain
  native precision and ordering. Projectile activation failure does not turn a
  successful factory result into a null return in launch's caller.

Native evidence (numeric suites cover PC24/53/64; masked exceptions, nearest)
- native_performer.py: 102 sequences, 396 state/event transitions.
- native_admission.py: 186 cases, actual modifier and classification code.
- native_execution.py: 108 cases through actual DamageEffect and Body functions.
- native_projectile.py: 60 sequences, 285 flight/hit transitions through Body.
- native_constructors.py: 69 Performer/action/projectile constructor cases.
- native_weights.py: 72 unhooked decision utility cases.
- native_dispatch.py: 105 Performer delivery/mana/source impulse cases.
- native_launch.py: 102 creation record/spawn/factory/start boundary cases.
- native_surface.py: 1104 cases (552 shapes, 552 actor wrappers), including
  60 wrapper cases whose virtual getters mutate actor facing/position/ratio.
- native_payment.py: 366 real payment cases including callback mutation.
- native_fx.py: 52 sequences /208FX calls, copy-only Extended64 handoff.
Total: 2326 top-level cases. Transitions are reported separately, not added to
that count. Existing Phase6 fixtures remain separate evidence.

Original executable SHA256:
5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26
Accepted v14 executable SHA256:
3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1

Every producer executes original x86 with Unicorn and checks the visited
original/v14 instruction bytes. visited_windows are evidence of observed code,
not exhaustive branch coverage; some producers record intercepted leaf entry
bytes too. Full main-function byte ranges and assembly are separately pinned by
record_evidence.py. Native ABI/register assertions are oracle checks only: the
Rust implementation uses normal portable Rust APIs and does not claim x86 ABI.
Pointer values are normalized to documented fixture handles. Native integer,
f32 state and event ordering are compared. No GPU/game process is involved.

Required callbacks and honest limits
- The original engine has actor virtual calls and world services. Fixture hosts
  explicitly supply actor classification, generation lookup, allegiance, shape
  range, target hit surface, allocator, actor factory, scheduler, Animator/FX,
  economy payment and non-DamageEffect implementations. These are NOT silently
  accepted or defaulted by production. Production traits require real behavior
  or an error. Native hooks are listed at the producers' hook methods.
- Performer tests hook admission/delivery to isolate scheduler/animation logic.
  Admission tests run real native admission. Execution and projectile fixtures
  execute actual native admission where phase requires it and full damage/Body,
  while their external actor/world/economic/geometry services remain supplied.
  Launch tests inspect the call into 580BC0; projectile tests execute 580BC0.
  Their separate proofs are not a claim that a running world has been wired.
- Actual model animation resolution/RNG, turret turning,
  creation/publication, owned actor/Ability handles, cost binding, ability target
  ID-to-mask conversion and non-DamageEffect classes require runtime services.
  Surface4EEEE0/46E150 and payment4E2590/4CDD90 are now concrete modules.
  aa-spatial::range, aa-animation, aa-actors, aa-rules authored snapshots and
  aa-body::properties provide sibling components for those integrations, but
  their mere existence is not counted as completed composition here.
- Definitions and arena handles stay valid during a call; authored effect/list
  identities are stable while iterated. Arbitrary alias deletion, definition
  list replacement inside a callback, and recursive same-component dispatch
  are not supported by these APIs. Live area traversal is intentionally not
  replaced with a precomputed list. Required callbacks may fail; prior effects
  remain visible and are not advertised as rolled back.
- Numeric support is finite native inputs with explicit precision. Unmasked
  hardware exceptions, NaN/Infinity propagation, allocator addresses/capacities,
  invalid native pointers and hardware-exhaustion behavior are not emulated.
- A typed u32 handle is an identity, not a byte address or a RAM/VRAM ceiling.
  No new bandwidth, RAM, VRAM, disk-rate or artificial unit-count throttle was
  introduced. This does not remove native identity/protocol constraints.

Validation
cargo test -p aa-combat
cargo clippy -p aa-combat --all-targets -- -D warnings
22 Rust tests passed: 16 new Phase7 tests and 6 existing Phase6 tests. New failure
checks cover query cleanup plus retained prior payment, partial arena allocation,
and factory/activation failures after publication. See final-tests.log/final-clippy.log. Earlier logs remain immutable review inputs.
Native producers and fixtures are in phase7-ability and phase7-performer.
findings.json records current immutable source/fixture/dependency pins.
