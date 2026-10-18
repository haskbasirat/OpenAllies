# Organization view constructor tail

The new client `view_organization_fields::Fields` owns the native `44FDB0`
tail after the separately retained `453F10` Actor owner. It reads no source
data or global fields. `Fields::construct()` returns that owner, or retains
the partial owner in `Failure` after a portable allocation refusal.

The native constructor sets the array pointer/count to zero and capacity to
14, allocates 56 bytes, then stores the returned pointer. It writes null
links at `164` and `168`, byte zero at `16C`, and word zero at `170`.
`16D..16F` and the unused fourteen pointer slots remain unwritten. Rust's
growable vector is the sole item/count owner; initial capacity 14 is an
allocation policy, not an item limit. Numeric native pointer values remain
opaque. A future positive list writer must retain its actual resource owner.

The native fixture executes the actual `44FDB0` body against both original
and v14 binaries at all three precision modes. Four distinct object/heap
poison combinations and a cutoff at allocator entry produce 24 paired rows,
48 executions and 37 decoded equal instruction pins. The only seams are the
separately owned Actor constructor and allocator transport. Poisoned newly
allocated slots are checked byte for byte and receive no constructor writes.
The cutoff observes the preceding array stores; it does not claim native
exception, OOM or unwinding behavior. The Rust test uses that prefix to check
the failure owner, and otherwise calls the real public constructor.

Two exact-source isolated Rust tests pass. No large client unit link was
started. The SDK agent owns actual Directory integration and complete base
construction. This receipt does not certify derived Init, destruction,
native heap ABI, publication, GPU installation or view/application readiness.

```powershell
$env:PYTHONPATH='reconstruction/tools/python-packages'
python research/development10/view-organization-fields/native_organization.py
python research/development10/view-organization-fields/audit_organization.py
& research/development10/view-organization-fields/isolated-organization.exe
```
