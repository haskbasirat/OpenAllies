# Native routing preparation and SAI readiness

`aa_navigation::preparation` implements the fresh loaded-world work of `7ABC30`.
It uses the separately verified source topology to rebuild ordered neighbors,
store signed native rectangle areas, mark bottlenecks and allocate the original
triangular unknown-route table. `StaticMapBottleneckDetectionDepth` is an explicit
caller input; its original authored value is5. The GetSVar argument3 is the type
tag, not that value.

`7BB1C0` visits neighbors in reverse order, marks on enqueue, and does not expand
the excluded node or foreign-island nodes. `7BC8B0` checks pairs of same-island
neighbors with the current region removed, retaining the bounded-depth behavior
and existing set flags. `53F0A0` initializes N-1 rows, row i containing N-i-1 FF
bytes. Those bytes mean unknown routes. This does not implement later flag1
cache population or declare a cached route successful.

The preparation factory returns a fresh unpublished owned candidate. It does
not clear a currently published native world's raw pointers. Its graph remains
`ready=false`: the native manager's prepared byte+5 is distinct from SAI+20.

## Actual outer startup

`aa_navigation::startup::initialize(&mut State,&mut World)` ports `7A0D40`.
State preserves ready, world revision and the two native +6C/+70 words. Every
World service is required; no successful no-op substitutes for an unimplemented
Kingdom/AI/actor effect. Root's actual runtime must bind these to live owned data.

When initially unready, native code clears lists78/80, reads globalB3CC94 into
revision+24, prepares topology, registers each nonnull major Kingdom and starts
each live AI slot. The ready byte is written at7A0E25, before registry actors are
walked. Eligible special actors register through7A0C20. For B0 actors, actual
seen records, readiness, context virtual+38, context words+548/+534, region lookup
and region/Kingdom state all gate registration. Original null-skipping collection
and registry order are retained, including callback changes and field rereads.
Finally +6C/+70 become FFFFFFFF. Already-ready input returns without those effects.

A Result error preserves earlier effects, including a possible ready=true write.
The runtime must require successful initialization and an unstopped lifecycle
before authorizing its RegionGraph. It must not retry failure based solely on
the native ready byte. A callback can also clear readiness; successful return
does not override that native field mutation.

## Evidence and limits

`prepare_oracle.py` executes fresh original7AA9B0 then complete7ABC30, including
actual adjacency, area, bottleneck and cache routines. There are42 small cases
(seven source graph cases at depths-1/0/1/2/5/9) plus Berlin, Crete and Midway
source topology. Allocator/free, version framing and stream transport are seams;
the resolved SVar cache is supplied explicitly. Two Rust tests compare exact
neighbors, areas, flags, table lengths and complete table-byte hashes.

`startup_oracle.py` executes whole7A0D40 and original Kingdom/ID iterators in17
cases. Actual world-service and virtual callbacks are explicit fixtures. Cases
cover early ready, sparse arrays, ordinary/special actors, all seen-record gates,
mutable list lengths, flags, readiness and post-virtual context values. Native
stack and nonvolatile registers are checked. Two Rust tests compare state/event
order and failures before/after the native ready write.

These are integrated library operations, not complete live AI or startup. The
actual cleanup ownership, per-region Kingdom initialization79E170/7BC130,
AI-slot startup7B4830, special registration7A0C20 and seen-record persistence
remain required live services. Source topology, Game SVar resolution and actual
registry identities must already exist. Old-world static analysis and dynamic
terrain topology edits remain separate. No full-match or save-restoration claim.

Reproduce with the two Python producers, `inspect_native.py`, then
`record_evidence.py`. Original source game/assets/saves and releases are read-only.
