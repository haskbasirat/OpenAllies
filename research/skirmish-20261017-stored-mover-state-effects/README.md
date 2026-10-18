# Stored Mover state-effects successor

This successor replaces the stored Mover adapter's required `4D4A90` and
`4D16B0` callbacks with the actual shared stored Actor state-effects adapter.
The earlier Mover/Element kernel, native fixtures, frozen executable and
constructor-lab output remain unchanged in `../skirmish-20261017-stored-mover-element`.

The callback receives the same `Staged`, Catalog, object key, live precision,
and current state descriptor. The adapter commits the native state-bit prefix
before calling it, then rereads the actual component state and placement flags
on both success and error. The successful falling suffix ORs bit 12 into those
reread flags through the existing narrow placement operation. Both identity
directions must still identify the same generation before stores. The shared
Actions callback is also bound to the actual Bank, Arena, published index and
current variables; falling state 24 is nonexclusive and does not invoke that
callback in the native state kernel.

The existing diagnostic constructor laboratory deliberately has no initialized
World. Its live-falling probe now reaches the real callback's missing-World
refusal, preserving the earlier state-24 bit and omitting the later bit-12 store.
It cannot establish successful live falling, natural fresh Actor initialization,
mission startup or playable battle. `actual-lab-v2.json` is a new output; it does
not replace the old explicit unported-callback fixture.

`shared-kernel-preservation.json` records six unchanged shared/saved source files.
No new native execution is claimed for this ownership wiring. Final author
execution and source pins are recorded separately in `findings.json` when the
current-source focused test and immutable replay have completed.
