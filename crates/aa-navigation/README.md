# aa-navigation

Original owned-path, prepared UAI movement-state, and concrete tile-routing
operations. These preserve native queue and geometry arithmetic; authoritative
world data and object collision dependencies remain caller responsibilities.

- `OwnedPath::assign_from`, `clear_nodes`, and `prune_kind2_tail` recover the
  original node ownership and metadata operations. Every node retains its exact
  eight-u32 payload. Tail pruning is driven by the signed type-2 count.
- `activate_move` recovers `0x52df60` field writes and activation order.
- `move_path_event` recovers ordinary/formation path events `0x532170/0x532310`.
- `action_go` recovers `0x5300a0`: prepared-node arrival checks, mode/speed choices,
  waypoint adjustment, Mover request, one native RNG draw and timer scheduling.
- `endpoint` recovers destination adjustment and map-interior clamping.
- `search` implements the native heap, first-discovery tile search, neighbor
  ordering, heuristic, and predecessor writes. It is not a substitute A* policy.
- `materialize` turns native predecessors into owned path nodes with ordered
  line-of-sight smoothing; `aa-spatial::tile_walk` supplies the recovered iterator.
- `solver` composes destination adjustment, search and materialization, retaining
  the region-prepass interface and native failure cleanup.
- `result_plan` recovers retry accounting, turning geometry, collision-directed
  continuation, and Solve/offset/ActionGo dispatch.
- `region::prepass` turns an ordered native region-graph result into the original
  kind-2 waypoint chain.
- `region_search` implements the native region graph heap, ordered discovery,
  cached/geometric distances, traversal epochs and region-chain construction.
  Native topology construction and world-to-region lookup remain required.

`MoveState` and `ActorState` expose only fields used by these operations, in
native XY coordinates and raw binary32 bits. `ActionContext` requires explicit
x87 precision and the actual `GroupSpeedMinimum` / `MoverCatchupSpeed` settings.
There is no inferred constructor or global precision default.

The required `NavigationWorld` methods preserve unresolved native activation,
organization, formation, geometry, actor eligibility and scheduler boundaries.
`begin_mover` can call the independently recovered `aa-movement` operation after
converting `MoverRequest` bits; its bool result is intentionally ignored by the
original ActionGo caller. `next_world_random_u32` must draw from the actual shared
world stream. Do not use a rounded float draw or create an independent seed.

Errors are explicit and nontransactional: earlier state changes and callbacks
are not rolled back, and automatic retry can duplicate effects or random draws.
Allocation failure and invalid native-pointer domains return errors instead of
imitating undefined memory behavior. Source paths cannot alias the destination
under safe Rust borrowing; original self-copy is a recorded native no-op.

Evidence, exact offsets, native-fixture boundaries and remaining dependencies:
`research/phase5-navigation/README.md` and
`research/phase6-navigation/README.md`.

Phase 8: `formation_follow` ports the UAIStateMove formation speed controller
`0x52ff00` (ActionGo's mode-3 `formation_ready` gate) with slot lag `0x52e400`,
path-length difference `0x52e300`, path length `0x52ce70`, `0x46b000`,
`0x45d880`, catch-up `0x577090` and Mover clamp `0x576d20`. Evidence:
`research/phase8-movement/README.txt` (498 original executions).
