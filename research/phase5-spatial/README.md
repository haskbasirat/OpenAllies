# Ground position and terrain-query recovery

This component ports `4EEB40 -> 4F0E50 -> 4EE5C0`, plus the `46EE00`
half-cell snap and the `4CF210`/`4EA0F0` shape-placement construction sequence.
It leaves terrain heightfield sampling, tile ownership, collision and rendering
as required boundaries. It does not connect command5 straight to a Mover or
replace the unrecovered UAI/path-production chain.

Original input image SHA256:
`5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26`.
Accepted v14 image SHA256:
`3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1`.
The native ranges and every actually executed instruction range in the fixture
are hash-checked against both images. Original assets/builds/profiles are never
written. No GUI, game process, injected code or operating-system changes are used.

## Position sequence

`4EEB40` captures actor+8 flags. Bit21 chooses query flags `0x123`; otherwise,
ordinary movement uses `0x122`. Bit12 without bit21 bypasses height initialization
and later copies an uninitialized stack word to Z. That unsupported native domain
is rejected before callbacks, rather than replaced with an invented height.

The query pointer is **actor+20**, not the proposed XY. A base-height callback
may change actor position; grid lookup and an overlay callback re-read it. The
height return is stored as float32. The routine then re-reads actor bit16. If set,
it captures the map cell size, calls actor virtual+0xC and invokes `46EE00` on
candidate XY. The shape getter's opaque return is not treated as a radius.

`4EE5C0` checks actor bit26 once. If set, it builds these descriptors in order:

1. New A: candidate XY and current facing, then virtual+0xC.
2. Old A: current actor XY/facing captured before the next virtual+0xC.
3. New B: candidate XY and re-read current facing, then virtual+0x10.
4. Old B: re-read actor XY/facing captured before the next virtual+0x10.

Each descriptor is six words: opaque shape identity, facing XY, position XY and
zero. `5539B0` receives actor, oldB, newB, oldA, newA in that order. It is a required
tile-owner boundary; its own bit26 recheck, transformed extent calculation,
overlap update and spatial lists are not silently implemented by this crate.
The candidate is a distinct immutable buffer, as supplied by `4EEB40`; arbitrary
aliasing between an external candidate pointer and actor fields is outside scope.

All three position words are committed before actor virtual+0xC8. Then `50C250`
checks current GWorld+0xD8 against6 and manager+4 against null before calling
`43B760`. **The manager is `[GWorld+0x130]`; this is not actor+4.** The decompiler's
fastcall inference omits this distinction; the actual caller passes actor on the
stack and the manager in ECX. Mutation fixtures cover phase/target changes during
the position callback as well as changes to actor position/facing/flags.

## Terrain-query arithmetic

Flag0x10 selects `550190` (nearest) instead of `550210` (interpolated) on the base
heightfield at GWorld+0x34. The returned value is always stored to float32 first.
Flags1 and4 together return this stored base minus0.5 immediately.

Flag2 enables map-cell inspection. XY is converted toward zero to signed32,
arithmetically shifted by `map.coordinate_shift & 31`, then clamped to the grid.
Grid dimensions/count must form a positive checked i32 domain. Cell kind is
`cell.definition+0x1C`. Kind1 and kind2 select the overlay heightfield at
GWorld+0x38. Otherwise eight neighbors are examined in the original static order:
`(-1,-1),(-1,0),(-1,1),(0,1),(1,1),(1,0),(1,-1),(0,-1)`, skipping out-of-grid cells.
Finding kind1/2 in a neighbor selects overlay but does **not** give the center
cell the kind1 offset.

For a center kind1 cell only, native signed32 wrapping arithmetic computes
`(((y*width+x)*0x1273+0x18B3)%0x1B55)%1000`. Its integer value multiplies
`(offset_max-offset_min)`, then exact f32 `0x3A83126F`, adds offset_min, and adds
the overlay height. All arithmetic follows the explicit supplied x87 precision;
there is no RNG state transition and no guessed normal distribution.

Overlay plus stored base is stored to float32. Flag1 returns the larger of that
stored sum and the un-stored base-minus0.5 value. Remaining query bits do not
change this routine. The map/grid/definition/settings objects are a stable
snapshot; mutation of those global objects during height callbacks is outside
the API contract. Actor fields remain callback-mutable as described above.

The actual `550190/550210` heightfield interpolation is **not implemented here**.
The oracle supplies these leaves using native x87 addition of two authored
float32 fixture values, so extended return/store distinctions are exercised
without pretending to recover a map-height sampling formula.

## Half-cell geometry

`46EE00` stores half the cell size to float32. It computes the reciprocal in x87,
stores a float32 copy, but retains the extended reciprocal for X. Y uses the
stored copy. Each scaled coordinate adds0.5, is stored to binary64, passed to
original CRT floor and multiplied by the stored half-cell size before a float32
coordinate store. The port uses exact dyadic arithmetic and explicit PC53
rounding for the binary64 store, then a checked floor-to-i32 result.

For finite float32 inputs/cell sizes, intermediate exponents in the accepted
floor-i32 domain are normal binary64 or zero; rounding to53 significand bits
therefore has the same value as that store. Cell sizes must be positive, their
stored half must be nonzero, and the reciprocal must fit float32. Larger floor
results reject: this checked-i32 floor domain is narrower than the native CRT's
general binary64 floor domain. No host floor,
sin/cos or implicit FPU mode is used.

This preserves a counterintuitive native boundary: under PC53, cell size7.25
and equal coordinates1.8125 produce X3.625 and Y0.0. A symmetric reciprocal or
ordinary nearest-grid formula would change the game result. Dedicated fixtures
cover this and two other X/Y precision asymmetries.

## Validation and limits

`original_spatial.py` executes full original query, move and commit routines,
the two original descriptor constructors, the real `46EE00` and CRT floor/
conversion instructions. World leaves are hook seams and explicitly excluded
from executed-instruction coverage. Each case checks callee-saved registers and
stack cleanup. All 504 cases compare exact actor/candidate bits, height outputs,
descriptors and callback events against Rust across PC24, PC53 and PC64.

Four Rust tests cover native parity, safe invalid-domain rejection and callback
failure propagation before/after the position store. Failure tests are Rust
integration-contract checks, not additional original-x86 cases.
`cargo clippy -p aa-spatial --all-targets -- -D warnings` passes. These are
component results, not a complete live movement/rendering acceptance test.
No default actor height, facing, flags, map definition or game-wide FPU mode is
invented. There is no CPU/GPU performance claim.

Errors can follow earlier mutations/callbacks. Operations are nontransactional;
they do not roll back and must not be automatically retried. A required world
implementation must preserve its own ownership/invariants and propagate failures.

`aa_movement::GroundWorld::move_on_ground` currently returns `()`, while this
crate returns `Result`. There is no adapter between them. A future integration
must propagate failure through the movement step, stop later scheduling and
retain accurate partial-state diagnostics. Ignoring the error, unwrapping it,
or claiming rollback would violate this boundary.

`client_samples.rs` and `client-samples.json` provide six selected native cases
for the separately validated client diagnostic. They use explicit fixture
callbacks and never supply a default live heightfield, shape or tile owner.
