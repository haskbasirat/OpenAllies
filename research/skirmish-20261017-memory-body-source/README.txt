Staged detection-memory Body and Supply stats (author work until freeze.json)

The former4CEC90 diagnostic incorrectly called this a modified maximum. Actual
instructions always pushNULL before529C30->45D730; this reads ActorData+284
base health, independently of current Body.health or Body.maximum. Positive
Properties/5014F0 is not reached. Rust now validates the SAME owned Body/header
and reads actual typed Fixed BodyComponent.health using the SAME moved Bank's
source precision. It does not substitute the live maximum or an effect factor.

A second genuine guard correction: native4CECB0/4CECE0 tests Actor+8C, Supply
slot11. Denizen is slot10 at+88 and is not consulted here. Supply absence or
Data+424 is_relay=false returns the actual zero leaf. For a relay, virtual118
reads the sole Supply.stockpile(+18), and virtual11C reads Data+428 through
58AA10. Source stockpile_max is read lazily only on that last relay path.

native_memory_body_source.py default immutable:54 rows/108 original-v14/all3PC
machines,245 address-byte pins, whole4EC4A0 Body stats and partialstat3/stat5
prefixes. Source/current/maximum deliberately differ; Properties points to an
unmapped poison address. Static mapping pins identify464BD0's health getter+
fstp Data+284. Typed getter/source compilation is separate from these probes.

native_memory_supply_source.py default immutable:72 rows/144 original-v14/all3PC
machines,248 address-byte pins. Positive/absent Supply and relay false/true,
current stockpile vs source max, partialstat4/stat5 stores. False relay carries
NaN poison stockpile/max fields; Denizen pointer is unmapped poison throughout.
These fields remain unread on the native guard paths. Heap/CString transports
are inherited; no native Win32 allocation-failure/unwind equivalence is claimed.

Rust fixture tests reuse the actual refresh coordinator and explicit stat
callbacks for pure ordering. The actual saved-map acceptance separately binds
real ActorStore/Body/Supply/source storage; whole completed mission/Fog/battle
claims are prohibited until their actual paths finish.

FROZEN: coherent final Rust3/3 passes old252+Body54+Supply72rows. Actual capture1/1 PASS44.16s; immutable repeat1/1 PASS63.07s. Genuine113-object stopped snapshot explores ALL16384tiles with Ok, sameRuntimeObject/memory allocations and actual movedBank/CatalogBankempty. Both Farm memory records publish sourcehealth500 as stat114. Positive Supply callback in an actual saved owner is not separately claimed in this prefix; native guard/typed storage evidence is distinct. This does NOT certify a whole mission/PostLoad/battle.

Exact replay: set AA_GAME_ROOT, unset AA_CAPTURE_STORE_FOG; frozen-memory-source-tests.exe load_store_memory_refresh::tests --nocapture; same exe load_store_fog::tests::actual_stopped_load_moves_same_actor_fields_through_memory_refresh --ignored --nocapture. Native producers default immutable, PYTHONPATH=<game>/reconstruction/tools/python-packages. Shipping profile unchanged; testLTO=false/codegen16.
