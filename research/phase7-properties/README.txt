Concrete Properties and Modifier state
=====================================

PropertiesState::new implements5812F0 including4DB020/50AB90 base fields:
owner, unlinked scheduler, empty effect vector with initial growth metadata2,
ModifierState, and zero modifier flags. ModifierState::new implements502750:
25 scalar values(+4..+64), with+30=-1 and+38..+64=1, otherwisezero. Seven
empty vector headers preserve native initial growth hints; the fourth hint is
the loaded AbilityType registry count (B36530+3A8, registry+28). These are
growth hints, not caps. domain-evidence.json pins the original registration,
constructor, count getter, loader and current source-bound catalog enumeration:
repair and attack, count2. Callers derive the count through aa-rules, rather than
hardcoding it. Native registry capacity7 is not its count.

The state exposes real ordered immunity and damage-factor tables. Immunity
501610 accepts the first exact type or native wildcardzero. Damage501650
returns the first exact match or1. Offense501530 prefers positive+58, then
positive+5C, else1. Health5014F0 and other multiplication getters retain their
extended result; recharge501560 and other division getters do likewise.

Defensive/range getters501B10..501DB0 store their adjusted base tobinary32,
floor to min(original base, resolved global minimum), then floor atzero.
501CF0 subtracts+24; other getters add their specific delta. Native fixture
testing caught this exception before acceptance. Named helpers cover defense,
projectile range, bombard range, health maximum, recharge and offense.

420 original executions atPC24/PC53/PC64 compare every constructor field or
exact80-bit getter return and binary32 store. Only the constructor's8-byte
effect-vector allocation is replaced; the getters and Modifier constructor
have no behavioral hooks. Original/v14 function bodies and visited bytes agree.
Two Rust tests verify all outputs and unsupported arithmetic/offset errors.

The five remaining vector entry types are explicit runtime handles, not
inferred gameplay effects. Numeric property activation/removal/recomputation and
Body property consequences now have separate implementations/evidence described
in LIFECYCLE.txt. Complete effect definitions and timed world binding remain work.
This constructor/getter evidence does not claim that rank modifiers are already
automatically applied to every world actor. Root runtime must supply real
definition identities, domain count, scalar state and loaded global minimum.
No native pointer layout, string/heap ownership, exceptional FPU mode or whole
gameplay parity is claimed. Original assets/executables/releases/saves unchanged.

Reproduce: python research/phase7-properties/original_properties.py
           python research/phase7-properties/record_findings.py
