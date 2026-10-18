# aa-movement

Phase6 makes every `GroundWorld` boundary fallible, including activation,
lookups, collision, facing/position changes and notifications. Failure stops
the operation at that callback; earlier native writes and external effects are
retained. `stop_ground_motion` also returns `Result`. Successful native traces
are unchanged. Hosts must propagate spatial/query failures and must not retry
a partially completed movement operation automatically.

Original Mover ground-order and ground-step recovery. This is a simulation
component with required native-world boundaries, not a direct-walk replacement
for the game's route solver.

Public operations:

- `begin_ground_motion`: complete original `0x5773c0` state transition, including
  falling/activation rejection, destination/direction fields, strict arrival
  radius and scheduling. Near destinations zero current speed but retain the
  prior requested speed and multiplier.
- `stop_ground_motion`: original `0x5767c0` flag-controlled speed/scheduling stop.
- `ground_step`: the `0x5774a0` ground-step control flow: gradual/instant turn,
  acceleration, terrain speed cap, multiplier, target crossing, collision,
  position commit and ordered Mind/Character notifications.
- `ground_step_original_pc53`: the concrete original-trig integration, requiring
  explicit PC53 and rejecting other precision modes before mutation/callbacks.

`MoverState` and `ActorMotion` use native X/Y ground and Z-up coordinates.
`StepContext` keeps the function's acceleration delta separate from GWorld's
turn/displacement delta and requires explicit x87 precision. `aa-numeric` performs
dyadic arithmetic with native store boundaries. No host `sin`/`cos` is used.

`GroundWorld` is required for native activation, target generation/shape checks,
collision queries, scheduler/Mind callbacks and terrain/spatial position commits.
`OriginalTurning53` uses `aa-trig`'s recovered angle and quantized rotation; its
PC53 domain avoids the native CRT branch controlled by sticky precision status.
Other precision modes still require a separately supplied `GroundTurning`.
There is no default that silently clears collision or invents a straight route.
Callbacks can mutate state; later reads follow native order. Arithmetic/provider
errors can follow prior state changes or world callbacks. Operations do not roll
back those effects; callers must not automatically retry a failed step as though
nothing had happened.

Important integration boundary: native UAI `ActionGo` (`0x5300a0`) calls
`begin_ground_motion` only after it has a path node and adjusted waypoint.
Actor command type 5 must not be wired directly to this operation as if native
path production had been recovered. Ground step is not the outer scheduler or
the separate falling/aircraft movement paths.

Details and native evidence: `research/phase4-movement/README.md`.

Phase 8 additions (evidence: `research/phase8-movement/README.txt`):

- `mover`: Tick dispatch `0x577e50` (falling, active component, Properties
  suspension `0x4d03d0`, zero-speed arrival/failure stops `0x5768a0`/`0x5768f0`,
  overlap refresh, aircraft or ground step), stop-turning `0x5767f0`, speed setter
  `0x576b20`, maximum-speed/turn-rate refresh `0x576c70`/`0x576850`, post-load
  falling fixup `0x576700`, launch speed `0x577380`.
- `falling`: fall starts `0x576740`/`0x5770a0`, falling step `0x577a70` with pose
  commit `0x576bb0`, map interior test/clamp `0x5766a0`/`0x552640`, normalize
  `0x4453a0`.
- `formation`: organization slot positions `0x5797e0`/`0x578250`/`0x57bef0`, slot
  reassignment `0x57ba80`, leader change `0x579a00`, embedded native PC53
  2048-entry direction table and its draws `0x444a90`/`0x45d400`.
All are replayed against original-x86 fixtures under PC24/53/64.
