# Native routing and ResultPlan, Phase 6

This phase computes actual routes on a supplied native-compatible tile grid.
It ports endpoint adjustment `53D340`, interior clamping `552640`, path
materialization `53D500/53CF40`, Solve orchestration `53E1F0`, and the ResultPlan
controller `5336B0`. The concrete search/heap/neighbor implementation is owned
by `research/phase6-pathsearch`; exact grid ray traversal is owned by
`research/phase6-spatial`. None of these modules replaces native decisions with
a generic A*, Bresenham line, or straight-line movement shortcut.

## Endpoint adjustment

`endpoint::adjust` follows each native attempt: clamp a temporary candidate to
the map interior, run the `53CE40` acceptance predicate, try `53CA30` object
displacement if rejected, then recheck only when displacement reports success.
When these reject, the persistent endpoint steps back toward the fallback using
the original integer-truncated cell step and normalization/store boundaries.
The final Manhattan distance must be at least binary32 `0x3951b717`; a false
return may still change the endpoint. X difference is stored before length
calculation; Y remains extended until its original store. This matters for
fractional coordinates. The map-interior upper bounds each have their original
binary32 store after subtracting cell size and binary32 `0x38d1b717`.

Zero/nonpositive integer steps and a float decrement that cannot make progress
return explicit errors rather than reproducing a native infinite loop. These
are unsupported native-data domains, not route-length limits.

## Predecessor materialization

`materialize::materialize` follows `parent = cell - delta[direction]`, retaining
the original nine-entry direction tables. It prepends node payloads
`[x, y, direction_x, direction_y, 1, 0, x, y]` and updates path counts/dirty state.
Existing region nodes and opaque metadata are retained. If the chosen endpoint
is the goal cell, it uses the requested world coordinates; otherwise it uses
the native cell center for the partial route.

Direction runs are reduced in original order. Across a turn, smoothing creates
a lazy tile trace and stops checking at the first blocked cell. Descriptor word
+0x28 is zero during that trace and restored afterward. A blocked shortcut emits
the previous cell center and retries the same predecessor. The emitted center's
half-cell term has a binary32 store that the initial endpoint/trace centers do
not. Existing head directions are recomputed and normalized at native stores;
the final first-node direction uses the actual starting world position.

The same-cell finalizer may still emit one node if start/goal world coordinates
differ sufficiently. Both finalizers unlink the original strategy path and
collision references on success. Invalid/cyclic predecessor chains are explicit
errors. World errors preserve earlier path changes and restore the temporary
collision-word suppression; they are not safe automatic retries.

## Solve and ResultPlan

`solver::solve` resets diagnostic path logging, adjusts/rejects the endpoint,
checks the native normalized length threshold, and performs the complete
`52CD90` path reset. With the region pather enabled and actor bit18 clear, it
constructs the region probe, tests direct region connectivity and invokes the
required region prepass. The prepass result is propagated unless exactly 1; a
returned first path node becomes the grid-search goal. Grid indices use signed
truncation followed by the masked arithmetic shift, without invented clamping.

The composed native tile search writes predecessors which are immediately
materialized. Failure uses `53C450` cleanup: clear nodes and counters, set dirty
and word18=-1, and unlink strategy references. Queue limits are explicit inputs:
the native bounded policy is available for parity, while choosing a different
policy can change partial-route results. No storage is preallocated to that cap.

`result_plan::result_plan` copies the previous destination even on exhausted
attempts, increments the signed retry count before work, preserves near-goal
and region/collision decisions, and reproduces the turning-circle test with its
asymmetric X/Y stores. It preserves the two property virtual calls, target
generation lookup, context flag/target writes, and Solve result semantics:
nonzero enters waypoint-offset update then ActionGo(0), zero invokes failure.
Continuation callbacks and the original second argument remain distinct.

All callbacks are required; ResultPlan does not feed a generic move command
directly into Mover. Actor activation, region/object geometry, the offset tangent
operation, outer scheduling and world ownership remain explicit dependencies.
Native diagnostic string allocation/logging is excluded from the controller
fixtures. Object keys in collision descriptors are host identities, never
dereferenceable original process addresses.

## Validation and limits

- `endpoint_oracle.py`: 258 original executions, including retries, boundary
  clamping, successful/failed displacement, thresholds and fractional positions.
- `materialize_oracle.py`: 159 executions covering every direction, clear and
  blocked smoothing, repeated turns, partial endpoints, same-cell targets and
  preexisting region tails. Only allocation and LOS/collision leaves are seams.
- `result_plan_oracle.py`: 249 executions covering controller branches, turning
  geometry, target replacement, property calls and synchronous callback mutation.
- `solve_oracle.py`: 72 **whole native Solve** executions. Only node allocation
  and free are replaced. Endpoint adjustment, static collision masks, concrete
  heap/cost/search, original tile ray iteration and smoothing all execute from
  the original image. Rust matches status, every path word, metadata, strategy
  fields, search epoch and the complete collision predicate call order.

Each corpus tests PC24, PC53 and PC64, with explicit round-to-nearest arithmetic.
Observed original instruction bytes are checked against the accepted v14 image.
The whole-Solve corpus uses static collision masks with no actor objects and
has the region pather disabled. It does not prove dynamic actor collisions,
region topology, path-cost modifiers, authoritatively loaded map construction,
or end-to-end gameplay. Those boundaries are not silently accepted or defaulted
by the production traits.
