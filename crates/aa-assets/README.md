# Recovered asset access

`aa-assets` reads the existing recovered inspection assets in the sibling
`reconstruction` directory. It never modifies the installed game, archives,
converted assets, profiles or native releases.

```rust,no_run
use aa_assets::{AssetStore, HeightMode};
let assets = AssetStore::open("../reconstruction")?;
let scene = assets.load_scene("Campaign/tutorial01", HeightMode::NativeConditional)?;
# Ok::<(), aa_assets::AssetError>(())
```

Map identity includes the entire path below `Maps`, without `.RMP`. Basenames
are not unique. Models resolve only through the manifest's full `byPath`
resource keys; the ambiguous `byName` table is not used. Local texture URLs can
be resolved with `AssetStore::resolve_url`, which rejects traversal and paths
whose canonical target escapes the recovered root.

Raw structures preserve source paths, undecoded-placement errors, extractor
warnings, both height grids and the original actor coordinates. Prepared data
uses native `(x,y,z)` -> render `(x,z,-y)`: ground X/Y becomes X/-Z and native
height becomes Y. Converted model vertices already have this transform and
their UV V flip baked in; the crate applies neither transformation twice.

The terrain mesh preserves one set of vertices per cell so tile colors and UVs
can differ across a boundary. Batches are grouped by palette index. Each cell
uses corner order `(0,0),(1,0),(0,1),(1,1)` and upward-facing triangles
`0,1,2 / 1,3,2`. Terrain and model vertices both expose position, normal, UV and
color arrays in that order. Prepared model meshes retain original materials,
alpha settings, two-sided flags and static-skin-pose metadata.

Parallel work is indexed before dispatch. Collection preserves source order;
unique model keys and texture batches are sorted. Normal accumulation uses a
serial triangle order rather than nondeterministic parallel floating-point
reductions. No random model variant selection is introduced.

## Limits of these recovered intermediates

- `NativeConditional` uses the reviewed terrain metadata sidecar and native
  render-query 0x122 control flow: current/neighbor mountain or water permits
  the extra height grid; the current mountain tile also adds deterministic
  authored noise. Grid vertices avoid fractional sampling ambiguity. Missing
  active metadata falls back to base-only with explicit warnings; commented
  aliases are not revived. All retail/campaign maps have resolved metadata.
  The JSON exporter rounded float32 heights to five decimal places, and f64
  intermediates do not promise x87 bit equivalence. Arbitrary-position queries
  are inspection interpolation, not exact native collision queries.
  `BaseOnly` and `BasePlusMountain` remain diagnostic alternatives.
- Native terrain blending and per-tile orientation flags are not reproduced.
  Texture repetition uses the recovered palette dimensions with an eight-unit
  fallback when unavailable.
- Actor XYZ and scale were decoded from native records; yaw is an inferred
  XY heading. Pitch/roll and model forward-axis interpretation still require
  visual comparison. An unresolved model variant uses the first authored
  model with a warning, rather than inventing native random state.
- Models are converted authored poses. Animation, LOD switching, procedural
  effects, collision, omitted shadow/helper geometry and native team coloring
  are not reconstructed by this crate. Missing conversions remain warnings.
- This is asset loading and deterministic preprocessing, not game-rule parity.

Allocation guards derive from the address space, actual source lengths and
the present mesh's u32 index representation; there is no arbitrary map-size
or JSON-file-size limit. Larger-than-u32 terrain meshes would require a future
chunked representation. JSON reads use one opened file and its recorded length,
rejecting short reads or growth; large output reservations are fallible.

Source tools: `reconstruction/tools/map_catalog.py`,
`reconstruction/tools/archive_map_actors.py`, and
`reconstruction/tools/convert_models.py`. Native height-query evidence is
documented separately in `rust-rebuild/research/engine-baseline`. That reviewed
terrain sidecar is included at compile time; native-executed height fixtures
cover the control flow used by the Rust tests.

Run focused tests with `cargo test -p aa-assets`. To run the local recovered
asset integration test explicitly, set `AA_ASSET_ROOT` to the recovered root,
then `cargo test -p aa-assets -- --ignored`. The `inspect` example reports a
scene without launching a game or graphics API.
