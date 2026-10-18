PER-RECORD LOAD / INIT LIFECYCLE AUDIT -- 2026-10-16
Independent read-only audit by /root/restore_review.

Finding
-------
The saved loader is missing the immediate post-Load lifecycle of4FAE80.
This is earlier than the global PostLoad(+DC) pass, and earlier than later
Organization member attachment. It applies to Teams before Kingdoms as well
as Kingdoms before static/dynamic Things. Merely zeroing Kingdom flow arrays
does not implement this missing lifecycle.

Original/v14 vtable bytes agree:
                  +D8 Load   +E8 Init   +EC PostCreate   +DC PostLoad
Team A29780       4E6970     4E68C0     4D1E10           943800
Kingdom A299D8    4E8550     4E5DF0     4CF300           4E4220
Actor A273F0      4D8CF0     4D7230     4CF300           4D3560
Thing A2A680      4EF160     4EFA40     4D1E10           943800

4FAE80 order, after successful construction
-----------------------------------------
4FB2D9: virtual+B4 predicate; actor-like objects receive argument3 at+138.
4FB2F7: virtual+D8 Load(stream).
4FB300: current GWorld+2C<23 has an older-footprint compatibility path.
4FB4C4: reread actual Thing+8 bit7.
  bit7 clear:4FB574 virtual+E8 Init(NULL); only low AL is success.
  false:4FB582 diagnostic path,4FB5FD4F5490 destruction,4FB602 null result.
  bit7 set:4FB4D3 clear bit31,4FB4E4 enqueue via542A00 on world+110.
4FB4EF: reread object and bit7 after the preceding call.
  bit7 clear:4FB500 virtual+EC PostCreate(NULL,1), then4FB50F4F9510.
4FB51C: if object remains and virtual+B0 true,4FB5344296B0 on
  definition+BC (object,1), including the ordinary deferred-bit7 path.
Then return/cleanup; the next saved record is not loaded before these calls.
The false-Init destructor path is statically pinned, not fully emulated here.

4FC600 order is independently pinned from instructions and literal strings:
  TEAMS stringA2B208 ->4FCACA4FAE80 for each present Team.
  KINGDOMS stringA2B1DC ->4FCD524FAE80 for each present Kingdom.
  Then4FCF17/4FCF2E/4FCF454FC340 for required/static/dynamic Thing chunks.
The later4FD3B3 pass dispatches+DC; it does not substitute for+E8 or+EC.

Kingdom Init prerequisites and effects
------------------------------------
4E5DF0 first calls Actor4D7230 with the actual Init argument. If that returns
false, Kingdom Init returns false before touching the flow arrays.
4D7230(NULL) includes GObject4EBE50, required-object4D1FD0, component+8
InitExisting in slot order with Mind last, then every component+14, required
object attachment,4D4CF0,4D1350,4D67F0, applicable Properties updates,
4D4130, component-specific refreshes, spatial/discovery/SAI/track/FX work.
These are actual dependencies, not successful no-op leaves by assumption.

After Actor Init succeeds and logging:
*4E5E91 checks CURRENT GWorld+2C<22; if Kingdom+184 is null,4E5EA6
  calls50AC00(0) on Kingdom+180 before the flow reset.
*4E5EAB..4E5FB5 ensures indices0..ResourceCount-1 in each owned vector
  +1A0,+1AC,+1B8 and writes zero, interleaved in that order per resource.
  It grows counts as needed; it does not truncate an existing longer vector
  or erase its tail. Resource count is reread at the loop boundary.
*If Team+1EC exists, prepend this Kingdom to Team+44; initialize tail+48
  if empty; increment Team+68. This is not merely a saved Team reference.
*Enumerate eligible source ActorData, call4E1810, add recruit items through
  4E4C30, then conditionally79E170 SAI creation for a major Kingdom.
*4F8AF0 registers Kingdom into actual world+148(all) and+154(major),
  increments world+144 and applicable nation count; preserves native order.
*Traverse current major Kingdoms and conditionally refresh their+228/+22C
  services via54EBA0/559600 after virtual+138 relationship checks.
Only then does Init return true and the outer PostCreate/registration run.

PostCreate/registration still matter
-----------------------------------
Kingdom4CF300 calls base4EBA40, every present component+C in slot order,
then applicable owner4E6180. That owner insertion can itself refresh economy
and update HQ/Organization/class lists.4F9510 clears bit31, registers names
and class-specific world memberships (Team goes into world+16C), performs
actor state publication4D62F0/4CFEA0 and4F4C40 as reached. These are not
equivalent to having registered the saved GTID during construction.

Rust mismatch and required boundary
----------------------------------
At audit time restore_world_entity in aa-mapworld/src/restore.rs marks Team
and Kingdom AwaitingPostLoad directly after their derived saved fields load.
source_stage then advances to the next record. Dynamic/nested record Load
also marks AwaitingPostLoad without a shared4FAE80 finish driver. Staged's
later post_load executes+DC only. There is no successful saved Init marker
which would justify assuming the flows have already been initialized.

The correct integration boundary is immediately after each successful+D8
Load and before record return/next-record processing, including recursive
LoadThing calls. It needs actual Thing flags, source class virtual dispatch,
world deferred-init queue, owned spatial/components/queues/Team/Kingdom state,
and the real null creation-argument distinction. Missing services must stop
there explicitly; a loaded-prefix diagnostic may still retain its state but
must not be presented as having completed native record initialization.

Do not call actor_creation::init_existing with a fabricated default block:
that existing API documents a nonnull creation path and unconditionally
assigns+138 from its block. Native saved Init(NULL) skips that assignment and
uses the null branch of the Existing-FX condition. A null-aware adapter or
separate proven path is required.

The borrowed flow adapter can operate on genuinely initialized owned arrays.
It must refuse absent/short arrays. Place the native reset only inside the
real Kingdom Init path after successful predecessors. Do not put a fallback
resize/zero operation in Economy refresh728 or an attachment callback.

Evidence and limits
-------------------
static-proof.json pins actual original/v14 bytes, vtables, instruction windows
and current Rust source hashes. Pseudocode was used only as a navigation aid;
the byte-checked instructions are authoritative.
native_loadthing_lifecycle.py produces60 bounded original/v14 executions
across PC24/53/64,67 native instruction sites, with real vtable targets and
recorded Load/Init/PostCreate/queue/register/FX leaves. It exercises normal
Team/Kingdom/Thing dispatch, bit7 decisions, AL-zero failure, and explicit
post-callback flag-mutation probes. It does not execute those leaf bodies,
whole4FAE80 constructors/readers, pre23 shape compatibility, SEH or destruction.
Page zero is unmapped. Existing phase7-runtime native-kingdom-flow-init.json
has40 reset-fragment cases; those are evidence for the reset only, not proof
that the lifecycle was wired or that its earlier dependencies succeeded.

No production edits, Cargo builds, original game changes, full-map lifecycle
execution, battle, multiplayer or release validation were performed by this
review. Pure Team/Kingdom coordinators and runtime integration have separate
owners. This audit does not approve those future implementations.
