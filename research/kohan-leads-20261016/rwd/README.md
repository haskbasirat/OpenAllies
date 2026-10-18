# RWD public reference review — 2026-10-16

No production change is justified by this bounded review. The existing `aa-formats` reader agrees with the installed archive metadata; the public tool is useful corroborating reference material, with a footer-field naming trap documented below. No upstream code was executed, compiled, installed, or integrated. Original archives were opened read-only and have identical whole-file SHA-256 hashes before and after the check.

The selected public source and notices are preserved under `E:\Games\Axis and Alies\reconstruction\references\kohan-leads-20261016\rwd`. The observed `master` commit was `bea1f4e6040befe5fee6ec28d3d0670219049c6f`, dated 2024-02-09. `download-manifest.json` records the GitHub API endpoint, pinned raw/blob URLs, byte lengths, hashes, and physical line counts. Only `LICENSE`, `README.md`, and `rwd_edit.cpp` were downloaded; the manifest is audit metadata. The root license is CC0 1.0 Universal (`LICENSE`, lines 1–3), and the source repeats CC0 attribution (`rwd_edit.cpp`, lines 1–6). The upstream source credits watto.org; that secondary specification was not used as authority for a new implementation.

The [pinned upstream README](https://github.com/csernib/rwd_edit/blob/bea1f4e6040befe5fee6ec28d3d0670219049c6f/README.md#L1) describes RWD support for both Kohan II: Kings of War and Axis & Allies. Its [qualification at line 20](https://github.com/csernib/rwd_edit/blob/bea1f4e6040befe5fee6ec28d3d0670219049c6f/README.md#L20) says the author tried K2 and had never tried A&A. These are upstream claims, not two independently certified game compatibility results. No Kohan archive was available or inspected in this review. README line 18 limits packing to replacing existing members; adding/removing members is not supported.

The following observations come directly from the installed AA bytes and the existing Rust reader:

| Archive | Members |
|---|---:|
| Data.rwd | 13,428 |
| Music.rwd | 34 |
| Speech.rwd | 100 |
| Video.rwd | 42 |
| Video_En.rwd | 10 |
| skins/GBR.rwd | 226 |
| skins/GER.rwd | 220 |
| skins/JPN.rwd | 233 |
| skins/RUS.rwd | 229 |
| skins/USA.rwd | 226 |
| **Total** | **14,748** |

All ten archives have `TGCK`, version 2, and three EOF chunk records named `Header`, `Files`, and `Footer`. Every chunk has equal logical/stored lengths. Every member has zero flags and lies inside the `Files` extent. The footer count consumes its records exactly. An original audit script, `crosscheck.py`, independently decodes this metadata and compares every `(source_index, size, absolute_offset, normalized_path)` row against the existing Rust CLI's `list` output: 14,748/14,748 match in footer order. It saves canonical row hashes, table/footer hashes, exact chunk metadata, and whole-archive before/after hashes in `crosscheck.json`; it does not export archive contents. The existing 732-byte `Data.rwd:AVars_version.tgi` payload at absolute offset 22,419/source index 3 also matches the already recovered fixture byte-for-byte (SHA-256 `6cb15962a05d2488e5e5bf627c6bcafff3550851041d617c3f6d8526657ea2ab`). Full archive hashes prove preservation, not a full payload decode audit.

The public source agrees on little-endian integers, UTF-16 names, 64-bit offsets/sizes, and the 96-byte chunk layout (`rwd_edit.cpp`, lines 17–89, 123–138, 151–170). Its `Metadata` structure includes four leading bytes and assumes exactly three chunks. Its footer iterator starts four bytes before the actual first name-length field, calls those bytes `typeId`, and stops with four bytes left. Across all installed archives, the first supposed `typeId` is exactly the member count; each subsequent one is the preceding member's source index. The four bytes called `Metadata.padding` at EOF minus 292 are exactly the final member's source index. For Data.rwd these values are 13,428 and 5,902 respectively. This layout can retain those bytes through a list/pack traversal, but those names are not evidence of per-file types or intrinsic padding. The existing Rust reader already treats the footer as a count followed by names and `(relative offset, size, flags, source_index)` and preserves the source index independently of ordinal position (`crates/aa-formats/src/rwd.rs`, lines 96–108, 246–308).

The existing Rust implementation records its installed-archive/recovery-script provenance at `crates/aa-formats/src/rwd.rs`, lines 1–8. This review did not create that reader from the public source. The reader also derives chunk count and bounds from the archive, rejects malformed extents/trailing footer bytes/unsupported flags, and retains opaque chunk metadata. There is no missing format behavior established here that warrants a new production patch. A packer is outside this read-only reader's scope. The upstream pack path changes the footer offset before writing the old metadata (`rwd_edit.cpp`, lines 296–305); its ability to keep all chunk sizes consistent after size-changing replacements was not executed or certified here.

Fresh validation on the current source: `cargo test -p aa-formats --test rwd --release --locked --offline -j4 -- --include-ignored --nocapture`, with `AA_GAME_ROOT=E:\Games\Axis and Alies` and `CARGO_TARGET_DIR=target/s-graphics-reference`, passes **11/11**, including the installed member/ten-archive checks. `tests.log` records that result. `python research/kohan-leads-20261016/rwd/crosscheck.py` passes the independent metadata comparison. The existing Rust binary executed for list-only comparison is pinned in `crosscheck.json`; the downloaded C++ was never executed. Synthetic extraction tests write temporary test directories, not game assets.

Confidence is high for these ten installed AA archives' metadata and the single small payload comparison. This is not a native AA archive-loader parity proof, a Kohan compatibility test, a full payload semantic audit, a repacking test, or a new gameplay/rendering result. Native AA instructions remain the authority for runtime behavior. No original archives, game assets, saves, releases, or production sources were changed.
