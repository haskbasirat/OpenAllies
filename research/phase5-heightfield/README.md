# Original terrain heightfield sampling

This crate ports the original complete nearest (`0x550190..0x550205`) and
interpolated (`0x550210..0x55042f`) routines. It supplies two previously required
`aa-spatial::World::sample_height` operations, but does not wire them into a
world or infer how authored map height data creates the native grid.

The original heightfield object stores width/height at +0x0c/+0x10, arithmetic
coordinate shift at +0x14, binary32 cell size at +0x18, signed index bias at
+0x1c, and a row-major binary32 sample pointer at +0x40. These fields remain
explicit inputs. A checked positive rectangular buffer replaces native pointer
arithmetic; invalid grids and undefined conversions produce errors.

## Nearest sampler

Each coordinate is converted by original `0x96cc4c` to a signed integer toward
zero. The signed index bias is added with 32-bit wrap, then the value is shifted
arithmetically by `coordinate_shift & 31`. Each result is clamped to the grid.
The selected binary32 sample is returned as an extended value without arithmetic.
The cell-size field is unused in this operation.

## Interpolation branches and arithmetic

The original CRT `0x96db7a` consumes each promoted binary32 coordinate and cell
size. Its normal finite path performs `FPREM` repeatedly until C2 clears, using
a quotient truncated toward zero. This is implemented by the exact dyadic
`aa_numeric::Binary::remainder_trunc` operation, preserving dividend sign and
signed zero. It is not division rounded to a selected precision followed by
subtraction. The oracle executes the CRT classifier and actual FPREM loop.

X remainder is stored to binary32, while Y stays extended. They are compared
against binary32 `0x38d1b717` (approximately 0.0001), using signed less-than,
not an absolute-value test:

- Both below the threshold: use the original nearest sampler directly.
- Only X below: average nearest heights at Y minus/plus half a cell.
- Only Y below: average nearest heights at X plus/minus half a cell.
- Neither below: use four weighted samples.

The half-cell product and offset coordinates have their original binary32
stores. The first height in each average is stored to binary32; the sum and
final half multiply stay extended. Negative remainders enter these same signed
branches. A finite nonzero negative cell size is supported as recorded native
input behavior, without implying that native map constructors create one.

The four-sample branch retains an extended reciprocal of cell size and stores
both normalized remainders to binary32. Integer grid indices here use truncated
coordinates shifted directly: **the nearest sampler's index bias is not added**.
Indices are clamped before neighbor access. The missing X or Y neighbor repeats
the base sample; when either diagonal coordinate is missing, the diagonal also
repeats the base, even if the other axis neighbor exists.

The two cross weights `tx*(1-ty)` and `ty*(1-tx)` are stored to binary32 before
sample multiplication. The diagonal weight `ty*tx`, base weight, weighted terms,
and ordered additions remain extended. The returned `Binary` therefore must
not be rounded before the caller's original store boundary. All operations have
explicit PC24/53/64 and round-to-nearest/ties-to-even arithmetic; native thread
precision and exception status lifecycle are not inferred.

## Native validation

`original_heightfield.py` runs the complete original routines without arithmetic
or callback hooks. It supplies only synthetic object/grid storage and a return
observer. The observer captures the exact 80-bit return before an additional
binary32 store. It checks return stack, all x86 nonvolatile registers, and
unchanged grid storage. Executed instruction ranges are checked byte-identical
against the accepted v14 executable and individually hashed.

The corpus contains 735 executions: 245 cases under each precision. Coverage
includes every high-level interpolation branch, threshold neighbors, negative
coordinates/remainders, truncation and signed-wrap/shift behavior, grid edges,
one-row/one-column/single-cell grids, non-power-of-two and negative cell sizes,
signed zero, denormal divisors with large FPREM quotients, and 100 deterministic
fractional grids. Test decoding reconstructs the full native 64-bit significand
from integer limbs and an exact power of two, avoiding host floating conversion.

Two Rust tests compare every native extended result and binary32 store, then
verify explicit invalid-storage/arithmetic errors. Tests establish the sampler
operations on supplied grids, not native map loading, pathfinding, collision,
terrain compositing, or a complete playable world.
