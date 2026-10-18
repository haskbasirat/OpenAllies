Phase 5: initial authored visibility and Crete static scenegraph audit

The floating wheel-shaped objects in the user screenshot are the original
Doodads/Audio/Speaker/Speaker.nif editor model. Campaign/axis01_crete contains
298 instances of Things referencing this model. Source templates
Templates/template_doodad_aa.tgi define AADoodadAudio and AADoodadAudioPermanent
with Flags.invisible=true. The preview previously drew every placement model.

The fix uses the existing original/v14 SHA-bound ActorCatalog inheritance
projection with one additional field: Flags.invisible. It recognizes exact
literal true/false values only, preserving unknown expressions, ambiguous IDs,
absent definitions and source failures without guessing. One cached compact
catalog per LoadSettings maps Thing ID to the literal value and source span/hash.
The client clears only PreparedInstance.model_key for an authored invisible
Thing. It does not alter source placement records, model meshes, transforms,
terrain, original files or old packages. Rendering and geometry picking both
use the filtered prepared instances. The standalone model browser is unchanged.
This is initial authored visibility only, not dynamic reveal, fog, stealth,
controller visibility, resource actor spawning or scenario gameplay.

Observed validation:
- 3 focused visibility tests passed, including the real original/v14 Crete scene.
- Crete: 324 hidden instances, zero unresolved Thing IDs; source map records,
  terrain and all model geometry preserved exactly. 298 are audio markers;
  the remaining 26 are other source-invisible placement markers/actors.
- Initial full rule load measured 1.927 seconds in the debug native integration
  test. Later map loads reuse the compact cached visibility catalog. Model-only
  loading does not load the rules. This is not an optimized startup benchmark.
- The read-only visibility audit covers all 80 recovered maps / 225851 records:
  6908 records have authored invisible=true. 62 maps contain one or more
  unresolved IDs; those instances remain unchanged. These include legacy
  developer/trailer aliases, ambiguous native Thing IDs and the previously
  documented USA carrier parser gap; the report lists them explicitly.
- ActorCatalog retains 1645 definitions. It now projects 8162 raw field values:
  previous 6535 plus 1627 resolved visibility flag values. Previous historical
  reports are retained unchanged.

Static scenegraph audit (read-only, no geometry rewrite):
- All 127 unique NIFs referenced by the Crete map are read from original sources.
- 331 native geometry nodes are inventoried, with transforms/mesh content not
  modified. 223 exported meshes match native hierarchy and vertex/triangle count.
- Every one of the 108 omitted meshes is accounted for: 55 authored hidden
  meshes, 53 native Sg/Sg2 shadow-tag meshes. No unexplained omitted main mesh.
- No exported geometry is beneath a hidden ancestor in this scene; no switch
  nodes or geometry visibility controllers are present in this selected corpus.
- This audit does not establish native LOD, animation or runtime scenegraph
  parity. Alpha blend/depth state fixes are a separate material-render task.

Commands from rust-rebuild:
  cargo test -p aa-client visibility::tests -- --include-ignored --nocapture
  cargo test -p aa-rules --test actors
  cargo run -p aa-rules --example phase5_visibility
  python research/phase5-scenegraph/audit_scenegraph.py

Artifacts:
  visibility-audit.json
    SHA256 78c1688cd06845be0e7ddc77863d3ab63227aaed85630e3f48640ffc04268d2f
  crete-scenegraph-audit.json
    SHA256 e3b845df7dcae548141fac8f058c87374dba6e7ad4467eb37d60d2db9cc7c42a
  Both reports bind the source identities used and preserve unresolved data.
