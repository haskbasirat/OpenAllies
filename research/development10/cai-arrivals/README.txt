CAI arrival list and concrete Sleep boundary

The actual installed Regiment and Arena identify one mutable Mind. Saved
Regiment Load already moved +2BC/+2C0 payloads into Owned.tracked_words. This
successor replaces that one Vec with cai_arrivals::List (a growable typed
LinkedList), moving saved payloads in original order. Fresh arrivals call
Builder's guarded take_constructed_owned only once, removing the old scalar
head/tail owners before retaining that same Owned in ActorStore.regiment_owned.
Neither Installed nor Arena receives a shadow list. Raw duplicate/zero GTIDs
are retained; generation resolution belongs to the consumer.

stored_cai_arrivals::push_front is now called by the real524B80 runtime caller.
The native4D0900 allocation-success path and524B80's whole current tail are
paired across original/v14/all three PCs. Tests include helper mutations of
list/mode/memberGTID and required-service cutoffs before allocation/publication.
Native null allocation faults are outside Rust allocator-success parity.

524F53..524FE1 resolves the current head GTID before unlink/free, then checks
membership and invokes522890. One shared cai_arrivals::drain implements this
order. The concrete Sleep overrides peek/remove on the SAME runtime owner;
legacy laboratory Sleep adapters retain their prior early-pop omission through
explicit compatibility defaults. Their existing400-case regression still passes.

stored_regiment_sleep calls the unchanged main524D70 algorithm. Arena alone
owns status/stack; Installed owns retry/flag/point payloads. It uses real live
queues/52BF30 and same522890 command construction/delivery, plus shared exact
522F60/523000 field kernels. Native source-derived member-vf78 kinds and lazy
Element-status reads are pinned. Source preflight is for valid managed graphs,
not native malformed-pointer/error fault equivalence.

think_on_owners is a concrete entry for the actual scheduler. The whole natural
scheduled Think has NOT executed in this author scope. Positive follow/home,
exhaustion/SAI, and57BF30 relocation children remain required; this checkpoint
does not mint activation or certify first gameplay tick/unfreeze. New bounded
field tests do not certify all source/owner callback combinations in a live map.

New evidence counts exclude existing constructor/load/fullSleep dependencies.
No fixed memory/VRAM/bandwidth/list cap or native ABI pointer is introduced.
