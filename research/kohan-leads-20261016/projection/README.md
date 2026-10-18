# Bounded original-AA projection producer

This is a new native-AA port authorized after the separate Kohan source lead
review. **No Kohan patch or vendor SDK implementation was copied.**

Production changes are limited to `crates/aa-runtime/src/camera_projection.rs`,
`crates/aa-runtime/tests/camera_projection_native.rs`, and one reread-before-append
module export in `aa-runtime/src/lib.rs`. Its prior shared file was backed up
here. No dependency/manifests, renderer/client, original assets, cache, saves,
profiles or releases changed.

## Boundary and inputs

The original instructions execute from **AA `00892B7A` to `00892D5E` exclusive**.
They are inside the candidate six-argument camera-data function at `00892970`.
The fragment stops before the actual device/vtable call submits the projection;
the final argument pushes are incidental ABI setup, not a Rust call contract.
It does not cover the function's device-loss handling, view/basis matrix,
screen viewport, helper implementation, culling, picking or world integration.

`Frustum` supplies six raw binary32 words, in native offset order
left(+0), right(+4), top(+8), bottom(+C), near(+10), far(+14), plus the
orthographic mode(+18). Inputs are stable and read-only throughout the call.
The native arithmetic itself confirms top-minus-bottom at `892BAC`; matching
evaluation header declarations independently clarify the field naming. No
reentrant mutation of the frustum input is claimed.

The caller supplies x87 `Precision` explicitly. `World` must implement:

* `renderer_flag_54`: the actual helper virtual +54 result, at its native point.
  No meaning beyond selecting the observed horizontal branch is guessed.
* `store_word`: each immediate raw store to the native logical renderer offset.
* `load_word`: the actual later matrix +708/+718 reads used for depth caches.

There are no success defaults or detached-world snapshots. A missing service
returns an error while keeping all previous stores. The implementation neither
allocates a D3D object nor supplies a live-world adapter.

## Native arithmetic and output convention

Original-AA data at `009F32E0` is 1.0 and at `00A0083C` is -2.0. They are read
and pinned independently of the Kohan project. No cross-game address, FOV,
baseline resolution or zoom multiplier is used by the producer.

Native `FST` writes the depth span to +350 but leaves the unrounded x87 span on
the numeric stack. Width, horizontal sum, height, vertical sum and the reciprocal
depth-span locals each pass through their actual binary32 stores/reloads.
The Rust code preserves these distinctions using `aa_numeric::Binary`, with
the requested precision at each operation; it uses no host float arithmetic.

The matrix is in the original renderer's row order at +6E0..+71C. Its
perspective branch has M34=1 and M44=0, and depth terms corresponding to
far/(far-near) and -far*near/(far-near). The orthographic branch has M34=0,
M44=1, depth 1/(far-near), and -near/(far-near). Near/far map to the D3D
zero-to-one depth interval under these conventions. The code does not replace
the inspection camera's wgpu/right-handed projection with this matrix.
Arithmetic order and intermediate rounding, not algebraic simplification of
these explanatory formulas, are authoritative.

The +708 depth-scale word is reread before the depth-translation store; +718 is
reread after that store. Captured values are then copied to +3FC/+40C in native
order, with the intervening matrix-zero stores intact. No whole-result commit
or rollback is introduced.

## Proof and explicit seams

`native_projection.py` executes the unchanged fragment in **both pinned PE
images separately**, at PC24, PC53 and PC64. Executed in-image instructions
are byte-compared with accepted v14 and recorded. Page-zero accesses fault
except permitted FS operations; no allocator is reached. The frustum remains
byte-identical and complete paths leave the FPU stack balanced.

The 1,536 rows are 256 inputs x two executables x three PCs:

* 40 bound tuples x both projection modes x both flag results = 160 inputs;
* 92 pre-instruction boundary stops, covering every store/read/service of all
  four mode/flag paths, with retained native state and full event order;
* four depth-cache mutation probes, distinguishing the native reread/capture
  positions from stale copied results.

The flag helper is an explicit callback seam; neither its native body nor D3D
executes. Ordinary field-boundary failures/mutations are laboratory observers,
not a claim that native memory reads/stores are virtual calls. Failure injection
happens **before store instructions**, avoiding Unicorn's pending-memory-write
commit behavior. The tests compare final words, complete event order and every
partial-error prefix. All 768 original/v14 row pairs match.

The focused Rust test replays all rows. A second portable test verifies lazy
finite-domain rejection preserves the preceding raw near-word store and that
zero-width rejection occurs after the actual flag boundary. These are explicit
Rust domain errors, not claims about native infinity or exception behavior.

## Checks and reproduction

From `rust-rebuild`, PowerShell:

```
python research/kohan-leads-20261016/projection/native_projection.py
$env:CARGO_TARGET_DIR='target/s-graphics-review'
cargo test -p aa-runtime --test camera_projection_native --release --locked --offline -j4
```

Final replay: 2 tests passed; see `tests-final.log`. The first accepted replay
is also retained in `tests.log`. Generator output is pinned in
`native-generation-final.log`. The independent reviewer owns separate evidence
and does not regenerate or overwrite the author fixture.

Scoped `cargo clippy -p aa-runtime --test camera_projection_native --release
--locked --offline -j4 --no-deps` completed successfully. Its three existing
library warnings concern other modules; no new projection warning was emitted.
This is not a claim of a warning-free strict whole-crate Clippy run. No lint
allowance or unrelated source fix was added.

## Limits

Only finite supported `aa_numeric` arithmetic is certified: nonzero divisors,
supported extended exponents and representable binary32 temporaries/results.
Overflowing binary32 local reloads, NaN/infinity, exception flags and other
rounding modes are not covered. Raw stores/copies that do not perform numeric
operations remain raw; validation is lazy rather than an eager whole-input scan.

This does not establish AA's complete FOV/aspect producer, camera defaults,
pointer-mutation reentry, actual helper +54, device behavior, visual parity,
DPI correctness, full SetCameraData, game startup or gameplay compatibility.
No rendering integration, widescreen patch or existing user setting changed.
