Native actor value and experience distribution
=============================================

crates/aa-body/src/experience.rs implements the argument=false path of
4D01E0/578C00 and full outer4CE770/578710 routing.

The actor value begins at exact zero unless component7C exists, in which case
its+6C value replaces zero. Optional wall+98/+34, combat+74/+60 and component88/
+14 values are then added in that order, retaining extended intermediates.
The result remains Binary; the caller chooses its native store boundary. The
alternate argument=true resupply calculation is not part of this interface.

Award routing preserves the0.0001 threshold, componentA4 priority over7C,
regiment member order and null members, one-step organization leader fallback,
and separate two boolean arguments. A recipient with an experience component
may first forward to its leader with both flagsfalse; its own component pointer
is then reread before applying. A regiment uses the original member component
identity while rereading its count and member array after each callback. No
velocity, generic unit class or inferred experience distribution rule is used.

The World::apply callback is precisely571A60(component,amount,1). Actual rank
thresholds, promotion abilities, notifications and stat effects inside571A60
are still required world services. Routing does not pretend that recording an
event applies those effects. Callback errors propagate after prior effects;
they are not retry-safe. Missing components after callbacks produce errors.
The cycle guard rejects a repeated route only between promotion callbacks,
while the pure graph snapshots are unchanged. A successful callback clears all
active route keys: it may change any suspended ancestor's component, members or
count. Repeated actors after callbacks therefore reevaluate their live routes.
Callback-producing loops cannot safely be classified as infinite because a
later callback may terminate them; no arbitrary callback count cap is imposed.
Pure field views must be coherent with actual actor/component allocations.

Validation
----------
native-experience-fixtures.json contains249 original executions at PC24/53/64:
165 component value cases with no behavioral hooks, including cancellation,
presence masks, signed zero and exact80-bit returns;84 routing cases with only
571A60 intercepted. Cases include nested regiments, duplicate/null members,
leader flags, member-array shrink/replacement, changed experience component,
and a self-leader case which terminates under the original boolean arguments.
Two regressions revisit an ancestor after a callback: one grants it an
experience component, and another clears its regiment without changing its
actor fields. Both execute unchanged native routing and must succeed in Rust.
Every case verifies stack and nonvolatile-register preservation. All visited
original/v14 bytes agree; hook entry bytes are not proof of the intercepted leaf.

Rust compares exact extended values and caller binary32 stores, or every
promotion callback/final actor/member state. A separate test checks invalid
recursive graphs, nonfinite numeric inputs and callback failure propagation.
This removes arithmetic/routing dependencies of credit without claiming a full
combat/ownership/promotion runtime or any playable-game parity.

Reproduction:
  python research/phase6-experience/inspect_native.py
  python research/phase6-experience/original_experience.py
  python research/phase6-experience/record_findings.py

The complete native game, release, saves and assets remain unchanged.
Independent review, when available, is separate from the producer ledger.
