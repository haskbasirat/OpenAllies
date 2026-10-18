Localization escape + moved Bank checkpoint

See findings.json and freeze.json for exact scope, tests and pins.

Native replay: PYTHONPATH=<game>/reconstruction/tools/python-packages python native_display_escape.py (default is immutable).
Rust replay: set AA_GAME_ROOT=<game>, unset AA_CAPTURE_LOCALIZATION_ESCAPE and AA_CAPTURE_STORE_FOG; frozen-localization-bank-tests.exe localization_escape::tests --include-ignored --nocapture; then same exe load_store_fog::tests::actual_stopped_load_moves_same_actor_fields_through_memory_refresh --ignored --nocapture.

Source projection checks the actual canonical manager, provenance and index getter. It is not a second native manager or a completed World source certificate. Storage handoff borrows the same Bank moved out of Attempt, with Catalog bank explicitly empty.

Actual stopped113-record prefix advances2147/16384 explored tiles and stores glyph133+Hedges. Next required child is49B480 ObjectData+118 default-aspect source. Prior memory writes and publications survive this error. No full mission or battle claim.

Production source snapshots and exact test binary are preserved. Whole shared Catalog snapshot provides historical context; only actor_display_name parameter/lookup closure is in scope. Shipping profile unchanged; test binary used LTO=false and16 codegen units.
