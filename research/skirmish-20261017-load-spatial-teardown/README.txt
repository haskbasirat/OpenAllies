LoadSpatial teardown before global identity retirement (2026-10-17)

Scope
-----
LoadSpatial::detach_for_destruction is the callback-free spatial prefix of
4EE560(retain=false). The caller keeps the actual shell inside its current
registered record. This helper does not drop that record, uninitialize its
components or execute4EFEC0 global GTID retirement. The caller does those later
in native order, with the same flags/source/queue owners retained.

The existing immutable Binding now also records its actual definition-load PC
at prepare time. Teardown separately validates the actual same Bank source PC,
canonical source/constructor/proof, typed ActorDefinition token, current index
key/reference/generation, and explicit live PC matching the existing index.
Source completion is never inferred from the live arithmetic PC.

Only actual current fields are leased. Known native getters form4CF210 blocking
then4EA0F0 soft descriptors;553830 executes the existing native removal kernel
only if flag26 is present. The actual shell receives the current flags on both
success and Result error. The auxiliary index reference and immutable Binding
are removed only after membership removal and unregister both succeed. The
same tiles/nodes/cache/index remain owned throughout; nothing is rebuilt.

The current-only signature cannot provide other records to a primary/blocker
rescan. The actual first Kingdom has no Wall and cannot mark a blocker, so those
reads are unreachable there. Any reached unavailable other-member read retains
an explicit error after its native prefix; no fake empty scan is returned. A
general teardown requiring such candidates still needs their actual owner loan.

Proof and validation
--------------------
native_teardown.py/json executes original and v14 separately at all3PC:
18 rows/36 runs,291 address-byte pins =289 executed +2 intercepted entries
(node free6F7430 and required GTID-retire boundary4EFEC0). Actual first Kingdom
fields and labelled ordinary class2/3 vtable probes cover membership present
and absent. Present cases execute4CF210,4EA0F0,553830 and node removal; absent
cases skip descriptor construction and membership writes. Each machine stops
BEFORE4EFEC0, with the actual native global identity still present. This is not
fake successful identity retirement. Normal producer invocation is immutable.

New Rust tests replay all18 kernel cases and use actual source/capture03 to
detach registered Kingdom shells at all3PC. They check wrong live/source-PC
refusal without mutations, same shell/component allocations, retained global
identity, native matching flags/tiles/events, released auxiliary binding, and
query-only unindexed detach. Full Rust execution is pending the root's shared
coherent runtime test cohort; the own duplicate build was deliberately stopped
to avoid parallel full-runtime links. Production check passed50.08s.

Original files/profiles remain untouched. No full ActorUninit/destructor,
nonconstant derived shape service, all-object teardown, allocation/SEH/unwind,
mission activation or playable-battle claim is made by this checkpoint.
