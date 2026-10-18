# Native order registry correction

The previous 357-entry completeness claim was wrong. Its discovery used only
the Ghidra `calls.tsv` export, then skipped every other actual CRT initializer.
That export omits `9E65F0`, which is unconditionally present at CRT slot
`B26644` and calls the real registration constructor `766E40`.

`native_registry_audit.py` independently decodes all 3,634 actual nonzero CRT
initializer entry points through their direct control-flow branches and tail
jumps. It then executes the actual CRT pointer loop plus every discovered
direct order initializer, registration constructor and append operation in
both original and v14 images. Both register the same 358 descriptors.
Non-order callbacks remain explicit seams; this is not whole CRT execution
and does not claim discovery of arbitrary indirect registration.

The missing descriptor is `OrderVersionAckRequest`: initializer `9E65F0`,
descriptor `B40AF0`, handler `766CF0`, actual ordinal **323**. Previous ordinals
0 through322 remain unchanged; previous323 through356 become324 through357.
The descriptor width remains9 bits. Descriptor identity uses the actual
descriptor allocation/handler, since names are not unique.

The complete prior producer, evidence and Rust source snapshot is preserved
under `historical-357/` with a byte manifest. Earlier proof/findings/review
reports remain historical and do not certify the corrected inventory. No
native executable, user save, profile or film was changed. Portable packets
using the old shifted IDs were not native-compatible for those descriptors.

`research/phase4-orders/crt_order_initializers.py` now provides the corrected
discovery to the shared recovery harness. `recover_orders.py` executes both
images' real registration bodies, and `gen_registry.py` generates from those
executed registrations. Payload and template-flow oracles are replayed with
that actual complete native registry installed before every handler run.

The new323 handler is a direct thunk into Admin `751FC0`, not a
`TGC_RequestDecreeOrder` template. Its wire payload is empty. The generic
template `flow::shape(323)` deliberately returns an unsupported error; no
neighboring request/decree behavior is assigned to it. All357 real template
Shapes retain their behavior under the corrected ordinals.

`native_version_ack_header.py` separately runs full `766D40` after the real
CRT setup in six fresh original/v14 and24/53/64-PC machines,324 cases over
byte/bit offsets and Admin-active values. It uses the actual registered
count/ID without injecting either value. The two mutex virtuals are explicit
OS callbacks. Actual `74B0D0`, `4B2830` RET, `40C7C0` and `40D420` run. The
emitter always appends the one9-bit header: no active gate, SetPlayer prefix,
payload or sync log. The live adapter borrows the same retained NetNode mutex
and Admin request stream; it does not construct another request queue.

Commands (read-only native replays unless the explicit fixture authoring flag
is supplied):

```
python research/skirmish-20261017-order-registry-correction/native_registry_audit.py
python research/skirmish-20261017-order-registry-correction/native_version_ack_header.py
cargo test -p aa-orders --release --locked --offline -j4
```

The refreshed phase8 trace/flow scripts execute the original PE with declared
synthetic manager/process seams, after actual corrected CRT registration.
They are not whole native game or network runs. Native registry and direct
acknowledgement-header evidence separately execute both PEs as described.
