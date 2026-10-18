Phase 5: asset-free world-fragment integration diagnostic

Run AxisAlliesRust.exe --world-check from any current directory. The option runs
before asset discovery, renderer creation, worker-pool configuration, or native
game/profile access. It prints a JSON report and exits unsuccessfully if an
expected native state, byte sequence, or callback sequence differs. It cannot
be combined with --mechanics-check; that existing diagnostic is unchanged.

The compiled sample bundle has 24 exact selected original-instruction oracle
records, six per area:

* Actors: base Thing constructor fields, facing normalization, startup direction
  table selection, raw RNG progression, incoming record mutation, and saved or
  allocated identity. Resolved definition flags and precision are fixture inputs.
* Navigation: activation success/failure, ordinary and formation path events,
  organization catch-up, and callback mutation during random scheduling. Paths
  are already prepared. This check does not produce or solve a route.
* Orders: native raw command decode/encode and exact read/write cursor positions
  across selected bit offsets and optional fields. Registry sizes are explicit
  inputs. Successful decoding does not resolve, authorize, or execute a request.
* Spatial: native height queries, old-coordinate ground height behavior, snapping,
  position commit, and ordered mutable shape/tile/notification callbacks. Height,
  shape, tile ownership, and presentation implementations are fixture boundaries.

These checks invoke independent Rust libraries. They are not wired together into
a fabricated live world, and no command is passed directly to Mover. The report
includes source executable identities, source corpus hashes, sample coverage,
actual and expected results, scope, and unfinished systems. Original native ABI
preservation is tested by the original corpus, not claimed by this Rust check.

Reproduce the selected bundle from its reviewed source selections:
    python research/phase5-client/build_samples.py

The script verifies source-file hashes, compares every selected row unchanged
with its full oracle corpus, and requires matching original/accepted executable
identities. The resulting world-samples.json SHA256 must match SAMPLE_SHA in
crates/aa-client/src/world_check.rs. The embedded bundle is sufficient at runtime;
no separate fixture or original executable file must accompany the diagnostic.

Focused validation:
    cargo test -p aa-client world_check -- --nocapture
    cargo clippy -p aa-client --all-targets -- -D warnings

Tests run the pinned bundle, deliberately alter one expected result in each of
the four areas and require each mismatch to fail, and reject an empty component
instead of issuing a vacuous passing report. Full source-library corpus coverage
remains in aa-actors, aa-navigation, aa-orders, and aa-spatial tests.

This is a reconstruction diagnostic, not a playable match. Derived actor
factories, world publication, route planning, full collision/terrain services,
outer AI scheduling, combat, campaigns, original saves, and multiplayer are not
implemented by this option.
