ActorData WhenEnter native binding, 2026-10-17
============================================
Original AA instructions are authority; no vendor SDK implementation was used.

464F99..464FE7 builds WhenEnter through42BF10 with a fresh local FXC reference
context (464F18..464F99), then validates every completed root's state+34.
4985E0 registers WhenEnter=0 and WhenLeave=1. Native464FF2..465007 passes
source ActorData+194 ENTER and destination+19C LEAVE into42B8E0, flag0.
That later copy creates persistent CreateVFX stop commands in LEAVE. This
change only certifies the ENTER compiled prefix; it does not load/synthesize
LEAVE or claim the entire ActorData loader. The previous reversed comment is
corrected, and Leave nodes no longer incorrectly make Enter appear nonempty.

4D0A00 walks head-first immutable Enter roots, evaluating vf20 before reading
state+34 only when active_only!=0. Sound6A71F0 is false. Create428360 requires
source VFXData+24==1 and only then node+64==0. Bounded494F30..494FA0 executes
the actual permanent/duration classifier: for finite typed duration, class1
iff permanent is true. No model-kind default is inferred. The native state
ordinal is the42-entry477FC0 descriptor index at Data+C, not source load order;
the existing paired native descriptor-table proof is hash-pinned separately.

An opaque non-Clone MatchedState retains the same immutable graph and exact
state/active arguments. No match reaches no queue, worker, time or global
position service. A match copies position before the mute guard, then uses
4293C0/4286E0, actual creator/thing, the real OS worker/application identities,
and lazy selected clock. submit uses the same real presentation manager and
flag0. Required B28114 global target-position owner is explicit; root's current
World adapter refuses a positive event until that owner is supplied. App
View+8C clock and attached View delivery/destruction also remain required.
The new helper does not manufacture zeros, another queue or execution role.

Proof fixtures, both original/v14 images and PC24/53/64:
- native-actor-state-fx.json:1728 paired rows,3456 executions,249 pins;
  actual4D0A00 scan plus4293C0 construction, predicate/read/event order, mute,
  state/no-match, Actor/Kingdom identity, app/worker clock, complete fields.
- native-vfx-active-source.json:144 pairs,288 executions,37 pins;
  finite permanent/duration/model classifier and source getter/store order.
- native-actor-source-boundaries.json:30 pairs,60 executions,85 pins;
  actual event enum, complete-list state42 validation and copy callsite args.
- native-actual-actor-state-fx.json:3 compile pairs plus252 scan pairs,
  510 executions,620 pins. Canonical tutorial02 saved2696 Farm source capture
  matches every compiled field and all42 states x2 active flags at allPC.
  Match replay is muted: it does not supply a pretend live B28114 owner.
Total2157 paired cases /4314 fresh executions; overlapping pins are not summed.

Native producers default to immutable comparison; --write creates their own
fixture on deliberate capture. Actual-source-input.json was created_new by
an ignored real-source Rust test; source hashes/provenance are retained.

Focused Rust3/3 passed. Same built binary's legacy/new FX10-test run passed9
and refused existing manual Created census only because AA_CAPTURE_FX_GRAPH
was omitted. That refusal is retained honestly in its log; a separate explicit
new-path census recheck is recorded separately. No production fallback changed.
The original source test's explicit capture requirement is intentional.

Pre-edit backups are under before/ for parent graph, graph tests, reentry and
World. load_creation_fx only changes validate/creator visibility to pub(crate);
no contemporaneous pre-visibility backup was captured for those two lines.
Root-owned load_world.rs remains actively edited elsewhere; only its
RequiredEffects state FX callback/Context::with_fx wiring belongs to this scope.
No manifest, dependency, release, original executable/save/profile edits.
