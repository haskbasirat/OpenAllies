Current GWorld / MapResource mode alias
=====================================

This is a bounded native pointer-flow proof and its Rust ownership binding.
It does not prove the whole C++ constructor, MapResource load, first-map
execution, arbitrary callback immutability, or a playable battle.

native_world_settings_alias.py runs fresh independently mapped original and
v14 images at PC24, PC53 and PC64. Its 12 rows / 24 executions pin 86 executed
instructions plus the WorldCreator vtable entry A32170 -> 5126C0. The immutable
fixture SHA256 is 766084fd6960fbe834178a8c0e84bfb2afb8301b5c85b0d00f3f2ae38fd758b5.
Default invocation replays and compares; only --write replaces the fixture.

The actual fragment beginning 5A2E0B passes the same MapResource to creator
virtual+24. Real WorldCreator5126C0 forwards it to Sim76AA90 and real GWorld
constructor4F7D00. The probe stops at 4F7D63 immediately after the native stores
B36534=this and this+28=that MapResource. Logging404F60, allocation6F7810 and
WorldParameters511DF0 are explicit seams. Constructor tail/SEH return are not
executed. These stopped fragments make no full function return claim.

A separate unchanged fragment 5A302A reaches 4FC600 with that global World as
this and the same MapResource as argument, then stops before the Load body.
Actual 4F3BC9..4F3BD8 then reads B36534 -> World+28 -> MapResource+A8 twice.
Rows change A8 between reads (including zero and FFFFFFFF); each second read
uses the changed current word. Redirecting B36534 to a foreign World reads its
foreign mode, establishing why alias identity must be conditional.

The concrete root World now owns a stable Box<Option<u32>> together with
SameWorldSettings. Only its crate-private constructed-World factory can issue
the capability. The capability holds the safe address identity of that actual
owner, never dereferences a saved address, and compares the currently borrowed
owner on each reached global read. Moving the Box does not move its contents.
Foreign/missing owners are errors; no copied scalar can bless the alias.
Local placement reads also use that current borrowed owner. Explicit legacy
Present/Null/Unbound values remain distinct diagnostic inputs.

live_object_init::Services::placement_null now receives the actual mode-owner
borrow at the reached native boundary. load_object_init_world forwards it into
live_placement::MapFacts; it no longer captures a mode word before callbacks.
The task's eventual A8 restoration must write this same owned field. The
concrete managed path currently constructs one World and no implemented child
replaces GWorld. Before adding such a child, root must retire or rebind the
capability. This is an explicit lifetime requirement, not perpetual global
immutability or a promise covering unported callbacks.

Tests and logs are recorded in findings.json. The two focused unit tests cover
all fixture mode reads, Box moves/current mutations, and foreign/null/unbound
identity failures. The affected placement/Object/child tests are rerun once
after the callback signature change. No original, save, manifest or release
files are changed by this bounded task.
