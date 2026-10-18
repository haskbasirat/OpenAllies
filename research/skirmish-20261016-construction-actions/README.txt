Saved Construction action bridge, 2026-10-16

Scope and reuse
New aa-runtime/src/saved_construction_actions.rs binds the existing proven
aa-actors construction_actions565140/565090 to SavedActions. The old early
Performer-construction adapter remains unchanged: it correctly expects later
Structure/Properties payloads to be absent at that earlier construction step.
This new adapter reads their actual restored storage. It does not duplicate
the pure algorithm or add gameplay rules.

Integration API
  crate::saved_construction_actions::contribute(&mut SavedActions, actor)
Runtime forwarding is owned by the shared SavedActions integrator. This task
does not edit saved_organization_actions.rs. Native owner identities are
checked for Construction, Structure and Properties before using those values.
The existing pure588990 assumes Structure.header.owner equals queried actor;
an inconsistent owner explicitly errors rather than returning a wrong result.
State and action fields are read live; only the action setter mutates storage.
No actor, component or mutable arena state is cloned.

Source and settings
ActorData can_sell(+4E4), can_raze(+4E5) and outgoing Upgrade count(+4C0)
reuse ActorCatalog::native_construction_flags. Source bool loading and the
sell-clears-raze rule come from467C16..467C63. The zero Upgrade count is
conditional on the existing complete source census proving no instances;
a newly nonempty source graph returns an error. Canonical Thing translation
identity is checked before this existing source helper performs lookup.
AllowSellUnderSiege uses the actual loaded Settings two-file Bool getter:
Game/SVars.tgi true, then Game/SVars_AA.tgi false. 474C50 and512200 publish
WorldRules by their registered definition IDS. The actual source registry has
no AllowSellUnderSiege WorldRule; the adapter checks that absence and refuses
a newly introduced rule instead of discarding its saved override. This is
saved restoration, not a claim to support later console/script SVar mutation.

Remaining explicit boundaries
Present Wall+10, Settlement+14 and585F00 upgrade readiness have no concrete
owned payload/service yet. They error when reached. Missing Settings errors
when565140 reaches the SVar lookup, after already completed bit21/22/19
writes; those writes are retained. The state21 early return never reads that
setting. Existing source error paths and coordinator/publication callbacks
remain required. No construction/upgrade/sell/raze execution is claimed.

Native validation
native_bridge_validation.py imports the old Construction oracle without
calling its fixture-overwriting main. It independently maps the original and
v14 images before actual native state registration. Each of the preserved640
inputs is executed at PC24/53/64 in both images and compared with the existing
fixture output, for3840 comparisons. No pure result is regenerated or blessed.
The six extra explicit missing-Settings seam cases prove the adapter's
partial mask prefix; this injected dependency failure is not a claimed
native exception. Both images' observed native instruction bytes are compared.
The wrapper writes its new validation report only with --write; normal use
compares the existing report. Original fixture and original oracle are pinned
and unchanged. No floating-point operation is performed by565140/565090.

Rust validation
The unchanged native_construction_actions test replays640 cases and passes.
The new actual-source adapter test passes after successful StopOnError load
of DeveloperMaps/(3v3)Firefight.RMP. It moves the actual ammo_bunker actor
into SavedActions context and reads its real Construction, Structure and
Properties allocations. Actual source fields: can_sell=true, can_raze=false,
upgrade_count=0; actual Properties flags=0; effective siege setting=false.
Explicit state variations cover normal, siege, state21 and upgrading, plus
missing-Settings partial writes and the state21 early return. Unrelated bits
and component allocation identity are preserved. An explicitly unused
Organization context supplies the attach API shape; no Organization query
or successful simulated Organization behavior is claimed.

Commands
  CARGO_TARGET_DIR=target/s-structure-actions cargo test -p aa-actors --test native_construction_actions --release --locked --offline -j4
  CARGO_TARGET_DIR=target/s-structure-actions AA_GAME_ROOT=<game root> cargo test -p aa-runtime --lib saved_construction_actions::tests --release --locked --offline -j4 -- --ignored --nocapture
  PYTHONPATH=<game root>/reconstruction/tools/python-packages python research/skirmish-20261016-construction-actions/native_bridge_validation.py
See pure-tests.log, adapter-tests.log, native-run.log and findings.json.

Preservation
Backup: backups/source-before-saved-construction-actions-20261016.zip.
No original asset, save/profile, release08, generic source parser, Research
contributor, manifest or dependency was changed. No broad census or full
playable match validation is claimed.
