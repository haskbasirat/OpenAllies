# Saved Area, Camera and DynamicDoodad Init

The runtime binding borrows each actual held record's Object shell and derived
allocation. It calls the existing concrete Object4EBE50 service before the
derived Init tail. World mutation generations, the current RNG, the load queues
and their Updatable instance owner table retain one allocation across records.

`native_special_classes.py` executes the three real derived constructors with
explicit base-constructor and empty-string allocation seams. It also executes
their actual runtime/Data virtual predicates and Object potential-visible and
memory predicates. ActorData+174 reads are forbidden for these Object classes.
The World constructor's camera/area generation initialization is pinned and
executed separately. The evidence has231 rows,456 separate original/v14
executions at all three x87 precision settings and208 instruction pins.

`native_special_tail.py` executes the actual derived Init functions,85D840 RNG,
5904A0 period arithmetic,50ACC0 owner-head insertion,58F960 state guard and50B250
owner/bucket unlinking. Object Init success/failure, diagnostic logger transport,
successful allocator/free and deferred queue bucket allocation are explicit
services. The deferred callback records owner/delay/data and returns an actual
node; it does not answer RNG or delay arithmetic. The full periodic/deferred
queue arithmetic has its own existing aa-world evidence. The126 paired rows
cover252 original/v14 executions, all three precision settings and216 pins.

The runtime reads source update-time before drawing RNG, preserves the
unspilled x87 fraction until its product is stored to binary32, and leaves
earlier state/RNG/queue effects intact on a later required-service failure.
Breakable state0/1 cancels the current authoritative queue owner list after
scheduling. Constructor/load head receipts do not substitute for the scheduler
owner. A null component does not demand a source definition.

Missing source definitions, a missing queue, mismatched live queue precision and
legacy Area scheduling remain explicit reached errors. The current shipped
map's World version27 does not enter the pre-version22 Area schedule path.
No complete map, playable battle or full engine readiness follows from these
bounded Init tests. Allocator failure, corrupt native pointers and concurrent
native reentrancy are outside this receipt.

Both producers are immutable by default and compare their output with the saved
fixture. `--write` explicitly regenerates their author fixture. Original and
v14 game files are only read; all executed original instruction bytes are
compared with both game images.
