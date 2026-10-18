# Concrete NiCamera base fields and list ownership

`native_camera::base::initialize_874b40` initializes the same camera State that
the7016A0 author retains. It performs86A680/86B240/86F710 field producers,
8751C0 local/world transforms,88A5A0 frustum byte, two owned null-reference
arrays, six869C30 planes, then the existing concrete8742D0 numeric update.
The final position copies,200=1 and1FC=0x3F follow that actual update.

All27 current B43CF8/B43CFC/B43D00 reads occur at their reached points; source
failures retain earlier field writes and the same allocated lists. The real
application caller supplies its retained checked startup Globals binder.
Laboratory source vectors do not establish an observed current process context.

Each array owns its slots/capacity/used/effective/growth fields once. Numeric
State getters project170..17C and188..194 from those same owners; writes to the
count fields route there. No mutable scalar copy of those counts is kept.
16C/184 remain unavailable as numeric pointers. Initial capacity5/growth5 and
five NULL slots are actual constructor values, not memory limits. Positive
external reference insertion, resizing, parent/attached mutation and native
destruction require their real future owner services.

Native vtable addresses, allocator ABI and the process Ni-instance counter
B45F14 are not implemented as portable identities. Native executes that counter
increment in the oracle; this report does not claim Rust/global Ni accounting
parity. Rust allocations and reference ownership deliberately replace that ABI,
as in the scene owner. Native constructor scalar fields such as initial refcount4
are bounded constructor values; later parent/reference mutations require their
real writer. There is no completed Ni ABI or whole7016A0/directory receipt.

`native-base.json` contains99 paired rows/198 original-v14 executions at all
three PCs,822 individually decoded equal instruction pins, three allocation
fills, zero and nonzero current source vectors, and all27 source-read partial
failure prefixes. All constructor/transform/plane/viewprojection bodies run;
6F7810 allocation is the only transported leaf. Required-read instruction PCs
are conservatively excluded from execution pins. Known byte masks distinguish
unwritten fields/padding from zero heap inputs;108 remains unbound. Allocation
success and finite valid source values define this test domain; no allocation
failure/panic equivalence is claimed.

Two base-specific tests replay every99 row's value, byte mask, source order,
array ownership and retained failure. The exact-parent reset/pose/Ni/target/base
suite passes10/10 in isolated-tests-final.log. The isolated source only relocates
actual child paths, omits unrelated backend/render-list modules, and supplies a
Globals trait with the exact production signature. `make_isolated.py` checks
that transformation. An initial harness inner-doc placement error is retained
in isolated-build-01.log; it was corrected in the harness only.

Real runtime production typecheck passes in15.88s, with existing warnings
retained. The current parent is shared with the7016A0 author; later constructor
and scene/directory ownership changes need their own acceptance. This field
producer cannot writeAD, certify renderer upload or unfreeze a simulation.

```powershell
$env:PYTHONPATH='E:/Games/Axis and Alies/reconstruction/tools/python-packages'
python research/development10/native-camera-base/native_base.py
python research/development10/native-camera-base/audit_base.py
research/development10/native-camera-base/isolated-base-final.exe --nocapture
```
