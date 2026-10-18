# aa-body

Original-instruction-backed BodyComponent health, damage sequencing and corpse
initialization. This is not a completed combat world or death/ownership system.

Implemented native boundaries:

- `from_resolved`: body field initialization from561960 after the actor base
  constructor and derived maximum-health/defense getters supplied their values.
- `set_health`:561E50 clamp, unchanged-value early return, optional actor stance
  refresh, disabled-state request, all four health-condition comparisons, first
  matching health-effect trigger, and newly-full recovery callback.
- `set_health_and_maximum`:562200 with the original0.0001 maximum lower bound.
- `update_resolved_maximum`:561D40 with the actual intermediate binary32 ratio
  store and original below-one early return. This accepts the binary32 derived
  getter result of the null-kingdom path, not an unverified authored field.
- `heal_plain`:562BA0 ordinary-healer branch, including notification before
  adding health and the world-time read after setter callbacks.
- `check_death`:561C20 dispatch order. The host owns actor destruction semantics.
- `sink_fraction_f32`:5617E0 observed through a binary32 caller store; elapsed
  time, rot delay and actual sink duration are inputs. Progress is not capped1.
- `set_health_from_source`:561E50 linked setter dispatch and directional health
  effects. The host resolves the source and extended derived maximum at their
  native read points; XY is normalized/scaled while Z remains the raw difference.
- `damage::deal_damage`:5625C0 damage propagation, modifier multiplication,
  damage history, health subtraction, component responses, capture-health and
  death queue selection in original order. Every world service is required.
- `dying::start_dying`:562380 ability lifetime, corpse timestamp, terrain-adjusted
  sink duration, decay scheduling and allegiance-removal order.
- `credit::credit_kill`:4D57C0 credit eligibility, native classifications,
  ordinary/relative statistics, notifications, experience amount and reward
  callback order. Cached identities and post-callback rereads are preserved.
- `experience::actor_value`:4D01E0(actor,0) extended component-value aggregation.
- `experience_amounts`:571820 value ratio and its callers571960 (kill),
  5719E0 (capture),5718E0 (rout) and5718B0 (engagement); phase8 oracle
  research/phase8-combat/native-experience-amount-fixtures.json.
- `credit::World::add_statistic`: kingdom statistics are written only through
  this callback, bound to aa-economy's single `Statistics` (`add_score`).
- `experience::award`:4CE770 and578710 member/leader routing, including fresh
  regiment length/member reads and the current experience component after a
  leader callback. Actual promotion application571A60 remains required.

All scalar fields use exact binary32 bits. Every arithmetic operation chooses
an explicit `aa_numeric::Precision`; no process-wide FPU default is assumed.
The full original x86 routines execute in isolated fixtures with no game
process or file mutation. Host leaves are recorded seams, and selected fixtures
mutate state inside callbacks to test the original reread/order behavior.

`Host` callbacks are **synchronous**. A host may mutate body and environment.
Failures are nontransactional: prior health changes and callback side effects
remain, so a caller must not automatically retry. Events identify actual native
call boundaries; emitting them alone does not implement stance, actor-state,
effects, recovery, death abilities, scheduling, or removal.

The domain is finite values and positive current maximum. Plain helpers retain
their no-linked/no-attacker contract. The new source-aware setter and damage
entry dispatch linked bodies through required host operations. The plain healer
still rejects damage-converting healers. NaN/infinity, unsupported arithmetic,
missing host services and invalid callback state reject; there are no inferred
defaults or automatic retries.

The actual graph, organization/wall/ability effects, modifier context, capture
eligibility and transfer, terrain service, ownership, timers and death queues
must come from the world runtime. Upstream weapon/armor derivation is provided
separately by aa-combat. Actual reward application, promotions, resource loss
and save serialization remain world dependencies.
The damage operation is not a substitute for those services.

Phase6 source windows, callback contracts, arithmetic/store details and evidence:
`research/phase6-combat/README.txt`.
Credit and experience scope/evidence are under `research/phase6-credit` and
`research/phase6-experience` respectively. Connect a credit World's actor-value
callback to `experience::actor_value` using its actual component fields, and
its AwardExperience callback to `experience::award` using live actor/organization
data; these adapters must still provide real promotion/effect services.

Actor IDs are opaque native-width `u32` values here. `aa-actors` owns generation
and lifetime validation; convert explicitly using its `ActorId::raw()`.
