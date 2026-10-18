# Public NIF graphics references and metadata retention

The public Niflib source directly describes fields present in AA's NIF 10.1
models. NifSkope provides useful renderer comparisons, but its preview choices
are not original AA rendering behavior. The concrete change from this review
preserves authored base-texture clamp, filter and UV-set words through ordinary
conversion and Rust asset preparation. It changes no sampler or shader behavior.

## References and provenance

Selected source files and complete notices are isolated under
`E:\Games\Axis and Alies\reconstruction\references\public-rewrite-20261016\graphics`.
Its `download-manifest.json` records each commit-pinned URL, byte length,
SHA-256 and physical line count. Eighteen source/license/readme files were
downloaded; none was executed, installed, built, linked or integrated. No
third-party bundled libraries or proprietary vendor SDK files are included.

| Repository | Observed default branch head | Commit date |
| --- | --- | --- |
| [niftools/nifskope](https://github.com/niftools/nifskope) | [3a85ac55e65cc60abc3434cc4aaca2a5cc712eef](https://github.com/niftools/nifskope/commit/3a85ac55e65cc60abc3434cc4aaca2a5cc712eef), `develop` | 2018-02-23 |
| [niftools/niflib](https://github.com/niftools/niflib) | [e29166183e23c3477fb7e08d4523dd111440947e](https://github.com/niftools/niflib/commit/e29166183e23c3477fb7e08d4523dd111440947e), `develop` | 2021-09-30 |

The official GitHub repository pages identify `develop`; GitHub's commit API
resolved each head and date during this review. Their root notices are
BSD-style three-clause terms. NifSkope's notice explicitly limits that statement
to original project sources and identifies separately licensed dependencies.
The selected files retain their original notices unchanged. Niflib's
`include/niflib.h`, referenced by individual source notices, is also preserved.
This is a file-specific source reference, not an endorsement or license finding
about every dependency in either repository.

## Direct relevance and limits

Line numbers below are physical lines in the downloaded raw files; some web
renderings omit blank lines and show different numbers.

* Niflib `include/nif_versions.h:21-25` names all three formats in the prior
  original-model inventory: 4.2.2.0, 10.0.1.0 and 10.1.0.0. Having constants
  does not establish complete parser compatibility with AA's files.
* Niflib `src/obj/NiTexturingProperty.cpp:76-115` reads apply mode and separate
  base-map clamp, filter and UV-set words for the version interval containing
  10.1.0.0; optional texture-transform data begins at 10.1.0.0. This confirms
  these are independent serialized descriptor fields, not an inference from
  a modern preview sampler. The existing local PyFFI reader supplies the actual
  original-data observations used by the change.
* Niflib `src/obj/NiBillboardNode.cpp:40-48` reads a separate billboard-mode
  field beginning at 10.1.0.0. The ordinary converter currently flattens the
  node hierarchy, so original camera-dependent behavior still requires a
  representation and native update/display proof. No billboard rendering
  change was made here.
* NifSkope `src/gl/glnode.cpp:2035-2051` creates a view-space billboard
  transform and sets its rotation matrix to identity. This particular routine
  does not dispatch on the NIF billboard mode. It therefore must not be copied
  as AA's complete implementation of the SDK's different billboard modes.
* NifSkope `src/gl/glproperty.cpp:333-399` reads per-map state and chooses
  filter/wrap settings, but `:437-441` unconditionally selects linear
  magnification, uses linear minification when a texture has only one mip, and
  selects modulation. Its OpenGL choices and fallback policy are useful
  comparison cases, not native D3D9 pixel or sampler evidence. In particular,
  these lines do not justify copying its defaults or changing AA's proven
  alpha/depth/sort behavior.
* NifSkope `src/gl/glnode.cpp:480` considers parent-hidden state. The existing
  AA corpus audit found no unflagged triangle geometry under a serialized
  app-culled ancestor and no switch/LOD nodes. Generic tool capabilities do
  not establish a newly affected AA asset in those categories.

The previous material census remains the exposure inventory: 1,847 models,
11,035 historical mesh rows, 10,699 retained rows, 7,113 retained non-team rows.
Within that last population are 285 clamped base maps, 165 bilinear/no-mip maps,
one nearest/no-mip map and 58 REPLACE rows. Those are authored mesh-row counts,
not proof of runtime visibility or an ordinary-versus-VFX routing decision.
This review did not reparse that whole corpus.

## Narrow production change

The lead assigned these files before editing; pre-change backups and hashes
are in `before/` and `before-production.json`:

* `reconstruction/tools/convert_models.py`: when a base descriptor is authored,
  export `baseTextureState` with raw `clampMode`, `filterMode` and `uvSet`.
  Preserve a descriptor even if its optional source link is absent. Existing
  texture resolution and preview UV selection are unchanged.
* `crates/aa-assets/src/types.rs`: `Material::base_texture_state` is optional;
  `BaseTextureState` retains three raw `u32` words. Old payloads default to
  `None`, which is omitted when serializing. Present records require all three
  words. Unknown numeric modes are retained rather than reinterpreted.
* `crates/aa-assets/tests/base_texture_state.rs`: exercise source-derived
  material metadata through decoding, `RawMesh::prepare`, and serialization;
  cover legacy absence, a descriptor without a source link, and raw words.

Existing converted files and manifests were not rewritten. The new metadata
is retained in converted JSON and the Rust `Material`/`PreparedMesh`; the
client's `CpuMaterial` and GPU samplers do not consume it yet. There is no new
rendering result, default sampler interpretation, billboard behavior, complete
native-NIF loader, or native game parity claim.

## Evidence and validation

`material-retention-before.json` records the pre-change materials from three
original NIFs: `IwoJimaBeachBunkerMG01`, `HealthBar`, and `AssemblyLine`.
`material-retention-fixtures.json` records all seven converted material rows,
their original source SHA-256 hashes, direct PyFFI descriptor observations,
and the post-change material JSON. The sources are all version 10.1.0.0.
Their states include nearest, bilinear, clamp and wrap, plus one row without a
base descriptor. Removing only the new metadata field reproduces every
pre-change material exactly.

`probe-material-retention.py` independently recaptures the descriptor words
from the existing reader while converting to memory, checks legacy fields and
source preservation, and writes a new evidence file only. It also probes a
descriptor with no source link. The successful recheck is
`material-retention-recheck.json`.

Validation used `CARGO_TARGET_DIR=target/s-graphics-reference`:

* Focused release/locked/offline test: 3 passed.
* Full `aa-assets` release/locked/offline tests: 16 passed, 1 existing
  recovered-asset test ignored. No build or test outside this crate was run.
* Assigned Rust files pass `rustfmt --check`; the bounded Python recheck passes
  for three source files/seven mesh rows and the separate descriptor probe.

Logs are `cargo-base-texture-state.log` and `cargo-aa-assets.log`. Exact hashes,
source citations, observations, test scope and production files are recorded
in `review.json`. The independent reviewer owns separate
`independent-review*` evidence. No SDK/public-reference executable was run,
and no native rendering oracle or pixel comparison was added or revalidated.
