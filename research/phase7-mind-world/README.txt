Live Mind runtime boundary — Phase7

mind_world.rs owns plain command references, native embedded state bindings,
Element simple state lifecycle/Stop dispatch, ground Mind InitCreate/InitLoad,
and concrete Element scheduling over canonical ActorAccess/Arena/Owners/Queues.
No temporary empty Arena or duplicate mutable component/state allocation exists.
Movement payloads live in movement_world::Runtime, embedded in this Runtime.

New original executions:
166 simple state/lifecycle/scheduling cases: can/defers, Enter/Leave, state update,
InitLoad/default reset including actual Sleep activation/reentry guard and
priority rejection, concrete InitCreate slots, Element52DC10 and Regiment50AD80.
Native code executes timer cancellation529CD0/50B250, actor-active predicate
4DB0F0, world+208 selection, old intrusive owner unlink, and new owner linking.
Queue allocation/admission, actor-state effects/default stance/rebuild and absent
group predicate are explicit observed leaves; they are not claimed complete.
32 no-hook command4D9CE0 copies prove every initialized plain field. Native
constructor initially has zero references; runtime clone_owned returns the one
owned reference required by its already-proven caller. ParameterSet+34 absent
is the bounded domain. Owning parameter objects cannot be represented here.
18 no-hook moving virtual+64 cases: all Element/Regiment/Aircraft status values.
All visited instruction bytes match unchanged original and accepted v14 images.

Rust tests: three tests passed, including164 real ground ActorStore cases against
the native fixtures,32 plain copies,12 ground moving cases; plus shared-guard
reentry, committed service failure/recovery, Stop/current-command ownership and
invalid command identity handling. Two Aircraft lifecycle and six Aircraft
moving native rows are source proofs, not ground-store execution claims.
Actual source usa_infantry and regiment_usa_infantry are assembled with one RNG,
canonical ActorGTIDs, installed Mind states, and initialized five native queues.
Strict scoped Clippy passed.

Required live services still include state effects4D64F0, full action rebuild,
Character default stance4CFC80, derived Move/attack/other state entry/exit,
non-Element default reset, non-Element state synchronization, group completion,
and command handlers other than Element Stop6. Actor authorization remains
upstream. Aircraft initialization and scheduling explicitly error until bound.
World effects failures preserve prior committed writes and release the global
Element default-reset guard; a caller must stop or recover the partial world.

The Mind scheduler uses deferred queue4 (world+208), cancels existing entries
before replacement, suppresses scheduling while current Performer stage!=4,
and invokes default stance before positive-delay admission if current Mover
has no pending timer. This is not a replacement AI cadence or a fixed delay.
