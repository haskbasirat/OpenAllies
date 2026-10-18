FINITE X87 SQUARE ROOT

Binary::sqrt(Precision) rounds the exact nonnegative dyadic input to the selected
24-,53- or64-bit significand with round-to-nearest/ties-to-even. Negative zero is
preserved. Negative nonzero input is an explicit error. Exception flags and
other rounding modes are not modeled; this matches the existing numeric scope.

Algorithm
Given x=M*2^e, let t=floor(log2(x)) and k=floor(t/2)-(p-1). Compute the exact
rational scaled square x/2^(2k)=N/D. The lower integer root is
q=floor(sqrt(floor(N/D))). Compare4N against D*(2q+1)^2 to decide which side of
the exact halfway point sqrt(N/D) occupies. An equality rounds to even. Restore
the scale2^k and use the existing normal-extended exponent/precision checks.
The production code uses only integer arithmetic and the BigUint integer root.

The synthetic oracle executes isolated x87 instructions. It records the full
80-bit result before any binary32 store, which is essential for native distance
comparisons and normalization. Cases cover subnormals, exponent ranges, exact
squares, PC24 midpoint neighbors/even-odd ties/binade carry, signed zero and
sum-of-squares operations. Original game control flow is validated separately
by the actor/navigation native instruction fixtures.

3267 emulator cases match the Rust extended result and binary32 store. All3267
extended results also match actual host x87 FSQRT in tests with the caller's
FP environment saved/restored. Additional tests cover negative-domain rejection
and extended exponents beyond binary64. Earlier234 numeric cases remain green.
This pass does not change the existing arithmetic methods or default precision.
Independent review and final workspace validation are recorded separately.
