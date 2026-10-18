# Native region search and prepass

`aa_navigation::region::prepass` ports complete non-diagnostic `53DC00` behavior.
It consumes the result of the required `53F320` region graph service and produces
the native kind-2 path nodes used ahead of tile routing. It does not invent a
region graph, infer connectivity from renderer meshes, or treat the graph
provider as an always-successful straight-line route.

Native region topology is derived during GWorld loading. The 80 inspected maps
have TERRAIN, HEIGHT_MAP/MOUNTAIN_HEIGHT_MAP and actor chunks, but no separately
named region/passage chunk was proven. The native manager at `DAT_B3652C` stores
a region pointer array at +0x14, count at +0x18, and traversal epoch at +0x3c.
Nodes expose ID+0, center+0x54/+0x58, predecessor metadata+0x6d/+0x6e, adjacency
count+0x70 and ordered neighbor pointers+0x74. Actual topology construction and
world-to-region lookup `7AA3D0/7B9300` remain required services.

The graph entry `53F320` writes output/start/goal/cached-route fields, then calls
`53EDE0`. Its concrete vtables are solver A3680C, strategy A367B0, and queue
A367F0. The prepass requests cached-route flag0. `region_search` implements this
concrete graph search with independently verified 16-byte queue entries. The
cached-route flag1 tables are not implemented here (phase 8 ports them: aa_navigation::route_cache, research/phase8-movement/native-route-cache-findings.json). Per-node cached distance values
are supported and are distinct from those route tables.

## Recovered graph behavior

`region_search::search` consumes an ordered caller-provided `RegionGraph` and
reproduces `53F320/53EDE0`. `region_route` adapts its chain to the prepass's
`RegionRoute` contract. Node IDs are validated array indexes; native map topology
is not inferred from visual geometry or guessed adjacency.

The original heuristic `53ED30` uses the fixed start/goal pair, even when passed
a candidate. Distance lookup `7BB420` first checks the source node's distance
cache. Its geometric fallback `7B9A60` stores X delta to binary32 and retains Y
delta extended before square root. Queue entries preserve node ID, signed depth,
priority bits and accumulated-cost bits. Costs and priorities have separate
binary32 stores. Equal priorities rise on insertion; equal child priorities
select the right child on removal. Nodes close on first discovery, before the
goal and category tests; there is no decrease-key policy.

The byte traversal epoch wraps. On wrap, native code resets every predecessor
byte to FF while retaining old node epoch bytes. Start marking uses its neighbor
lookup; a missing self-edge produces FE. Busy/readiness failures clear the output
and unlink the strategy without incrementing the epoch. Initial category mismatch
fails after incrementing it. Successful finalization leaves output linked, clears
the queue and resets busy.

The depth255 limit and partial-result behavior are retained. Closest-node
comparison uses an extended heuristic against its stored binary32 value, so it
must not be simplified because the heuristic is fixed. Finalizer `53EBD0` builds
backward from the strategy goal, even in partial-result branches. A missing or
cyclic goal predecessor chain returns an explicit Rust error instead of reading
invalid native memory. Earlier path and state effects remain; automatic retry
would not be safe. The queue limit is an explicit caller option; 0x2000 reproduces
the original configured limit, while None imposes no artificial capacity limit.

`graph_oracle.py` executes the original entry with concrete native solver,
strategy and queue objects. Only allocation/free are hooked. Its 96 executions
(32 cases under PC24/53/64) compare every queue pop, discovery pair, final route,
node epoch/predecessor byte, busy/link state and cleanup result. Coverage includes
cycles, equal priorities, reversed neighbor order, cached/negative distances,
category boundaries, stale epochs, readiness failures, bounded queues, the depth
limit and generated connected graphs. A separate Rust test checks explicit
failure when a bounded queue leaves the goal predecessor unavailable.

## Recovered prepass behavior

The routine resolves start and goal IDs independently, then requests the ordered
region chain. A zero graph result leaves the caller's path unchanged and fails.
Any nonzero graph result continues, including a partial result.

It removes the chain's last region, then its first. Each collision-probe iteration
removes the current first once more before testing the successor. That ordering
is observable: for `[0,1,2,3,...,goal]`, the first candidate is region2. A candidate
is placed through `552250`, tested by `4F3C40`, and only if that returns zero is
the start-to-candidate connectivity test `522DD0` made. Blocked or directly
connected candidates are skipped. The first remaining candidate with a zero
placement result and false connectivity result becomes path.word18; exhausting
the chain writes -1.

For a selected region, the routine prepends the final goal and every remaining
region center in reverse construction order, producing forward kind-2 waypoints.
Every node preserves payload `[x,y,dx,dy,2,0,x,y]`, wraps the native kind-2 counter,
and sets dirty. Existing nodes and other metadata are retained.

Directions use the original asymmetric arithmetic: Y delta is stored to
binary32, X stays extended, and the reciprocal length is stored to binary32
before multiplication. This differs from the tile-materializer normalization.
The final endpoint adjustment uses the selected first center, or the original
goal if no region remains, with original start used as fallback. A false result
retains any nodes already emitted. If a successful adjustment changes the head
position, its direction is recomputed from the start using the different native
store boundaries; words6/7 retain the original position and word18 is resolved
again from the adjusted point.

## Evidence and remaining limits

`prepass_oracle.py` executes original `53DC00`, the real path-node constructors
and list operations, placement field writes, and all original x87 arithmetic.
Fixtures supply region lookup, graph result lists, probe construction, placement
rejection/connectivity, endpoint adjustment, allocation/free, and debug output.
These are explicit production trait boundaries. Existing endpoint and solver
fixtures independently cover the recovered endpoint routine.

168 native executions cover 56 cases under PC24/53/64: chain lengths0..8, graph
failure/partial success, immediate/blocking/connected probes, coincident centers,
endpoint success/failure/changes, and preserved existing tails. Rust compares all
eight words of every output node, counts/metadata, return values, descriptor
position and callback order. The native oracle also checks stack/register ABI
and exactly-once release of every supplied region-list node. Rust owns that
temporary chain in a Vec and does not reproduce allocator addresses/timing.

Graph and prepass evidence are recorded separately by `record_graph_evidence.py`
and `record_evidence.py`. Native topology construction, world-to-region lookup,
cached-route flag1 and binding dynamic collision to live actors remain required
before this supplies actual-map gameplay.
