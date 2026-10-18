# Texture-effect mode 2 and transformed plane

`view_environment_projection::update_mode2_86ba10` borrows the same effect's
numeric fields through a crate-only field loan. It retains no texture, copied
effect, source allocation, view flag or completion capability. The concrete
environment114 owner must supply those same fields. The scene118 effect is
a distinct allocation.

The mode2 branch creates the actual half-scaled rotation and translation, then
always executes86BBB7..86BC25. That tail calls the real870720 matrix/vector
helper, spills each transformed normal component to binary32, and retains the
extended distance arithmetic through its final store. It preserves all current
read and write order, including normal X/Y before the D0 counter read, wrapping
counter store before distance188, then normal Z184 last. Signed zeros and the
asymmetric matrix/vector summation order are covered.

The helper refuses other projection modes at the actual mode read; it does not
implement their native branches. Finite successful arithmetic, typed nonalias
fields, masked round-nearest x87 and explicit precision define the supported
domain. Native unmasked traps, infinity/NaN arithmetic, invalid pointers and
Rust allocation/panic behavior are not claimed equivalent.

The native fixture has153 paired rows/306 original-v14 executions at all three
precision controls. The real86BA10 and870720 bodies execute without a child or
allocation seam. Twenty successful numeric cases include signed zeros,
subnormals, cancellation, varying transforms/planes and counter wrap. Every31
reached current field read of a nontrivial case is also interrupted before
consumption. The two Rust tests replay all field reads, field writes, retained
numeric values and required-error prefixes. The94 equal native instruction
pins are individually decoded; each also executes in successful rows.

The broader update86E8D0 is deliberately not certified here. Its actual path
is vf48=86EED0, optional866BE0 controllers/property list traversal, then
vf68=86BC30→8A2C70→86BA10, vf64=4B2830, and a new parent24 read followed by
positive parent vf7C. Current parent/controller/property/bound owners must be
bound or their reached services remain required. In particular, passing an
empty callback is not an environment-update or VWorld-completion proof.

The first isolated build omitted LTO when linking existing bitcode libraries;
that link failure is retained in build-01.log. build-02 uses thin LTO and the
exact production source, and tests-02.log passes2/2. No production correction
was needed after fixture generation.

```powershell
python research/development10/view-environment-projection/native_projection.py
python research/development10/view-environment-projection/audit_projection.py
research/development10/view-environment-projection/isolated-projection.exe --nocapture
```
