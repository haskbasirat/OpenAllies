# Original Mover operation recovery

Original AA.exe SHA256:
`5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26`.
Accepted v14 native SHA256:
`3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1`.
All inspected movement ranges are byte-identical between these two images.
Development03, original assets, native runtimes and profiles remain unchanged.

## Scope and entry points

The recovered complete operations are begin-ground-motion `0x5773c0`, stop
`0x5767c0`, and ground-step `0x5774a0`. The Rust implementation preserves the
native branch/callback order and explicit numeric store points. It requires
world/geometry and native-turn providers rather than guessing those systems.

Native UAI::ActionGo `0x5300a0` obtains a path node, applies its direction and
waypoint adjustment, chooses requested speed/factor and calls `0x5773c0`.
TellActorMove's actor command type 5 follows an earlier command/Mind route;
the recovered order decoder is not a proof that command 5 can directly invoke
begin-ground-motion. No path search, route tie-break, formation destination or
collision solver is invented in this crate.

The outer Tick entry `0x577e50` receives the tick subobject (`Mover+8`). It routes
falling to `0x577a70`; otherwise it checks active component and actor/property
state, handles both speed fields zero via arrival/failure callbacks, then chooses
aircraft or ground-step. Those scheduling, falling and aircraft operations are
not silently folded into the ground-step API.

## Native field contract

`0x497fa0` registers MOVER as component slot 3, MIND as slot 4. The actor
component array starts at `+0x60`: Mover is `+0x6c`, Mind is `+0x70`.
Mover constructor `0x576920` sets vtable `0xa3ce88`. Its owner pointer is `+4`.
Actor identity uses the original generation-bearing 32-bit handle; `aa-actors`
owns that registry and can bridge through `ActorId::raw/from_raw`.

| Mover offset | Meaning used by recovered operations |
|---|---|
| +10..18 | Destination XYZ; begin-ground-motion sets Z to zero |
| +1c | Last/reported step speed |
| +20 | Current stored speed |
| +24 | Requested speed |
| +28 | Effective maximum speed; constructor/property refresh, outside step |
| +2c | Acceleration |
| +30 / +34 | Turn radius / derived turn rate |
| +38..40 | Desired direction XYZ; begin sets Z to zero |
| +44 / +48 | Target handle / range |
| +4c | Ignore terrain speed limit |
| +50 | Speed multiplier |
| +54 / +55 / +56 | Instant turn / currently turning / falling |

Actor position is `+20..28`, facing `+2c..34`; Character is slot 2 (`+68`).
ActorMotion.flags is the word at actor `+8`, not the separate property flag word
at `+100` used by the body component.
Character `+24` is its paused/stopped timer flag. The terrain speed input is
read through actor `+f8`, pointee `+38`; the upstream producer is not replaced
with an invented calculation.

Data loader `0x46593e` reads MoverComponent: maximum speed `+228`, movement
field `+22c`, acceleration `+230`; constructor also reads turn radius `+234`
and turn type `+238`. No constructor parity is claimed by a guessed Rust Default.

## Ground-step details that matter

- Acceleration uses the function argument. Turning/displacement use
  `GWorld+0xe8`. They are distinct inputs even when an ordinary tick uses equal
  values.
- The fixed near-destination test compares squared XY distance against original
  f32 `0x38d1b717` (approximately 0.0001). This is separate from the begin-order
  radius `TAIReachedDestinationThreshold` at SVar cache `+54`, native default
  f32 `0x3dcccccd` (0.1). Its registration is `0x50701d..50704e`, ID 21; original
  `Game/SVars.tgi:86` contains the same value. It remains a supplied setting.
- Acceleration toward a higher requested speed is additive and clamped. A lower
  requested speed takes effect immediately. `TerrainMinimumMoveSpeed` has native
  default 0.5 (`0x508f29..508f55`) and floors a positive terrain cap; it is not an
  unconditional minimum actor speed.
- The multiplier product is stored as f32 current speed while remaining live
  in x87 for displacement and the speed-change comparison. Reloading that f32
  prematurely can change physics. `aa-numeric` retains this exact boundary.
- Destination projection uses desired direction; actual displacement uses the
  current/turned facing. Overshoot is clamped only when both XY directions are
  exactly equal and projection is positive. Crossing also controls arrival even
  in other directional cases, matching original branch behavior.
- Actor flag bit 18 bypasses the world collision query. Otherwise a hit pauses
  Character, notifies Mind `+154` with collision/facing, sets reported speed zero
  and returns without committing position. Current stored speed is retained.
