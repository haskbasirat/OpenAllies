# Fresh Actor initialization checkpoint (in progress)

This work connects actual fresh `4D7230` / non-null Object `4EBE50` to the same
`Staged.actors`, moved `Staged.definitions`, `World` and adopted `SpatialWorld`.
It does not create a saved `LoadShell`, call saved `Init(NULL)`, clone a mutable
world or substitute a successful callback for an unported native service.

## Source changes

- `aa-actors::actor_init_existing`: storage-independent common algorithm. The
  older public `actor_creation` wrappers forward through a borrowed field adapter.
  The historical `native_actor_creation` regression passed after this extraction
  (`legacy-common-regression-01.log`). Its fixture is
  `research/phase8-actor-creation/native-actor-creation.json`; named seam callbacks
  in that fixture do not certify concrete game-side child implementations.
- `map_actor_init_existing`: direct runtime fields and original mutable Params;
  source precision comes from the actual moved Bank, while arithmetic/index
  precision remains the explicit live input. World terrain, source-field receipt
  and index precision are checked before native initialization stores.
- `live_placement`: crate-private source/field/descriptor view extraction preserves
  the public saved-record adapter. `check_created` uses current/current and actual
  zero/null call arguments; it reads the caller's original Params directly.
- `map_actor_placement`: the same adopted index and actual source shapes, zone
  allocations, typed overlap lists and first-hit kernel. Preparing a logical
  current identity transfers its sole placement owner without inserting native
  tile membership. Actual `4EE4C0` remains the later membership operation.
- `map_actor_footprint`: real mode-one occupancy traversal and native-order zone
  reads. It stops immediately before an unbound positive Actor `+64` reveal call.
- `SpatialWorld::prepare_actor_bound`: same Bank/source-PC shape preparation.
  `set_initialization_scalar` permits only native Z/cache words `28/44/48` in the
  same published member; it cannot move the actor through an arbitrary raw write.
- `map_replacement_effects`: direct mutable queued Params loan into the actual
  fresh creation adapter. Startup precision comes from the borrowed sealed
  application `StartupInputs`, threaded through the existing mission path.
  Required destruction, non-Actor creation, property and diagnostic services keep
  their explicit boundaries. No alternative successful mission path is added.

## Limits and verification

Production checks have passed for the placement/footprint extraction. A current
actual-data test and the previous saved-placement regression are pending final
execution. Early test attempts stopped during compilation on shared, concurrently
edited source; the logs are retained and are not passing test evidence.

The actual-data test uses an explicitly constructed fresh actor inside a genuinely
stopped Tutorial load. It checks ownership, source/live precision separation,
original descriptor reads and retained state. It does **not** certify completed
`4D6F10`, natural Nation selection, mission readiness, full fresh initialization,
combat or a playable battle. Positive required-zone, reveal, relation, component,
property and presentation services remain exact reached requirements until their
real owners are connected. Consult the final findings/pins when frozen; this file
is an in-progress scope record.
