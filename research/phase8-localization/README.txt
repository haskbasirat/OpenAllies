Original DisplayString byte decoding -- 2026-10-16

Problem and bounded change

The restoration inventory mislabeled two GER_Art_Map.RMP actor names as font
escapes. Their source MAPSTRINGS bytes are name_192 = Fallschrimj[E4]ger and
name_195 = Fallschrimj[E4]ger-F[FC]hrer. They are ordinary no-BOM byte strings.
The original character reader widens these bytes to UTF-16 U+00E4/U+00FC.
aa-runtime localization previously accepted only printable ASCII and rejected
both names. It now accepts the proven no-BOM high-byte path, preserves source
bytes/hash and text spacing, and removes exactly one leading @ as the original
DisplayString loader does. It does not reinterpret no-BOM bytes as UTF-8 or as
Windows-1252. Existing declaration order and duplicate-entry refusal remain.

Original functions

  856F90  character reader; +0C/+0D distinguish UTF-16/UTF-8 input modes
  96CBD7  CRT lead-byte test against image C-locale table
  9710C6  mbtowc wrapper using thread locale
  971006  mbtowc C-locale path: unsigned input byte -> UTF-16 unit
  5BEE60  quoted localized-value parsing
  5BA3C0  DisplayString storage; 494000 escape stage, then one leading @ removed
  494000  no-escape fast path only in this change
  4BA710  TextData load/registration
  4B9F40  DisplayString lookup

Evidence

native_display_bytes.py runs original executable code with the prior documented
MapStrings loader harness. It checks that the seven relevant character/decode/
lookup entries executed and pins every visited original instruction window
against both original and v14 images. Null-page guarding is enabled in the
shared harness. Heap, CString allocator, Win32 primitives, thread C-locale
storage, logging, timers, parser lock, LanguageIDS and DataCheckingStrict are
explicit seams. No byte decoder or lookup result is replaced by a seam.
There is no floating-point arithmetic in the ported byte transform.

native-display-bytes.json contains complete source bytes and hashes, 13
numeric UTF-16 lookup results, source-image hashes, producer/dependency hashes,
and 1,274 visited instruction-window hashes. The synthetic case covers every
byte 80..FF in one string, UTF-8-looking bytes without a BOM, C1 bytes, final
high bytes, a single/double/embedded @, empty text, and spacing. The second case
loads the complete original developer-map MAPSTRINGS chunk and resolves both
affected keys. decoder-instructions.txt contains relevant original instructions.

Replay

  PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-localization/native_display_bytes.py

  CARGO_TARGET_DIR=target/s-restore-string
  AA_GAME_ROOT=E:\Games\Axis and Alies
  cargo test -p aa-runtime --release --locked --offline -j 4 \
    --test localization_bytes --test localization -- --include-ignored

The standalone fixture test needs no installed source assets. The ignored
developer-map integration test reads the original RMP, checks its SHA256,
decodes MAPSTRINGS through add_map_strings, and resolves both names. The prior
ignored tutorial localization coverage is included in the same command.

Limits

This is not a full DisplayString escape-table port. Ampersand and backslash
escapes are explicit unsupported results; ampersands were previously accepted
literally even though native 494000 interprets them. Non-ASCII BOM-marked UTF-8
input uses a distinct native OS-conversion path and remains unsupported.
Control bytes and inner quotes remain outside this runtime loader's bound
subset. Duplicate registration semantics, language selection beyond the
existing default-node restriction, and missing-string fallback are unchanged.
No generic TGI parser, map object restoration, catalog, or gameplay code was
edited. Fixing these two names alone does not make the developer map playable.

Validation results are recorded in findings.json and qa-output/
restore-string-tests.log after the scoped test run completes.
