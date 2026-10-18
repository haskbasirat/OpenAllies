# Bounded point2 decimal-prefix successor

The original `usa_hq_infantry.tgi` contains `creation_offset = 2.5, 1.5.0`.
The actual template declares a point2. Native classifier 5BD6E0 retains
numeric classification after an additional dot. Point assignment 5BB7F0
splits components and calls CRT `__wtof` 96D3E8 for each, then stores binary32.
The native result is exactly `(2.5, 1.5)`. GetPoint2 5B4D80 reads that actual
point2 allocation through vtable A48610 +40, target 5B6970.

The new Rust helper first uses the established strict scalar converter.
Only point2 can additionally accept outer whitespace, one optional leading
sign, decimal digits and dots when a second dot follows a prefix containing
at least one digit. Everything after the second dot must also be dots/digits.
That prefix then goes through the same finite decimal conversion and f32
store. Scalar fields and point3 keep their existing strict path. Arbitrary
suffixes, expressions, references and nonfinite values do not become defaults.

`native_point2_decimal_prefix.py` runs the whole original/v14 TGI parser and
actual point getter for 63 rows at all three explicit PCs, with 126 native
executions and 5,262 executed address-byte pins. Inherited explicit transports
cover line source, CString pool/heap, C-locale TLS, mutex and empty registry.
The unrelated inherited GTID-registry initializer 4EFF60 is skipped because
the parser does not use it; no registry readiness is claimed. Hooked entries
are excluded from executed pins. Requested FPCW is installed explicitly.

The fixture also records native behavior outside the new bounded subset:
`..5` becomes native zero, and repeated-dot f/% suffixes can succeed. The
Rust adapter deliberately continues to reject these unneeded extensions.
The test distinguishes this declared unsupported subset from parity rows.

Reproduce the immutable native fixture with `python native_point2_decimal_prefix.py`
from this directory or its repository-relative path. The package test is
`cargo test -p aa-rules --test native_point2_decimal_prefix --release --locked
--offline -- --include-ignored` with `AA_GAME_ROOT` set. Required full aa-rules
and aa-metagame suites run with `AA_METAGAME_EXPORT=0`; their exact results,
source snapshots and hashes are in `point2-findings.json` after sealing.

The successor actual-source census reads all 1,028 canonical ActorData at
three explicit lab PCs: 40 DeploymentInfo present, 988 absent, zero typed
errors. This is source-level evidence, not proof of an initialized battle or
successful positive native deployment children. The earlier deployment
findings and census are retained unchanged as historical evidence.
