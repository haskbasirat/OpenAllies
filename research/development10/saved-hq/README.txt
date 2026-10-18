Saved HQ slot30 integration (Development10)

The actual original tutorial02 managed diagnostic record pass now reports no
record errors. The former first stop was slot30 on actor4099, source ordinal434.
That same real HQ is now constructed and loaded in its authoritative Actor
component slot: regiment_usa_hq_infantry_unpacked, owner Kingdom3172,
HQComponent.max_attachments/ActorData+530=5, saved badge3, Nation nation_usa
with28 registered Badge entries, five null slots, no pending items. The exact
25-byte payload begins at DYNAMIC_THINGS offset68482. tutorial02-v3.json and
tutorial02-final.log record this. Publication remains false: this is a managed
record diagnostic, not a completed world Init/PostLoad/battle certificate.

Implementation
- Existing aa-actors::hq_state remains the native574490/574040 algorithm.
  owned::Component::Hq stores that same Box; Definitions.hq_slots_530 is required
  for construction, never an inferred zero. Slots are growable storage.
- Runtime saved_hq binds the existing generic resolved typed integer field
  HQComponent.max_attachments. Native4684E5..46851C finds HQComponent, calls
  5B4ED0 with keyA06988 and stores EAX at ActorData+530. No parser edits.
- Slot30's actual restore branch constructs/inserts once, then calls574040's
  HqState kernel directly on that state. It does not predecode a scratch HQ.
  Table registration and partial writes remain observable on later failure.
  The raw saved payload is retained only after successful Load. A pure wire
  framing decoder exists for inspection; it is not used ahead of live Load.
- The adapter reads current Actor+E8 and generation-checked Kingdom/Nation
  state from the real registry. Existing Updatable table/count/occupancy and
  Nation Badge getters supply actual owners. Nonnegative IDs register exactly
  once in the native Load; SavedComponent::updatable deliberately returns None
  for raw Hq payloads to prevent generic double registration.
- Native Warning/Fatal/41DFB0 errors keep the load-warning exception marker so
  the existing4FAE80 catch path can recognize them. Truncation/unsupported world
  services stay explicit errors; current slot state is not rolled back.
- Hq vtableA3CA4C+20 targets580F70 (returns-1): no required component.
- The three PostLoad-world accessors now use the actual Hq pending vector and
  badge field. Existing integer5736B0/4E2640 algorithms and actual Kingdom
  badge/RNG ownership are reused, with no badge default or new parallel state.

Proof
native_saved_hq.py executes original andv14 4CF480(case30)->574490, no-required
virtual580F70 and Load574040 at PC24/53/64. There are162 cases (27 each),766
executed instruction pins excluding declared leaf entries. The old saved-HQ
26-case producer/machine are hash-pinned dependencies. Stream, allocation/free,
owner display-name and diagnostic leaves remain declared. The27th case uses
the real tutorial02 payload, source constructor slots, reached Nation Badge
count and table bound from the actual diagnostic. Unreached display strings
and occupancy in that isolated case are labeled synthetic, not native capture.
32 separate static original/v14 instructions pin the source store and class
vtable/destructor windows; those are not counted as execution pins.

For successful cases the direct scalar deleting destructor5739E0->573890 also
executes. Its state is sampled BEFORE the final allocator free callback. It
frees pending items/buffers and the slot buffer, then the component; it does not
release the Nation badge. This is direct destructor research, not a completed
PreDestroy574550/external attachment lifecycle port. Native4D8290's
remove-after-Load applies only to Body/Experience(0/17), never HQ; slot30 mask
mismatch throws before construction. The explicit unreachable removed-HQ guard
is not an actual shipped-data dependency.

Tests
- All9 aa-mapworld selected regression tests passed: old170 wire cases,
  old248 dispatch cases/order/partial-prefix probes, new162 HQ factory/framing
  rows and90 native partial-error/registration-prefix rows.
- Existing aa-actors native_saved_hq and native_saved_post_load:4 tests passed.
- Actual tutorial02 source/record test:1 ignored real-data test passed, including
  verified canonical CV source setup, managed fresh task, real resource catalog,
  actual Kingdom/Nation ownership, and zero record errors.
- Final metadata-updated HQ fixture replay:2 tests passed.
No broad workspace or repeated Clippy pass was run for this task.

Limits
Invalid-badge native warning text currently needs the actual GActor display-name,
NationData+10 and diagnostic sink; those callbacks refuse explicitly when
reached. The core HQ kernel already models that path and is separately native
proved. Current tutorial02's badge3 is valid and never reaches these callbacks.
Actual Updatable id==count reads outside its native allocation; production
refuses it. The historical synthetic harness deliberately maps an extra cell;
that row is excluded from current table-adapter prefix acceptance. Allocation
success, no external allocator reentry and valid typed owner graphs remain
explicit domains. No full battle/world lifecycle, HQ attach/detach/recruit UI,
networking or gameplay success is claimed by these record tests.

Ownership/backups
before/ contains actual pre-edit shared source copies. Root authorized the HQ
restore branch and three PostLoad accessors. restore.rs also contains separately
authorized lifecycle/world-version edits by the world-start agent; they are
not claimed as HQ changes. Catalog remains shared: only its exact HQ hook is
owned/pinned here. lib.rs changes are one appended export each. The original
HQ kernel, root manifests, native EXEs, assets, saves and profiles were not
edited. freeze.json records the final owned files, oracle and shared hook.
The historical element checkpoint's slot30 stop is obsolete for this context;
root owns updating it to this observed zero-error boundary while retaining its
separate native membership probes.

Commands (from rust-rebuild, own target only):
 python research/development10/saved-hq/native_saved_hq.py
 cargo test -p aa-mapworld --test saved_components --test restore_components --test saved_hq --release --locked --offline -j4 --target-dir target/s-settlement-completion -- --nocapture
 cargo test -p aa-actors --test native_saved_hq --test native_saved_post_load --release --locked --offline -j4 --target-dir target/s-settlement-completion -- --nocapture
 $env:AA_GAME_ROOT='E:/Games/Axis and Alies'
 cargo test -p aa-runtime --test saved_hq_restore --release --locked --offline -j4 --target-dir target/s-settlement-completion -- --ignored --nocapture
AA_HQ_REPORT optionally writes a NEW diagnostic report; leave unset for a rerun.
