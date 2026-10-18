Post-adoption Object detection memory (2026-10-17)

Scope
-----
The private load_store_fog::refresh_memory_published entry reuses the existing
4EBC70 coordinator and same RuntimeObject.object.memory allocation after native
placement ownership moves into SpatialWorld. The caller supplies this map's
paired ActorStore, SpatialWorld, moved Bank, Kingdom registry and Session-backed
load_zone_services Context. This is not a readiness/admission certificate.

MemoryOwner only borrows those owners. PlacementRead::Published synchronously
checks current ObjectKey/ActorKey/GTID mappings, then routes every reached
position/flag getter through ActorStore::placement and the current SpatialWorld
member. It retains no placement words or second mutable flags/pose allocation.
Staged global Fog keeps its original staged placement path.
The new published wrapper first validates its actual source/layout/current
published allocation even when native eligibility or an empty vector would
return early. This portable binding guard is not native invalid-pointer failure
equivalence; it precedes the unchanged coordinator on valid bindings.

4EC4A0 record construction shares one ordered kernel. Its timestamp/name stores
still precede position, followed by definition, owning Kingdom, aspect, the
native statistic order10C/110/114/118/11C, byte30 and full flags. The same memory
record/slots and every successful prefix survive a later service refusal.
Source authority remains the actual moved Bank source PC; placement arithmetic
PC is read from the same spatial index. No arithmetic PC is substituted for
source completion. Unsupported numbered names/stance/container/observer effects
retain their existing required-service boundaries.

Native evidence
---------------
native_memory_count.py/json executes actual Object virtual8C=4EA3D0 in separate
original/v14 images at Single24/Double53/Extended64:18 rows/36 runs,5 executed
instructions. The result is signed Object+58 >0, not pointer presence. +5C is
poisoned and never read. A negative count is a getter probe, not admitted as a
valid initialized allocation. Normal invocation immutably replays the fixture.

Existing fixtures are reused, not rewritten:
* load-detection-memory/native_memory.py:249 rows/498 runs,389 executed pins;
  refresh virtual answers are explicit probes; record/slot construction and
  concrete delete/release children have their separately executed rows.
* store-memory-refresh/native_memory_refresh.py:252 rows/504 runs,311 pins;
  the record getter/callback and partial-store prefixes remain explicitly
  bounded. This is not one unintercepted full native mission refresh chain.
* memory-body-source/native_memory_body_source.py:54 rows/108 runs,245 pins.
* memory-body-source/native_memory_supply_source.py:72 rows/144 runs,248 pins.
* load-store-fog/native_store_fog.py:existing1728 global-Fog coordinator rows;
  mutable node callbacks/field prefixes remain explicitly test seams.

Storage acceptance
------------------
load_store_published_memory_tests.rs uses a real stopped Tutorial02 immediate
load prefix and same completed source Bank, converts that prefix into ActorStore
and adopts the exact existing load-time index. This diagnostic adoption is not
a naturally completed PostLoad/mission. On actual tree GTID28, a clearly labeled
controlled explored/visible word exercises record creation and deletion over
the actual Kingdom entries. The test checks the original memory allocation,
current published pose/flags, no duplicate records on repeated refresh, a native
retained-bit2 write to the same flag word, and a wrong-world failure after the
timestamp store. It also rejects published access before actual adoption.
No original map/save/profile is changed. No native allocation failure/SEH/
unwind or arbitrary invalid-world equivalence is claimed.

Current validation is recorded in findings.json/freeze.json after the scoped
Rust replay completes. Passing a bounded memory test does not establish full
actor initialization, mission activation, combat, campaign or multiplayer.

FINAL ROOT SHARED-COHORT ADDENDUM
Root runtime43 exact current test passed1/1 in23.52sec. Log copied to
root43-final-replay.log. Production/test hashes unchanged since the independent
source-only review. Same actual actor28 created2 records, retained reached
error prefix, reread published fields and deleted the same records. This
closes the pending current Rust replay; the reviewer still ran0 executions.
No whole map/PostLoad/mission/battle readiness is implied.
