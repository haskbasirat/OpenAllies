# aa-numeric

Finite arithmetic for native-instruction ports. The caller must select
`Precision::Single24`, `Double53`, or `Extended64` for each operation. This does
not claim the original process always runs with the default x87 control word.

`Binary` accepts exact binary32/binary64 bit patterns and signed/unsigned32-bit
integers. It implements add, subtract, multiply, divide, square root, exact
truncated-quotient remainder, comparison, a binary32
store, and checked truncation toward signed32-bit integer. Arithmetic uses
`BigUint`, with round-to-nearest/ties-to-even at each requested precision;
neither host floats nor the host FPU mode determine results.

The finite dyadic implementation was extracted from the independently reviewed
`aa-economy/src/binary.rs`. That crate remains unchanged. Division, binary64
input, checked exponent bounds and integer conversion are new here.

Supported arithmetic results are zero or values in the **normal x87 extended
exponent domain** (-16382 through16383). Extended subnormal results, overflow,
NaN/infinity inputs, division by zero, other rounding modes, and floating-point
exception flags are not emulated. Binary32 stores do implement gradual
underflow and signed zero; overflow returns an error instead of infinity.
Integer overflow returns an error instead of the native indefinite sentinel.

The primitive tests execute234 isolated x87 arithmetic instruction cases across
all three precision modes, plus domain/truncation tests. These are synthetic
CPU fixtures, not evidence for any particular game algorithm. Original body,
movement and RNG call paths have their own fixtures.

Square root uses exact integer midpoint comparison and preserves the extended
result before storage. Its 3,267 cases also match actual host x87 FSQRT results.
Exact remainder aligns the dyadic magnitudes before integer modulo and keeps
the dividend's sign, including negative zero; no host fmod or PC rounding is
substituted. Both remain within the finite normal-extended result domain.

`Fast` (`src/fast.rs`) is the allocation-free equivalent for per-tick code: a
`Copy` value with the same operations, errors, signed zeros and binary32 store
as `Binary`, bit for bit. Values that are exactly zero or a normal binary64
are held as a host `f64`: at `Double53` the host binary64 operation is used
when its result is an exact zero or at least 2^-1021 in magnitude and finite
(then it is the PC53 rounding), and `f64 as f32` is the native second rounding
of the binary32 store. `Single24` does the same with binary32. Everything
else (PC64, 64-bit significands, binary64 overflow and subnormal results)
uses exact `u128` arithmetic with guard bits and an odd sticky stand-in.
`tests/fast.rs` replays every native fixture below and compares both paths
with `Binary` on randomized operands (`AA_FAST_CASES`, millions of pairs:
both exponent-range edges, constructed ties at 24/53/64 bits, chains); the
ignored `benchmark_fast_against_binary` measures about 2 ns per PC53 operation
(13-105x faster than `Binary`) and 18-60 ns at PC64.

Remainder validation uses 3,696 actual-host x87 results. The raw emulator
observations remain preserved: 1,188 wide-exponent cases disagree with real
hardware and exact integer arithmetic. They are documented oracle limitations,
not behavior copied into the Rust implementation. See research/phase5-numeric.
