# Native port rules (read before any work)

## Goal (from the user)

Rewrite the ORIGINAL Axis & Allies 2004 game (`E:\Games\Axis and Alies\AA.exe`,
v14 expanded target, see `docs/BASELINE.txt`) in Rust, so it:

- behaves like the original: every gameplay behaviour is ported from the
  original machine code, never designed by us;
- is 64-bit (x86_64), with no 32-bit address-space ceiling;
- is multithreaded wherever that cannot change results;
- has no artificial engine limits: no fixed memory pools, VRAM/texture
  budgets, disk/streaming caps, bandwidth or buffer caps. Hardware and network
  limits still exist; never promise "unlimited".

An earlier interim simulation (`aa-game`, `aa-net`) was written from data files
instead of from the original code. The user rejected it. It is archived in
`backups/removed-crates-20261014/`. Do not copy from it or revive it.

## Sources of truth

| What | Where |
|---|---|
| Original executable (never modify) | `E:\Games\Axis and Alies\AA.exe` (image base 0x400000) |
| Decompiled pseudocode, one file per function | `reconstruction/research/native-aa/pseudocode/<addr>.c` (42,171 files, Ghidra auto names `FUN_xxxxxxxx`) |
| Inventories | `reconstruction/research/native-aa/{functions,calls,symbols,strings,string-references}.tsv`, `findings.json`, `object-serialization.json` |
| REA CLI (morluto/rea 6.3.0, Ghidra provider) | `rust-rebuild/research/rea-locked.ps1 <command> ...` (machine-wide lock: only one Ghidra at a time). See `reconstruction/research/REA-SETUP.md`. Prefer the exported pseudocode; use REA for xrefs, instructions, data types, value tracing and fresh decompiles. |
| Unicorn/Capstone/Keystone for Python | `PYTHONPATH="E:\Games\Axis and Alies\reconstruction\tools\python-packages"` |
| Native oracle harness pattern | `research/phase6-kingdoms/native_teams.py` (`Machine`), `research/phase7-restoration/native_*.py` |
| Original data (TGI rules, UI, maps, art, audio) | `reconstruction/assets/original/Data`, converted art in `reconstruction/assets/converted` |
| Prior native port and its limits | `HANDOFF.txt`, `research/phase2..phase7-*`, crate docs |

Pseudocode is decompiler output, not source: check stack/x87/float handling
against the instructions wherever it matters.

## Method for every behaviour

1. Locate the native function(s): strings and TGI keys -> `string-references.tsv`
   -> callers in `calls.tsv` -> vtables/constructors. Record addresses.
2. Read the pseudocode and the instructions. Note binary32/x87 precision, RNG
   draws, iteration order, error paths and partial-failure side effects.
3. Port to Rust with the same order, arithmetic width, RNG consumption and
   failure behaviour. Cite native addresses in doc comments.
4. Prove it: run the ORIGINAL code under Unicorn on generated inputs (seams
   hooked explicitly) and save JSON fixtures under `research/phase8-<area>/`.
   Add Rust tests that replay every fixture. No oracle means no parity claim.
5. Write `research/phase8-<area>/README.txt` and `findings.json` with addresses,
   cases, what is proven, and the open gaps.

Never invent defaults, never turn a missing callback or payload into success,
never "simplify" a native rule. If something cannot be proven yet, leave an
explicit unsupported error and record it.

## Emulator (Unicorn 2.1.4) caveats — check your harness

Found by the phase-8 agents; each can silently corrupt an oracle:

- FS base 0: with FS mapped at 0, a null-pointer read of [0..3] returns the
  SEH chain pointer instead of faulting. Make page-0 access fault unless the
  instruction carries the FS prefix (research/phase8-ai/native_machine.py).
- Missing allocator seam: native code falls into the real heap and crashes,
  which then looks like a "native fault". Compare the recorded seam calls
  before accepting any fault.
- x87 transcendental ops (FPATAN, FSIN, FCOS, FYL2X, F2XM1) are computed via
  binary64, and PE is never set from inexact arithmetic: 80-bit results of
  those instructions are not ground truth (research/phase8-tick/emulator-limits.json).
- Signalling NaNs are not quieted by x87 `fld`/`fst` of m32/m64; SSE picks the
  wrong operand when both inputs are different NaNs. Follow hardware behaviour
  in Rust and mark such rows.
- Rewriting code at an address that already executed runs the stale
  translation: write each stub to a fresh address.

## Precision

The game simulation thread runs at x87 PC53 (Double53); the main/UI thread at
PC24 (research/phase8-tick/fpu-precision.md). Prove at PC24/PC53/PC64 and keep
precision an explicit input. The x87 PE flag is set on every simulation tick.
Exact CRT `pow` (SSE2 path, the path every SSE2 CPU takes) is
`aa_numeric::crt_pow`; reuse it rather than writing another.

Configured-v14 startup qualification (2026-10-17): the actual installed D3D9
wrapper captures show the original main thread executing Grid430290 at PC53
(CW027F), both at entry and its43F124 return, in two separate launches.
Do not choose PC24 just because an operation is reached through a UI callback.
Keep the measured PC53 startup reference explicit; this does not establish the
precision of every later renderer call. See
`research/development10/view-startup-execution-role/findings.json`.

## Limits, 64-bit and threads

- Native fixed-size engine arrays, pools, caps and buffer sizes become growable
  Rust structures. Record each removed cap (address, original value) in your
  findings. Gameplay RULES (for example configured unit caps from v14 settings)
  stay as rules with their original defaults.
- Native pointers become typed IDs/handles. Saved-game and network wire formats
  keep their original 32-bit layouts where compatibility applies.
- The simulation stays deterministic: native update order, a single
  authoritative RNG sequence, float reductions in native order. Parallelise
  only provably order-independent work (pure reads, independent per-actor
  computations merged in native order, asset loading, rendering, audio,
  pathfinding queries whose results are applied in native order).

## Working in parallel (many sessions share this checkout, no git)

- Edit only the files your brief assigns you. For a change in someone else's
  area, message the lead session that started you and describe it.
- `crates/aa-runtime/src/lib.rs` is shared: you may append one `pub mod` line;
  re-read the file immediately before editing.
- Root `Cargo.toml`, `Cargo.lock` and new external dependencies: ask the lead.
- Build only your crates with your own target dir, e.g.
  `CARGO_TARGET_DIR=target/s-<name> cargo test -p <crate> --release --locked --offline -j 4`.
  Never run `cargo test --workspace`.
- Changing aa-rules parsing or definition loading? Afterwards run the
  aa-metagame data export check
  (`cargo test -p aa-metagame --release --locked --offline -- --include-ignored`)
  and the aa-rules real-data tests with AA_GAME_ROOT set, and report changes.
- Never modify original game files, saves or profiles; never overwrite
  `release/`; this folder is not a git repository.
- Report honestly: what is proven by oracles, what is only compiled, what is
  untested.
