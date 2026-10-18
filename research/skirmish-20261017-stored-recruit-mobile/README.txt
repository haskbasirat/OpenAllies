Fresh Recruit581F30 and MobileDeployment5757C0
============================================

The requested581F30 is Recruit slot16, not Research. Research slot21 has
native InitExisting6A71D0 (MOV AL,1;RET), already bound by the actual factory
and owner-checked generic path. This change does not invent a Research body.

component_virtuals now exposes narrow RecruitInitWorld/MobileInitWorld
services, factors the existing exact arm algorithms once, and forwards the
old ComponentWorld API through them. Existing native_component_leaves replay
passes unchanged. The new concrete stored_recruit_mobile_init_world uses the
actual same Staged ActorStore components, canonical moved Bank and source PC.
map_actor_init_existing delegates only the two actual specialized slots.

Recruit uses the existing completed+4EC source-list producer, preserving
target order/duplicates. Each target's actual transport ActorData184 class is
read immediately before setting the SAME Recruit10 or11 byte to1. Empty
lists preserve existing raw bytes. No queue, RNG, World version or unrelated
callback is required by native581F30.

Mobile reads actual Actor100 bit26 /104 bit4 (4D1050). A false result reads no
delay/queue/clock fields. Positive reads immutable ActorData554 at actual Bank
source precision:4624B0 constructor zero on the forward-unpack branch, or
4685C0's typed unpacking_length on the empty-forward branch. It captures that
delay once, resolves the constructor's SAME existing Updatable owner, checks
the real queues' begin/step projection against the retained World clock, and
calls actual deferred queue3 add_after. No new owner, copied queue/RNG or fake
pending event is created. On scheduling error the previous component14 stays.
After success the kernel reads current WorldE0 and stores delay+E0 at14 using
the explicit live precision. Queue arithmetic remains the existing aa-world
kernel. Native deferred scheduling consumes no RNG draw here.

Proof
-----
native_recruit_mobile.py defaults to immutable compare. Original/v14 all3PC
execute72 Recruit runs,360 Mobile runs and actual4D1050/529C30 getters, with
727 genuinely executed instruction pins. Actual primary and embedded
Updatable vtable words are checked (including true Research21 dispatch).
Actor constructor554 executes with explicit completed Object49C760 transport;
this is not a whole Object/ActorData source-load proof.

Only50ACC0 is a named other-owner callback. Its captured args/failure prefix
and mutations prove that delay is captured before the call, and WorldE0 is
reread after it. Queue storage behavior is covered by the existing queue
oracle, not fabricated in this fixture. Finite numeric inputs only; no FP
status, SEH/allocator failure or whole actor/World readiness is implied.

New aa-actors native_recruit_mobile_init tests2/2 pass; the unchanged
native_actor_creation_leaves integration test1/1 also passes. Fresh immutable
native replay passes. Final runtime production check log is pinned separately.
The actual Staged concrete helper is compile-checked, not claimed as a full
source-qualified fresh Actor integration run. No whole mission/battle claim.
