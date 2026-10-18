# Immediate lifecycle of each loaded record

`aa-mapworld::record_lifecycle::finish` is the original control flow at
`4FB4B7..4FB539`, including failed Init's warning/destruction path
`4FB570..4FB608`. Root integration owns its required callbacks and invokes it
on each actual held record before loading the next record. This module adds no
replacement gameplay, successful default callback or readiness certificate.

The authoritative out slot is reread at the original boundaries:

1. Null initial out returns. Otherwise read the actual current flags.
2. With bit7 clear, call virtual+E8(NULL) and test its AL byte. If false,
   reread the current out for warning XY; issue the required map warning;
   reread it again for4F5490 destruction; clear the out slot only after that
   service returns. A missing warning/destructor leaves preceding effects.
3. With bit7 set, clear bit31 in that actual record's flags and require
  542A00(&out) on world+110.
4. Reread out; if nonnull, reread bit7. With it clear, require virtual+EC(NULL,1),
   reread out and require4F9510 on that value. There is no native null gate
   between PostCreate and4F9510: the portable coordinator explicitly refuses
   null here instead of silently skipping registration.
5. Reread out. If nonnull, call virtual+B0 and test AL. If true, reread out
   again and require4296B0(record,1) on the current definition+BC. A null value
   at this unguarded native dereference is an explicit portable failure.

The callback owns the actual Init/PostCreate/state/grid/destruction/warning/
definition behavior; these services are not implemented by this coordinator.
`live_registration::State` provides the actual later4F9510 registration storage
when its own mandatory services are available. Object Init's separate world+8
list stays in `world_object_list`. Diagnostic restoration must keep omitted
immediate lifecycle work explicit; a clean decoded record is not ready.

`native_lifecycle.py` executes the unchanged original and v14 programs. It
starts at4FAE80 and executes the actual prologue, saved nonvolatile registers,
SEH frame and local allocation. At4FAEB3 it skips the explicitly out-of-scope
allocation/Load prefix, supplies its inherited EBP locals, EDI out slot and EBX
resource, restores the exact base ESP (`EBP-124`), then enters4FB4B7. Completed
paths execute the actual common native epilogue and ret16. They must restore
nonvolatile registers, return ESP=caller+20 and retain the requested FPU control
and empty stack. This is not an invented cdecl call to a function tail.

The failed-Init path executes native binary32 XY loads, widening to binary64,
warning argument construction and temporary reference decrements. String
construction/copy/display lookup and the map-warning leaf are explicit service
seams. Finite fixture XY values are used; no native CRT warning text/locale or
special-value conversion parity is claimed.

`native-lifecycle.json` contains636 rows (106 inputs x2 executables x3 x87 PCs)
and132 executed instruction pins. Inputs cover null/current out, bit7/bit31
combinations, zero and nonzero AL bytes, Object false/true, callback-controlled
out replacement/null and flag changes, and a stop before every required
boundary of normal/editor/failed-Init paths. These controlled seams prove the
coordinator's read and commit order; they do not establish arbitrary reentrant
semantics inside the omitted service bodies.

The Rust test replays full event order, selected record identity, flags and out
slot after success or each failure prefix. A native null dereference from a
callback removing a required current record corresponds to the explicit
portable required-null error. Allocator failure/native SEH unwinding and the
earlier4FAE80 allocation/conversion/Load paths are not claimed.

```powershell
python research/skirmish-20261016-live-registration/native_lifecycle.py
$env:CARGO_TARGET_DIR='target/s-graphics-review'
cargo test -p aa-mapworld --test record_lifecycle_native --release --locked --offline -j4
```

For read-only independent replay, import the producer, set `h.P` to the
fixture's original/v14 image, run `Machine(input, precision).run()`, and compare
to its stored output. Do not call `main()` or replace the fixture. The parent
loader's eventual concrete callbacks and inter-record integration need separate
tests; this fixture does not claim a playable battle or full world startup.
