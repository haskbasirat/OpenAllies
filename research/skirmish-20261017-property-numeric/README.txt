Numeric property consequences on actual pre-staging owners
=========================================================

Native authority: Add virtual+30 4FE590; Multiply virtual+30 4FE260.
These operate on the SAME held Properties.modifier and component allocations.
The Catalog's actual AbilityArena is updated in place, primary Performer list
before Defender list. No ActorStore, component clone/copyback or temporary
property-instance arena is used by the runtime helper.

Bound positive leaves:
  power:       4C8250/4BE3F0 ->501A30/501AA0
  defense:     561790/45D760 ->501B10
  supply:      58AA20/45D550 ->501BD0, range store then dirty byte
  detection:   56EA10/45D790 ->501C30, finalized ActorData+1CC
  breakpoint:  578020/45D4F0 ->501CF0 (subtract delta)
  resupply:    570530/45D5B0 ->5014E0 (Element, not Economy)
  health:      561D40/45D730 ->5014F0, actual null/stale/no-Body link
  control:     566F80/45D6C0/45D4C0 and577FD0 ->501520
  offense:     578000 ->501530

The immutable definition source is validated against the SAME Bank/source
precision, independently of live arithmetic precision. The component header's
real owner must match. Authored Detection range is not used in place of the
actual native finalization result. Present but unbound children stay required.

Clamp correction
----------------
ModifierAttributeClampAdd is index69, cache+1A4, default1.
ModifierAttributeClampMultiply is index6A, cache+1A8, default0.05.
The existing ten combat-value cells keep Add unchanged. One separate mutable
Multiply cell is now retained inside the SAME World.variables State. Its exact
Fixed declaration/source span initializes it during the controlled fresh reset;
the actual current cell is read lazily by root's PropertyEffects getter.
No ModifierAttributeClampSub declaration exists or is used.

native_modifier_variables.py executes unchanged401710,4B3580,40E0A0 for the two
separate type2/flags0 variables. Registration5086FE..508783 and names/defaults/
indices are pinned separately as static evidence, not claimed App execution.
Reset/current raw assignments (including nonfinite words) preserve cell identity;
nonfinite rows are storage probes, not claims of numeric evaluation support.

native_numeric.py uses the frozen v1 constructor harness with ONLY its missing
50252F-clamp observer replaced by actual captured cache/current-cell storage.
Original50252F executes. Default0.05 and explicit current0.9 are supplied from
the real Rust World.variables owner, and the complete Properties/Instances
after-state is compared. This qualifies the historical v1 missing-clamp proof:
that proof did not supply a successful SVar and its required-error text had
called it Add. The historical text/fixtures are preserved; the supplied runtime
path now correctly reads Multiply.

Actual-saved scope
------------------
The external property_numeric test restores the original tutorial02 prefix with
actual v14 definitions/completion receipt. This is InspectPartial/direct callback
testing, not a completed battle or naturally applied numeric property claim.
Explicit probes alter scalars on the same live modifier; they do not fabricate
property definitions, component presence or ability records. Native source
values, prior fields, actual handles, link GTID and component presence are
captured. Every admitted source/field needed by a native read is present;
unconsumed absent fields are poisoned and guarded against reads in the oracle.
The actual same property-instance owner is passed through (unused by these
numeric leaves), rather than constructing a replacement arena.

Native runs execute both unchanged original and v14 images at PC24/53/64.
The final external immutable replay compares actual state against BOTH the
captured-input fixture and the freshly reproducible native fixture. Input
capture requires an explicit environment variable and create_new output path.
Both Python producers default to immutable replay; --write is author-only.

Explicit remaining gaps
-----------------------
576C70 Mover speed/group/Mind consequences,578300 maximum-morale recomputation
and positive linked-Body561D40 recursion remain reached required boundaries.
The actual Mover and Organization probes stop at those native child entries;
they are not reported as completed callbacks. Invalid/mismatched payloads,
foreign/reloaded source definitions and unsupported nonfinite numeric inputs
are not given success defaults. Enchantment/Settlement/positive propagation/FX
scope limitations from the prior property-instance checkpoint still apply.

Default/absent-component probes deliberately use an actual Catalog with no
source-completion/Settings owners; native source/cache pointers are poisoned
for the same rows. They finish only because native never reads those owners.

See findings.json and freeze.json for final counts, exact artifact hashes and
author/independent validation. Draft captures/fixtures and failed build logs
are retained separately; they are not final acceptance evidence.
