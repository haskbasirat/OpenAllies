PHASE 6: BODY DAMAGE AND CORPSE INITIALIZATION
============================================

The production code is aa-body::damage, aa-body::dying and the new
Body::set_health_from_source API. Original AA.exe and accepted v14 remain
unchanged. This is a component implementation, not a complete combat world.

Source identity
---------------
Original SHA256: 5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26
Accepted v14 SHA256: 3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1
inspect_native.py binds full relevant functions and constants to both images.
Recorded visited windows include intercepted seam entry addresses. They pin
bytes; they do not claim execution behind the intercepted calls.

DealDamage 5625C0
----------------
DamageRecord preserves seven words: source GTID, allegiance GTID, damage-type
identity, binary32 amount, binary32 time, event kind and reason. World registries
must resolve the actual identities; there is no name/index substitution.

The implementation dispatches valid linked Body damage before checking amount
or the local destructed flag. The linked call forces neighbor propagation true.
Ordinary work requires positive incoming amount and a non-destructed local actor.

Initial wall propagation uses the incoming last bool, unless a valid weapon
definition has positive +38 or +4C. A valid wall owner/segment, nonzero +4AC
factor and nonnegative segment index produce a scaled copy of the damage record.
Its amount is stored binary32. Forward then backward neighbor queries occur
separately with recursion flag false; first propagation can change the second
lookup. Graph dispatch is mandatory, not silently omitted or replaced by a loop.

At 5627B8 the original stack slot that held the propagation bool is overwritten
by actor virtual+140's hostility result. Every subsequent hostility-dependent
branch uses that result. Reusing the original incoming bool would be incorrect.
Source resolution and modifier context are then cached at the native points.

When requested, 501650 returns the first exact damage-type match in modifier
order, or original1.0. The type factor multiplies amount and stores binary32.
Hostile damage copies all seven adjusted words into Body.last_damage. Health
subtraction stores binary32 before invoking the actual source-aware setter.

Recovery reset, leader organization response, outer-actor response, attack
modifier and Mind response, presentation, event-kind3 source response,
organization notification and ancestor-or-own component88 response execute in
the original order. Mutable callback record/body/environment effects are retained.
Event kind is word5; word6 is not an interchangeable damage-event selector.

At nonpositive resulting health, capture eligibility and attempt are required
world operations. Successful hostile capture sets maximum*CaptureHealthFactor
through the setter and returns. Eligible uncaptured actors without property
bit30 are restored to1. Remaining zero-health destructible actors dispatch wall
member death or the world death queue. Dispatch alone does not perform full
death, kill credit, ownership transfer or reward processing.

Source-aware health 561E50
------------------------
The old plain set_health entry remains compatible with its original no-linked,
no-source domain. The new API requires linked setter dispatch before local clamp,
clearing source context in that recursive call exactly as native code does.
Actor source position is generation-resolved only after the first FX rule match.

Directional FX subtracts XYZ with three binary32 stores, normalizes only XY
through 409230, then scales XY by abs(new health ratio - rule threshold). Z
retains the unscaled difference. It is not a normalized/scaled3D direction.

Property bit29 selects the actual derived maximum getter. Its modifier path can
return an extended base*factor value, so the new Host getter returns Binary and
no binary32 rounding is inserted before normalization. The old plain setter's
null-context binary32 contract is preserved. Missing source/linked/derived-host
implementations return explicit callback errors instead of invented values.

StartDying 562380
----------------
Body virtual+38 chooses immediate world removal or corpse initialization. Its
actual meaning remains a required predicate, not a guessed destructible flag.
An optional death ability has explicit begin, allegiance query, optional invoke
on the real modifier context, and release boundaries. Action kind2 has native
actor/allegiance handles and XY with zero Z; other context fields are zero.
Ability internals and lifetime/resource effects are still world requirements.
After a successful begin, every Result exit attempts release exactly once,
including failed allegiance/invocation and invalid callback state. Primary and
cleanup failures are both retained. Failed release reports the handle and leaves
ownership with the world for explicit recovery. World callbacks must use Result
errors; panic/abort behavior is outside this callback contract. No retry occurs.

Creation time is read after ability release. Sink duration comes from the actual
definition. Owner+8 bit21 with positive sink duration causes two independent
4F0E50 queries, each stored binary32. If the first exceeds the second and actor
virtual+8 returns a positive value, two further virtual+8 calls compute the
adjustment. The middle dimension/sink quotient is stored binary32 before the
final division; the three dimension calls cannot be cached as one value.

Rot time plus adjusted sink duration is capped at10 before timer dispatch with
argument0. Negative delays are not silently clamped. Two separate allegiance
queries precede ownership removal when the first is present. A scheduled decay
does not imply removal or complete destruction.

Evidence and limits
-------------------
original_damage.py executes the whole original5625C0 and561E50 paths, actual
501650 factor scan, weapon predicates, clamps, sqrt and normalization. Linked
and neighbor bodies, actor virtual/world relationship queries, capture, actual
component effects, presentation and death queues are explicit fixture leaves.
219 executions cover73 scenarios in each ofPC24/53/64, including callback changes
to health, maximum, event kind, components, second-neighbor availability and
capture state. Final body/environment words and ordered events are compared.

original_dying.py executes full562380 and the real zero action-context constructor;
its ability, terrain, dimension, timer and ownership services are explicit leaves.
84 executions cover28 scenarios per precision. Float-return leaves use separate
fixed RET0 and RET8 code stubs to preserve the two native ABIs; no translated code
block is rewritten to a different return convention during execution.

Eight aa-body tests pass, including all270 historical body cases, the303 new
native executions and explicit partial-state/ability-cleanup failure checks.
Clippy is strict.
Peer reports and findings.json bind the final evidence separately.

No runtime FPU default is inferred. Supported arithmetic is finite nearest-even
with explicit precision and the numeric crate's stated exponent bounds. Errors
are nontransactional; automatic retries can duplicate graph damage or effects.
Original graph cycles, dangling allocations, invalid owner/segment pointers,
unresolved definitions and callback failures require explicit host handling.
There is no hidden success fallback for these paths.

Upstream armor/weapon hit generation, complete kill-credit/reward logic, full
actor ownership/death queues, abilities, world integration, AI and multiplayer
remain separate work. Diagnostic logging/CString representation is excluded.
No native process, UI, original asset, save, profile, security setting or old
release was changed. The pre-edit aa-body backup is preserved under
backups/phase5-aa-body-before-combat.zip.

Reproduce
---------
python research/phase6-combat/inspect_native.py
python research/phase6-combat/original_damage.py
python research/phase6-combat/original_dying.py
cargo test -p aa-body
cargo clippy -p aa-body --all-targets -- -D warnings
