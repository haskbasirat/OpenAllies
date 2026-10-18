Object aspect scalar source projection (work in progress until freeze.json)

Native49C760 initializes fresh ObjectData+110 count0 and +118 default -1.
In49DC40, the nonvariation branch appends AspectNormal through real49C670
and49C360, then stores count-1 into+118. Later Aspect appends do not change
that default word. Variations retain the constructor's -1 word.

native_object_aspect_source.py defaults to immutable replay.36 rows/72
original-v14 executions atPC24/53/64 cover normal/list order, varying weights,
missing normal, duplicate IDs, and stopped resource-loader prefixes.426 actual
executed instruction addresses plus11 named intercepted getter/loader/CRT/heap
entries are pinned.46A8A0 is an explicit resource loader seam supplied with
labelled typed fields; this does NOT certify original model/resource loading.
49C670 constructor and49C360 append execute natively. Constructor proof is an
exact aspect-fields slice, not a claim of complete49C760 construction.

native_default_memory.py separately executes real4EC4A0->4EAAA0->49B480 for
class9/saved variant -1 and explicit default words0,7,-1.9 rows/18 machines at
all3PC;170 executed instruction addresses. Getter consumption does not load
source or assets. Successful CString transport uses the inherited fixture
allocator; no real Win32 allocation-failure/unwind equivalence is claimed.

Runtime DefaultIndex::bind borrows SAME moved Bank and validates canonical
actual source/proof/category/flags/sourcePC through Source::bind_in_bank.
It reuses existing native_creation_definition's ordered source metadata and
projects only scalar count/IDS/weights/default; no native pointer, asset loader
readiness, model count or construction list is manufactured. Missing/ambiguous
source stays an error. No aa-rules source/parser changes are part of this work.

Pure replay and actual stopped-prefix acceptance are pending until freeze.

FROZEN: pure scalar1/1 PASS; actual capture1/1 PASS34.14s; immutable1/1 PASS32.87s. Same113 allocations, Catalog Bank empty, actual moved Bank borrowed, exploration2147->8403tiles. Next required4CEC90/529C30/45D730 source health on actualfarm2696. See freeze.json and findings.json for exact pins. Test binary: frozen-aspect-tests.exe. Capture environments unset for immutable replay. No completed mission/battle claim.
