RNG NATIVE RECOVERY

The original RNG's integer transition was already ported. This pass implements
85D870 default construction,85D880/85D8A0 seed mixing,40C8A0 bounded integer
draws and the isolated4F62AC..DF binary32 float conversion boundary.

NativeRng::new continues to restore a raw state. NativeRng::from_native_seed and
reseed deliberately perform the native seed mixing instead. The two meanings
must not be exchanged when loading state or constructing a new generator.

Bounded draws use one native draw followed by modulo; no rejection-sampling
algorithm is substituted. Bound zero is rejected before mutation rather than
emulating the native draw followed by an integer divide fault.

Unsigned-to-float conversion first loads a signed32 integer, conditionally
adds the original binary64 2^32 constant, multiplies the original reciprocal of
u32::MAX and stores binary32. Each arithmetic operation honors the supplied
PC24/PC53/PC64 precision. The separately rounded addition matters in PC24. The
output interval includes1.0; no extra clamp or exclusive-upper-bound adjustment
is made. Call sites that retain x87 intermediates must not substitute this
explicit-store helper without recovering their actual store boundary.

original_rng.py executes unchanged original/v14 instructions and emits11345
cases:330 constructor,1100 bounded,9585 conversion and330 draw-plus-conversion.
It records code/constant and source-image hashes. The fixture tests compare
result bits and retained state. Independent review is recorded at
research/phase4-orders/rng-independent-review.json.

Global seed sources, generator ownership, authoritative draw ordering, engine
FPU-control lifecycle and multiplayer/save integration remain unimplemented.
This does not establish complete replay or game determinism.
