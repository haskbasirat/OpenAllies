# Original animation inventory audit

This is a read-only source/cache audit. No exporter, crate build or native animation execution was run; no assets, crates or release files were edited.

All 2,481 NIF/KF sources match installed Data.rwd member bytes and extracted originals: 1,900 NIF and 581 KF. The prior parsed source census is therefore bound to the exact installed bytes. It contains 743 NIFs with embedded controllers.

The browser enumerates 46 top-level rig files with 85 model/clip bindings and 69 distinct KF files. It ignores nested game/ rigs: 58 files with 317 role clips. Across all 145 rig files (including 41 staging duplicates), caches cover 99 model sources and 98 distinct KF sources. 483 original KF files have no cached clip. All current cached clip sources are KF files; embedded NIF timelines are not exported as source clips.

There are 20 HQ Death NIFs, 5 Corps Death NIFs and 77 building Death NIFs (case-insensitive filename classification). None of those building death models is in the browser list. Original building destruction uses embedded timelines, not necessarily a separate KF file. Full paths, source hashes and controller records are in hq-building-deaths.json.

Typical gaps are varying Euler curves (699 source assets), varying scale type2 (374), quaternion type3/TCB (26), alpha, visibility, texture transforms/flip, material/light color, look-at/path and particle-system controllers. These counts overlap. Inventory presence is not playback support. Complete destruction playback must retain and implement all reached controllers, not silently discard alpha/particles or fabricate tracks.

Use original-source-catalog.json for authoritative source identities and complete controller/sequence records. Use cache-coverage.json for existing cache files and exact skipped reasons. The older phase3-animation/animation-catalog.json supplies qualified recovered model/KF associations; it does not prove all native template/list resolution or runtime selection. Do not choose a model for unresolved/default Character animation lists by basename.

The findings recommend a complete source catalog with explicit supported/unsupported state, exact model/clip identity and an embedded-NIF timeline path. Existing native rules remain the behavioral authority; this audit does not authorize gameplay from the archived aa-game projection.
