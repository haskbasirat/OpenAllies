Saved Organization follow notification, 2026-10-16
=================================================

Purpose and source
------------------
Bind original 0057C800 (Organization member position notification), 00578E90
(Mind activity guard), and 0057C680 (goal update) to the actual temporarily-held
saved Organization. The implementation calls the previously proved
aa_actors::element_position::organization_moved core. It does not alter that
core, the native rules, actor positions, or original assets.

Storage and branch order
------------------------
OrganizationLeaderContext supplies the actual Organization, owner Actor shell,
owner Thing fields, and generation-checked registry. Organization+40 is the
selected member identity; neither member-array ordering nor a saved pointer
cast is used to infer it. Actual goal writes stay in Organization.position_xy.

Catalog.native_load_mode is required as the actual GWorld+D8 value. There is
no fallback mode. Mode 6 returns before selected-member reads. A nonselected
notification returns before reading the notified actor or its Mind. No Mind on
the owner returns before reading selected member positions. Selected IDs are
resolved only on native branches that actually need that Actor's fields.

The core's u32 Mind token is an opaque owning Actor GTID for that exact current
Mind slot, never a native pointer or a manufactured bool. Presence alone is
reported while reading an Actor. Only a reached Mind virtual dispatch validates
the slot payload, RestoredMindParts type, actual owned core, owner identity and
installed Mind identity. The adapter never returns a guessed Mind predicate.

For the dual runtime flag-30 branch, native 0057C680 writes both goal words
before calling actual ground relocation 004EEB40. The adapter preserves those
goal stores and then explicitly errors because that live spatial service is
not yet bound here. It does not move the owner's Thing position on failure.

Required services left explicit
-------------------------------
Actual Mind virtual +64/+7C, 004EEB40 relocation, 0057C080 interpolation,
0050B220 queue lookup, and 0050AC00 scheduling remain required errors when
reached. The current adapter stops at the first Mind predicate; interpolation
and scheduler functions cannot currently be reached through a successful Mind
branch, and no such branch completion is claimed.

Saved 0057CBA0 add-member callers provide a nonnull member. For a null external
notification, native mode-6 and nonselected guards are honored. A selected-null
notification is explicitly outside the supported saved-caller domain. Native
can return or fault there depending on owner/Mind state; this adapter does not
claim malformed-pointer parity or silently guess an outcome.

Evidence procedure
------------------
The actual-data Rust test loads original/v14 (2v2)Kursk with an explicit fresh
managed load context, normal mode 1, and elapsed-clock fixture 0. It selects three
actual saved Organization owners without Mind.
At each precision it captures unchanged saved input and four labelled probes:
nonselected unknown GTID, mode 6, selected-member substitution, and dual flag30.
The probes mutate actual saved allocations for the test and restore their
fields afterwards; they are not observed native Init results.

Capture:
 AA_GAME_ROOT=E:\Games\Axis and Alies
 AA_SAVED_POSITION_CAPTURE=1
 cargo test -p aa-runtime --lib saved_organization_position::tests --release --locked --offline -j4 -- --include-ignored

Then:
 python research/skirmish-20261016-saved-organization-position/native_saved_position.py --write

Replay the same Rust command with AA_SAVED_POSITION_CAPTURE unset. Every
captured input is compared to both original and v14 native executions at its
explicit PC24/PC53/PC64 mode. Native code executes until return or the first
required service entry. Mind dispatch, relocation, interpolation and queue
services never return invented success. Native page-zero protection allows
only legitimate FS:[0]. Unknown nonselected Actor IDs are represented by an
unmapped native pointer, proving those branches do not dereference it.

The portable guard test checks missing native world mode, mode-6 laziness when
Organization+40 is unavailable, unknown/nonselected GTIDs, no-Mind early exit,
reached identity/payload errors, and selected-null refusal. Invalid registry
or payload cases assert portable refusal, not native invalid-pointer behavior.

Two initial capture attempts assumed an actual Mind Organization owner had a
nonzero +40 or nonempty restored members. Neither is available in the current
partial restore. Those failed fixture-selection assumptions are preserved in
capture-selection-assumption.log and capture-mind-members-assumption.log. The
corrected capture uses three real no-Mind owners. Separate, explicitly labelled
isolated native Mind-presence probes stop at virtual+64; they are not compared
as observed saved Mind allocations or installed across Rust actor owners. No
Mind predicate is answered. Virtual+7C remains an unexecuted required boundary
in this bounded adapter because native first requires virtual+64.

Scope
-----
This advances saved loading where the native source-free early exits apply.
It does not complete immediate Actor Init/PostCreate, world publication,
simulation, spatial relocation, skirmish battles, or multiplayer. No vendor
SDK implementation was copied or integrated.
