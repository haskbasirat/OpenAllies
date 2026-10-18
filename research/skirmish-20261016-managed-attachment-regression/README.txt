Managed attachment regression correction,2026-10-16

Only crates/aa-runtime/tests/element_attach_restore.rs was changed. The prior
source is backed up at backups/test-before-managed-element-attach-20261016.zip.
No production files or original assets/saves were changed.

The previous combined acceptance failure is retained in
qa-output/economy-lifecycle-20261016/accepted-runtime-integration.log. It used
standalone mode1, whose observer ownership correctly remains Unknown, but
expected the superseded generic570020 resource callback boundary.

The updated diagnostic explicitly binds actual ResourceCatalog metadata and
a fresh managed Normal loading token from real UVars sources. Its375ms
constructor clock is a labelled fixture, not an observed original uptime.
The same token owner, immutable source Arc, Ability arena and definition bank
are retained when testing direct attachment. No Kingdom arrays are reset or
synthesized.

Natural tutorial02 StopOnError now reaches saved actor4099,source ordinal434,
regiment_usa_hq_infantry_unpacked, then reports the exact next service:
5788B0 combat value over the restore shells is unported. The test asserts
that ID,ordinal,error and runtime_publication_ready=false. Immediate per-record
Init/PostCreate and whole-world publication are still omitted.

All8 historical original/v14 prefix cases continue to compare Element
organization/slot/attached writes. Of the direct diagnostic calls,4 valid
state27 cases now return through the implemented contributors,gated HQ Economy
and inactive observer;4 state0 cases retain the explicit582C40 missing
group13 boundary. The older oracle proves only the membership prefix through
Recruit entry,not later contributor masks or full attachment parity. The
fixture resets only flags/state/actions/membership; later component effects
may cascade,so repeated cases are not independent full native simulations.

Verification:1 scoped integration test passed,including all8 prefix checks.
Final log:qa-output/economy-lifecycle-20261016/managed-element-attach-tests.log
Final observation:qa-output/economy-lifecycle-20261016/managed-element-attach.json

Run (PowerShell,from rust-rebuild):
$env:AA_GAME_ROOT='E:/Games/Axis and Alies'
$env:CARGO_TARGET_DIR='target/s-saved-economy-integration'
cargo test -p aa-runtime --test element_attach_restore --release --locked --offline -j4 -- --include-ignored --nocapture
