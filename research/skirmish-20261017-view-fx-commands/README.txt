The new runtime owner retains actual compiled FX commands in the same VWorld.

43B870 now has a typed E0/E4/E8 list; Command+68 has one typed next owner and
no independent mutable raw word. The command moves out of the manager's
Option only after reached429020 succeeds. Fresh commands are proved to have
+60=null by the existing native428480/embedded-FXL constructor receipt and the
new actual-source test. A positive +60 still requires its actual virtualBC and
45A940 retention owner. Delivery appends active commands; it does not execute
later FX update/render, instantiate VFX children or certify GPU/view readiness.
43B8E0/428850 instead remap active command source/target GTIDs on Thing replacement;
that separate native operation is not implemented by append.

The private list uses growable host storage and owner-bound opaque keys. Its
native u32 count wraps, while actual owned storage is usize and has no fixed
pool cap. Host allocation preflight and invalid-owner checks are portable
binding behavior, not native allocator, invalid-pointer or unwind parity.

The same view_init_world::State constructor owns the list. Mutable access
requires that state's existing actual World witness; no numeric revision or
AD-ready setter is introduced. The client owner validates its actual ViewToken
and directory before borrowing this same list. Initial50CAB0 attachment keeps
its exact detached Option on failure. Direct attached50C800 Effects submissions
by value remain a separate delivery-owner integration requirement.

Native evidence:90 append rows and3 constructor store-suffix rows,186 separate
original/v14 executions across24/53/64-bit x87 precision,46 executed instruction
pins. Prior lists/counts/FXL virtual pointers are explicit fixture inputs. The
constructor receipt executes442CF0..442D0E stores with an explicit EBX0 input;
earlier442C37 XOR producer bytes are separately pinned, not claimed executed.
Both append count overflow and required/null prefixes are covered. Positive
FXL true/false branches run natively in the fixture; runtime acceptance currently
binds the real constructor-null branch only.

Exact final pure production source tests:2/2 PASS, all native output/link/count
rows plus foreign/replaced key-owner refusal. No source copy or alternate list
algorithm was compiled. The first attempted direct rustc link selected an older
thin-LTO rlib and failed; v2 uses the pinned non-LTO serde_json dependency and
passes. Full current runtime command test is pending and must be reported from
its actual result, not inferred from pure tests or native row counts.

Actual-source test scope:compile canonical barbed_wire Created graph, construct
three true Commands with explicit direct-lab thread/time inputs, queue them in
the native manager order, then move them once into the same constructor-owned
VWorld list. A final null command must retain the manager's detached-null error
prefix. World identity tests deliberately stop the first prefix child; no
initialized VWorld, mission startup or playable battle is claimed by that lab.
