ORIGINAL ANGLE TABLES AND ROTATION

This is a bounded simulation primitive, not navigation or a movement policy.
All APIs take original binary32 bits and an explicit x87 precision mode. The
caller must resolve the native game's actual floating-point control lifecycle.

LOOKUP AND ROTATION
The original startup constructs 4096-entry cosine and sine tables using x87
FCOS and FSIN. research/phase4-trig/original_trig.py executes the unchanged
constructors and preserves their binary32 table values. The tables are identical
for the three tested precision modes. The fixed 32 KiB byte table is included
in the Rust binary; there is no dependency on a generated user cache.

lookup_cos/lookup_sin reproduce 85DA10 wrapping and truncation using aa-numeric
finite arithmetic, including each native precision boundary. There is no
interpolation, host trigonometry call or replacement normalization. rotate
reproduces 4454A0/4454E0, with native sine storage and independent XY stores.

Native lookup does not bounds-check its computed index. For very small negative
angles, rounding in PC24 can produce index4096. Rust explicitly returns
InvalidLookupIndex instead of reading undefined adjacent memory. Huge inputs
outside signed32 truncation range and nonfinite inputs also return errors.
These are safety-domain differences, not a claim that the native access is safe.

SIGNED ANGLE
signed_angle uses a tightly scoped x86_64 compatibility block for original
445400/96D780 cross/dot/clamping, FSQRT/FPATAN, endpoint constant, conditional
binary64 round-trip and quadrant correction. It saves/restores the caller's
x87/SSE state. The incoming sticky precision-exception flag is explicit; no
whole-process default or global FPU-state emulation is implied. Other CPU
architectures return UnsupportedPlatform rather than approximate the result.

The original CRT skips its sticky-flag-dependent binary64 round-trip in PC53.
Consequently a movement adapter can support that explicit mode without guessing
the exception status inherited from preceding movement arithmetic. PC24/PC64
integration requires that status to be tracked before they can be enabled there.
The standalone primitive still accepts all three modes and a supplied flag.

EVIDENCE
native-trig-vectors.json binds original/v14 code ranges, tables,1452 lookup,
168 angle and480 rotation samples. Two lookup and eight rotation samples invoke
the native out-of-table read; Rust tests require their explicit error instead.
The angle-specific research adds sticky-status and actual-host instruction
checks. Unicorn's incomplete precision-flag behavior is not treated as proof
that the real game's flag is clear. See the independent review ledgers under
research/phase4-trig and research/phase4-trig-angle for exact coverage and hashes.

These checks establish bounded operations, not universal FPATAN equivalence on
every historical CPU, a full game tick, multiplayer determinism or complete
movement parity. Native command routing, path production, collision, scheduler
and spatial commits remain separate required systems.
