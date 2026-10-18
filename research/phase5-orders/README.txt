Phase5 generic command and current-player reconstruction
=======================================================

Source: exact original AA.exe and accepted v14 native image, never modified.
Original SHA256 5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26
V14 SHA256 3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1

The original 357-entry registration proof remains Phase4. This phase implements
wire IDs84/86 (selected request/decree),85/87 (actor request/decree),320/298
(current-player request/decree). Request authorization/version/film acceptance
and mirror/recording are NOT ported. Only accepted decrees may reach process APIs.

Native serialization
--------------------
0x61E6B0 writes and0x61EBA0 reads a native command. Constructors0x4D9C80 zero all
semantic fields. Mask is11 bits; the field order is deliberately not bit order:

bit6 command definition at+8: index in[B3AA58]+24/count+28, count is null sentinel.
bit0 raw32 word at+10.
bit1 object definition at+14: B39F58 registry, then virtual+38 validity on resolution.
bit2 two raw32 words at+18/+1C.
bit3 two raw32 words at+20/+24.
bit4 raw32 word at+28.
bit7 parameter definition at+2C: B3AB08 registry.
bit5 parameter set at+34: null bit (one means null); if present, B3AB00 definition,
     signed32 nonnegative entry count, then object-definition references in order.
bit8 extra definition at+38: B3AA2C registry.
bit9 one boolean at+30.
Always one final boolean at+3C.
bit10 has no further field in either native function.

Each registry index uses native bit count for count+1 possibilities. Rust exposes
wire indices and null sentinels, separating actual pointer/type resolution. The
parameter-set constructor0x4C2110 dereferences its definition; decoded null or
missing registry objects must not be materialized as valid sets by a host.

Actor suffix0x76A500/0x76A5A0 is16 bits, the low16 slot from actor+14. It indexes
actor registry+20004 and calls virtual+B4. No high16 generation appears on wire.
Current-player is one raw32 ID. No string alias parsing or player-name identity.

Native process boundaries
-------------------------
0x61E220 and the inline equivalent in actor decree0x61F970 check nonnull actor,
call virtual+150(null player, &command, selection=false), then on success query
linked GTID via0x44DEA0. A valid link calls0x4D1900 then0x4DFF30; delivery0x4D9B60
always follows successful permission, even without a link. The permission callback
may replace the smart-pointer command. Final delivery return is ignored.

0x61F210 gets admin+284 current player, then player+50 selection. Empty head rejects.
The first-actor/default-definition diagnostic branch remains an explicit inspection
seam. Nonempty selection ultimately calls0x4ED360 as one operation. That routine's
formation, default-command choice, ownership, filtering and world delivery are not
ported here. Rust does not invent a per-actor broadcast. Missing current player is
a typed Rust safety error, not the native null dereference.

0x75D710 calls0x76B8F0, traversing session+BC until both player+20 ID and player+E
film flag match. A live match on the authorizing admin (0x74B900) checks player+4
client against admin+280. Mismatch returns without mutation. Otherwise it stores
the player at its client+2C via0x74B350, then admin+284 via0x74B3A0. Film players
skip client binding. Missing live ID stores null current player; missing film ID
calls0x751EB0 then sets admin+2E5=1. The fixture mutates state in film-stop to prove
the final abort store and absence of an invented current-player rewrite.

Oracle scope
------------
native_commands.py runs actual original instructions in isolated Unicorn:
309 complete write/decode cases: each mask bit, composite masks including bit10,
all starting offsets0..8, nulls, registry counts0/1/3/5/7/8/16, parameter lengths0/1/2/3/8.
18 slot serialization/resolution cases including0/1/65535 and validity callback.
5 actor guard cases, including permission replacing the command.
3 selected boundary cases: empty, nonempty/null-first, nonempty/actor-first.
7 current-player cases: owner/nonowner, nonauthorizing, film/live/missing and mutation.
3 actual native mixed packets298->87/86 via0x74D710 and native decree handlers.

The oracle checks callee-saved registers/stack for entry calls and records visited
native byte windows; originals and v14 are identical in every recorded range.
The compatibility field named executed_windows includes intercepted seam-entry
bytes recorded before the hook replacement. Those pins establish visited byte
identity, not execution of the original instructions behind a declared seam.
Allocator/free/CString construction, diagnostic sinks and clock are seams. Runtime
registry content is explicit synthetic input. Slot/object virtual type checks,
permission/group/delivery, selected world operation and film-stop are named leaves.
No process, graphics, network or original file is modified. Final padding framing
uses the already-proved packet contract, not a newly ported native finalizer.

Rust limits
-----------
References and commands must be resolved by the host before gameplay process APIs.
No ownership, actor initialization, pathfinding, tick cadence or new authority is
inferred. There is one sequential dispatcher; no parallel authoritative stream.
Negative parameter counts, malformed indices and truncation return typed errors.
The caller must choose a parameter-entry budget; zero-bit references otherwise
cannot be bounded by packet length. Allocations use fallible reservation. No fixed
game-unit cap or global memory guarantee is introduced. Failures are nontransactional.

Run cargo test -p aa-orders and cargo clippy -p aa-orders --all-targets -- -D warnings.
Phase3 and Phase4 native fixtures/ledgers remain immutable historical evidence.

Reproduce this phase with python research/phase5-orders/record_evidence.py.
findings.json pins source, original fixtures, command output and dependency hashes.
select_client_samples.py selects six unchanged source cases for each orders/spatial
diagnostic bundle. The Rust client_samples.rs adapters are fixture-only checks;
their callback/registry data must not be treated as real world implementations.
Client compilation and diagnostic execution are recorded separately by the client
integration owner; the crate evidence alone does not claim a working game UI.

NOTE (lead, 2026-10-14): superseded on negative parameter-set counts. Phase 8
oracles (research/phase8-orders) show the original accepts negative counts:
it reads no entries and writes the count back unchanged. See
aa_orders::commands::ParameterSet::negative_count.
