# Stored Organization and Denizen value refresh

`stored_actor_value_refresh` binds native `5788B0` and `569E40` to the existing
ActorStore allocations and the source Bank which moved with them. Store entries
and paired ActorAccess entries use the same owner bindings. The live arithmetic
precision stays separate from the completed Bank's source precision.

The native Organization arithmetic remains
`aa_combat::organization_combat_value::recompute`. Denizen calls the unchanged
existing `load_denizen_value_refresh::recompute`; only its internal visibility
was widened. `algorithm-preservation.json` checks that change exactly.

The current component remains in its actual slot while `Components::split_slot`
lends the mutable component separately from other current slots. The existing
borrowed entity focus supplies current-member lookups. No component is removed,
no Entity/Body/Performer/instance state is copied, and the Mind Arena remains in
its owner. Completed target values come from the actual moved Bank, with token,
canonical name, source provenance and universe checks. Every reset and sum store
goes directly to the current component, including a prefix before later errors.

The bounded actual test uses diagnostic Tutorial02 allocations moved once. It
checks native-fixture input equality for `usa_airfield` and the actual HQ
Organization `regiment_usa_hq_infantry_unpacked`, three live arithmetic
precisions, Store and paired entry points, allocation identity, and source/layout
fault prefixes. Source evidence explicitly checks that `gbr_corps` lacks an
Organization slot. These are direct leaf tests, not proof of natural fresh Actor
initialization, property callback reentrancy, mission startup or playable battle.

Final source pins, execution outcomes and independent review are recorded in
`findings.json` after the focused current-source tests finish. Existing native
fixtures remain unchanged; owner bindings do not establish a new whole-function
native corpus on their own.