- The successful position helper `0x4eeb40` queries terrain height at the actor's
  current XY before committing candidate XY. It normally uses height flags
  `0x122`, or `0x123` for actor flag bit 21. Tile locking/spatial notifications
  occur in that boundary. The crate does not guess a different height query.
- Near destination, current and reported speed become zero. After a crossing,
  only reported speed is cleared, followed by scheduler cancellation and Mind
  `+12c` (or deactivation when no Mind exists).
- A valid target may complete by shape-range before moving (returning that Mind
  callback's AL), or center-distance range after the position commit.

## Turning dependency

`0x445400` computes a float32 cross product, classifies dot sign, clamps the cross
to [-1,1], calls original CRT asin `0x96d780`, then performs +/-PI correction.
Ground-step explicitly stores the returned angle to float32 at `0x5775da`.
The turn increment is also float32 before comparison and rotation.

`0x4454a0/0x4454e0` rotate counterclockwise/clockwise using the game's table
wrappers `0x85da80` (cos) and `0x85daa0` (sin), common lookup `0x85da10`.
They do not call host trigonometry. The root owns independent native table and
angle recovery. `OriginalTurning53` integrates the independently reviewed
`aa-trig` API; `ground_step_original_pc53` rejects other modes before mutation.
This does not establish a native game/thread default precision.

The native CRT asin also inspects the x87 sticky precision-exception flag.
Unicorn does not reliably set that flag on earlier arithmetic, so its clean
status is not proof of a real CPU/game status. The concrete `OriginalTurning53`
provider uses the independently verified PC53 domain. Other precisions still
require a caller-supplied provider with a proven explicit FP environment or an
independently proven output-invariant domain.
Operation-entry precision status alone would not account for inexact arithmetic
or floating-point world callbacks executed before the angle calculation. The
concrete PC53 provider is sound within that explicit domain because the original
CRT skips the sticky-PE-controlled qword roundtrip when PC is 53. The compatibility
angle block saves/restores the host FP environment; no host asin/sin/cos substitute
is used. Quantized rotation is exact software arithmetic over the recovered tables.

## Differential validation

`native_oracle.py` executes original full entry routines in Unicorn, with
synthetic activation, target-shape, collision, terrain and world-callback seams.
It does not launch a game or call into a live process. Supporting vector equality,
facing setter, clamp and target registry instructions execute normally. Return
stack adjustment and EBX/ESI/EDI/EBP preservation are checked for every case.

The base 186 cases span x87 PC24, PC53 and PC64. Rust compares exact float bits,
flags, return bool and callback order for begin/stop, acceleration/clamp, separate
delta values, terrain caps/floor, multiplier, crossing/behind-direction, collision
and bypass, Character pause/resume, early/late target range, old-XY height queries,
signed zero and raw negative scalar inputs. A raw negative case exercises the
routine's finite domain; it does not authorize such a game configuration.

The extended native fixture file has 297 executions, including 37 additional
turning scenarios per precision mode. The concrete Rust PC53 test compares 90
complete ground steps: 53 existing straight-step scenarios plus those 37 turn
scenarios. This covers gradual left/right, snap, instant/zero-rate, facing lock,
collision while turning, crossing while unaligned, and inexact prior coordinates.
Thus Rust performs 276 comparisons representing 223 unique native cases, not
297 supported concrete turning cases. All four movement tests pass, including
nonfinite/falling rejection and rejection of concrete PC24/64 before side effects.
Other-PC turning fixture rows are retained research evidence, not an accepted
concrete-provider domain.

The API rejects nonfinite inputs, unsupported falling dispatch and missing Mind
on target callback paths rather than silently running undefined/native-invalid
pointer cases. This deliberate safe domain is narrower than arbitrary memory.
Tests do not prove the world seams themselves, full movement scheduling or
complete gameplay. `movement-evidence.json` binds the exact source, fixtures,
commands and dependency reviews. `actiongo-chain.json` records the separate
read-only path ownership/dependency roadmap; no path solver is implemented here.

Callback-mutation parity concerns the supplied mover/actor fields. StepContext
and the begin-order radius are immutable operation inputs; mutation of global
SVars or world delta during a callback is outside this contract. The native
world integration must supply a stable settings/tick snapshot for the operation.

An error can occur after earlier state changes or world callbacks. The operations
are nontransactional: they do not roll back those effects, and callers must not
automatically retry a failed step as though nothing had happened. The concrete
provider's unsupported-precision rejection is checked before entering the step.
