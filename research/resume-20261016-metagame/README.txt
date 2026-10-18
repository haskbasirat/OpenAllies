MGAI compatibility replay repair -- 2026-10-16

Cause
  crates/aa-metagame/tests/native_mgai.rs selected every filename beginning
  with "native-mgai-c". That also selected the newer controller fixtures
  native-mgai-controller-c0.json through c4.json. Their worlds have cw,
  yields and stages rather than the player fixture's variant and turns.
  The reader therefore panicked at world["variant"].as_u64().unwrap()
  after successfully checking all five intended player fixtures.

  The failing reader was byte-identical to its copy in
  backups/source-before-resume-20261016-152607.zip; it was not introduced
  by the current rule/map adapter work.

Change
  Only native_mgai.rs changed: fixture discovery now requires a nonempty
  decimal campaign suffix between native-mgai-c and .json. A regression
  test covers normal and multi-digit campaigns, controller names, empty
  campaign suffixes and backup extensions. No production logic, original
  assets, expected fixture data or oracle generator was changed.

Evidence and native scope
  native_mgai.py lines 240-253 writes the player schema and filename for
  native PlayTurn 0x964D30, constructor 0x963430 and its called analyses,
  goals, purchases, attacks, movement and end-turn stages.
  native_mgai_controller.py lines 553-578 writes the distinct controller
  schema and filename for native controller fiber 0x945090, including its
  player/yield/command processing. It remains covered independently by
  tests/native_mgai_controller.rs; excluding it from the wrong decoder
  does not remove its replay coverage.

  findings.json records SHA-256 for all ten fixture files and verifies
  their pinned producer, world helper and metagame-data hashes. All ten
  fixture hashes stayed unchanged across the repaired test run. This is
  a replay of existing native oracles, not fresh Unicorn execution and
  not a claim of end-to-end game compatibility.

Validation
  Environment: CARGO_TARGET_DIR=target/s-metagame,
  AA_GAME_ROOT=E:\Games\Axis and Alies, AA_METAGAME_EXPORT=0.

  cargo test -p aa-metagame --test native_mgai --release --locked --offline
    -j4 -- --nocapture
  Passed 2 tests: selector regression and all 112 player turns / 873 stages /
  889 commands / 3389 yields, with unchanged expected observations.
  Log: qa-output/resume-20261016/metagame-mgai-regression.log

  cargo test -p aa-metagame --release --locked --offline -j4
    -- --include-ignored
  Passed all 39 tests, zero failures and zero ignored, including controller
  replay, real installed data export comparison and RMC export comparison.
  Export mode was disabled; the comparison did not rewrite expected data.
  Log: qa-output/resume-20261016/metagame-compatibility-fixed.log

  The original failing log is preserved at
  qa-output/resume-20261016/metagame-compatibility.log.
