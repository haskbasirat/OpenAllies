# Public Axis & Allies FOV-fix reference audit

October 16, 2026. Read-only reference review. No DLL was downloaded, built,
injected, installed or run. No production source, manifest, save, profile or
release changed.

## Identity, version and license

The supplied [source page](https://github.com/alphayellow1/AlphaYellowWidescreenFixes/blob/main/source/fixes/Axis%26AlliesFOVFix/dllmain.cpp)
belongs to AlphaYellow's AlphaYellowWidescreenFixes repository. This review pins
repository commit `99a5f300685bb9a505eea7cf12e7dad3ee61875f`
(committer timestamp `2026-09-19T01:08:52Z`), rather than relying on moving
`main`. `FixVersion()` declares **1.1**. This is a source identifier, not a
claim that a downloaded release binary was checked.

The [pinned root license](https://github.com/alphayellow1/AlphaYellowWidescreenFixes/blob/99a5f300685bb9a505eea7cf12e7dad3ee61875f/LICENSE)
is MIT, copyright 2024 AlphaYellow. The complete notice is retained with the
downloaded source. This review does not audit or relicense the separate
SafetyHook/spdlog/inipp dependencies; no third-party implementation was
integrated into the rewrite.

Reference directory:
`E:/Games/Axis and Alies/reconstruction/references/public-rewrite-20261016/fov/`.
It contains pinned `dllmain.cpp`, `FixBase.hpp`, `helper.hpp`, project metadata,
upstream README and LICENSE, plus GitHub commit/tree responses and source URL/
SHA metadata. `dllmain.cpp` is 5,222 bytes, SHA256
`ac90fe78a6b7d652f4f89dc31a2784d2f01fa25cae15c71b579bce59c3c900d4`.

## Upstream mechanism and concrete source issue

The fix reads display dimensions, derives a ratio relative to 4:3, adjusts two
aspect-related multiplications and substitutes a degree-based FOV scaled by a
configuration factor. `FixBase` defaults that factor to one. Its helper applies
the usual tangent/atan horizontal-angle conversion. Compatibility checks the
executable filename and pattern matches; it is not an exact-build checksum
contract. The hook code uses the original x86 register/field layout.
[Pinned source](https://github.com/alphayellow1/AlphaYellowWidescreenFixes/blob/99a5f300685bb9a505eea7cf12e7dad3ee61875f/source/fixes/Axis%26AlliesFOVFix/dllmain.cpp).

**Static issue:** `dllmain.cpp` enters the two aspect-hook block only when
`AreAllSignaturesValid(...)` is false. In the same pinned `helper.hpp`, that
helper returns true when both matches exist. Thus valid matches skip those
hooks; missing matches enter their block. This is a source-level finding, not
a reproduced runtime failure or a claim about other revisions/release binaries.
Do not integrate this version without resolving and testing it.
[Helper](https://github.com/alphayellow1/AlphaYellowWidescreenFixes/blob/99a5f300685bb9a505eea7cf12e7dad3ee61875f/includes/helper.hpp).

The supplied translation unit also depends on its shared headers/hook runtime;
it is not a standalone engine, renderer, DPI/input fix, or 64-bit AA port.

## Independent original/v14 byte comparison

`review_native_fov.py` performed an offline wildcard scan of PE headers and
section bytes projected to image RVAs. It found exactly one match for every
upstream signature in **each** pinned original and v14 executable:

| Signature purpose | Original and v14 address | Enclosing recovered function |
| --- | --- | --- |
| Resolution fields | `0040E333` | `0040E260` application ChangeResolution |
| First aspect multiply | `006FE96A` | `006FE920` camera projection |
| Second aspect multiply | `006FE97C` | `006FE920` camera projection |
| Camera FOV load | `006FE926` | `006FE920` camera projection |

`native-fov-signatures.json` retains exact matching bytes, match counts,
disassembly, image hashes and enclosing-function leads. This is static matching,
not native Windows `VirtualQuery` page accessibility, actual hook reachability,
renderer state or a successful runtime injection test.

`native-fov-static-details.json` shows the complete 145-byte recovered camera
function range is byte-identical in original/v14, SHA256
`94daefabe1b4c70cde16b4392bec2dd3e0f63a7388f904a828da47c90e7618c9`.
Its constants are separately checked in both images: half-angle `0.5`, `90`,
binary32 radians factor `0x3C8EFA35`, and `1`. The function loads camera+218,
uses native x87 trigonometry, multiplies frustum fields+21C/+220 and
+224/+228 (the latter through+230), and calls `0088A5B0`.

The 547-byte ChangeResolution range differs between original/v14 at its first
five bytes, although the supplied resolution signature remains present.
Therefore matching signatures must not be expanded into a whole-function
identity or compatibility claim. Existing v14 interception/lifecycle behavior
still needs deliberate coexistence testing before any native patch is changed.

No original camera routine or transcendental was executed in this review.
`006fe920.c` is an indexing aid; its decompiler program-SHA header is not used
as the native executable identity. The actual original and v14 files are pinned
by the project's established SHA256 values.

## Comparison to the rewrite's current baseline

`docs/BASELINE.txt` preserves v14's separate camera-distance maximum change
from 1.4 to 2.1 and the original minimum/default/ranges/control semantics.
Camera distance and projection/FOV are distinct controls. The matching camera
routine does not establish that the external FOV fix is already installed.

`crates/aa-client/src/camera.rs` explicitly identifies itself as the inspection
camera with unproven projection parity. It uses a 43-degree vertical perspective,
its viewport aspect and chosen near/far planes; its eye position uses a
40-degree angle. Map zoom endpoints retain 0.7 to 2.1, with separate model
inspection limits. It does not port the native `006FE920` projection fields or
the public fix's hook pipeline. `aa-rules/src/camera.rs` only retains source
camera distance and a VFX reference. None of this certifies in-game visual parity.

The useful lead is now exact native projection sites. A future independently
written Rust projection port should first establish original/v14 camera field
producers, viewport/aspect convention, native frustum math/precision and a
native reference trace/pixel comparison. Any optional widescreen correction
must preserve the selected gameplay baseline and be tested separately from
zoom, mouse picking and Windows DPI. The current evidence justifies that
investigation, not silently changing the inspection FOV or shipping this DLL.

## Reproduction

From `rust-rebuild`, with the established Python native-tool package path:

    python research/public-references-20261016/ai/review_native_fov.py

Ordinary replay reads original/v14 bytes, recomputes the static result and
compares the frozen JSON. It loads no game, DLL, external C++ or SDK code and
does not touch runtime files. Reference acquisition notices/manifests retain
every URL and SHA; no game release includes these downloaded references.
