# aa-heightfield

Original terrain heightfield sampling, with exact finite dyadic arithmetic:

- `Heightfield::sample_nearest`: original `0x550190`.
- `Heightfield::sample_interpolated`: original `0x550210`, including its nearest
  and two axis-average branches, CRT remainder, and extended return value.

The caller supplies an explicit precision for interpolation. The `binding`
module now constructs the native grid from actual world extent bits and the
authored resolution SVar, and loads lossless version1 height chunks. It retains
limits, defaults, per-sample byte flags, source SHA256 and unread chunk tails.
The native reader is `0x551E60`; `0x5500F0` is its writer. Version0 requires a
world-dependent flag rebuild and is explicitly unsupported. Dimensions/counts
are checked before allocation; rounded preview JSON is not an input.

Choosing base/overlay sources and the native thread precision remain explicit.
This crate does not substitute the preview renderer's terrain geometry for the
game's authoritative heightfield.

Both functions return `aa_numeric::Binary`. The caller must preserve that value
until its native store boundary; preemptively converting it to `f32` loses bits.
Finite samples and defined coordinate conversions are required. Invalid storage,
nonfinite arithmetic, zero interpolation cell size, and out-of-range integer
conversions return explicit errors. Nearest sampling does not use cell size.

Evidence: `research/phase5-heightfield/README.md`.
Native construction, actual-map loading and sampling evidence:
`research/phase6-spatial/README.txt`.
