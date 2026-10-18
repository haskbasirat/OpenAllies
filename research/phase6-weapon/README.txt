Native combat callers and periodic Properties reconstruction
===========================================================

This component ports the finite-input weapon hit and periodic property paths
into aa-combat. It dispatches real aa-body healing and damage in tests. It does
not claim a playable combat world, attack selection, range/cadence/projectile
simulation, complete properties registration, or automatic scheduler wiring.

Weapon hit: 004CA400
-------------------
The target must pass the native actor-eligibility query and have a Body. Actor
handles are resolved to generation-bearing IDs at dispatch. Kind 4 takes its
power from the action/weapon instance +0x20; other kinds take rule +0x18. The
base is clamped at zero. The defender's 00501650 factor is stored to binary32
before later arithmetic. Kind 4 then evaluates, in this order:

  ((offense * base * scale) - nonnegative defense) * type factor

Each operation uses the selected native x87 precision; the final result is
stored to binary32 before the CombatDamageMinimum comparison. Definition +0x54
bit 1 substitutes offense 1; bit 0 substitutes defense 0. Other kinds use only
type factor * base, with no weapon minimum. Offense comes from 0057DBF0: a leader
factor when present, otherwise 00501530 chooses positive primary, positive
secondary, or 1. The leader factor is allowed to be nonpositive.

The resulting seven-word damage record preserves source, allegiance, type,
amount, time and event context. Reason is 4 for kind 4 and 5 otherwise. Body's
apply_type_modifier is false, because this caller already applied that factor.
Passing true would multiply twice. The weapon identity and neighbor-propagation
definition are preserved. A successful dispatch can still be ignored by Body
when the damage is nonpositive.

The oracle executes the original 004CA400, 004C8330, 004CF110, 0057DBF0,
00501530, 00501650, 005625C0 and 00561E50 instructions. There are 486 cases,
including all three precision modes, fractional values, weapon flags, no
source/target/component, leader and modifier choices, defense/minimum clamps,
wall propagation, lethal damage and health feedback. Rust compares complete
request fields, final Body/environment state and ordered Body events, using
the actual aa_body::damage::deal_damage implementation.

Periodic Properties: 005818D0
----------------------------
Expiry and removal preserve native 005034F0/005032D0 behavior. Selection uses
property identity plus generation-resolved source identity, so two stale IDs
can compare as null. Removing an enabled effect reenables the first remaining
same-property effect; removing a disabled effect does not trigger a complete
modifier rebuild. With no replacement, the native sequence recomputes flags
when the removed property's flags are nonzero, notifies actor and effect,
rebuilds derived stats, then releases the effect. The loop increments after a
removal and therefore skips the next shifted entry.

After expiry, healing respects owner, outer and ancestor suppression flags.
Healing precedes periodic damage. Each phase samples its count at the original
boundary; callbacks can rebuild derived arrays before that phase starts.
Periodic damage reads world time per item, sets event kind 2/reason 10, and
enables Body's type modifier. No weapon is fabricated. Tests call the real
Body::heal_plain and damage::deal_damage services.

The 90 native cases execute the actual removal, flag, getter, healing and damage
instructions, covering duplicates, disabled/stale effects, shifted-entry skip,
recomputed arrays, owner/outer/ancestor suppression, no Body, fractional and
nonpositive values, lethal damage, and health feedback. The native stack
address used as an opaque healing-event record token is normalized to token 1
on both sides. No damage or health numeric values are normalized.

Damage-type presentation: 004C0A80
---------------------------------
This function loads display color, glyph and damage animation group. The
authored template explicitly labels damage_filter as an animation grouping;
it is not an armor equation. Eight native cases cover four authored RGB/filter
combinations, absent glyph, negative filter, and integer limits. Glyph handles
are explicit fixture values. The base definition service, TGI getters and glyph
registry are required callbacks. Invalid glyph/filter cases stop at the native
00410C40 exception call and compare partial writes and callback order; Windows
exception unwinding is not simulated. Finite RGB values are copied without a
range clamp; nonfinite colors are explicitly rejected by the Rust API.

Required boundaries and limits
------------------------------
World traits require actor/component/definition lookup, target eligibility,
offensive factor resolution, Body routing, effect expiry, graph resolution,
stat rebuilding, native time, and external actor/component effects. Fixture
callbacks are labeled as such. The copied Body test-world adapter originates
from crates/aa-body/tests/native_damage.rs; its callback implementation is test
support, not a substitute for a live world. Body health/damage arithmetic is
provided by the real library and also executes in the original oracle chain.

The domain is finite numeric data and valid typed handles. A kind 4 hit without
a weapon is an explicit error instead of a native invalid pointer. Effect
membership and fields stay stable during expiry/source inspection. Removed
effect callbacks may rebuild the derived healing/damage vectors. Shrinking a
list during an already sampled phase errors after earlier effects; no rollback
is promised. Recursive ticking of the same Properties object and arbitrary
raw-memory alias mutation are not supported. The two Rust failure tests verify
that a removal callback failure stops later work and that shrinking a sampled
healing list preserves earlier real effects while preventing invalid reads.

Reproduction
------------
From rust-rebuild, with the original AA.exe and accepted v14 executable present:
  python research/phase6-weapon/native_weapon.py
  python research/phase6-weapon/native_periodic.py
  python research/phase6-weapon/native_damage_types.py
  cargo test -p aa-combat -- --nocapture
  cargo clippy -p aa-combat --all-targets -- -D warnings
  python research/phase6-weapon/record_evidence.py

The producers check original/accepted executable hashes, compare every visited
instruction window, and check stack/nonvolatile-register preservation on normal
return. Visited windows include intercepted seam entries, not a claim that the
whole intercepted service ran. Producer/dependency/source hashes are embedded
in the fixtures. findings.json pins the Rust source and recorded validation.
