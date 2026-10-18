# Raw model source coverage checkpoint

The exporter attempted all 1,847 entries in the existing complete converted
model/material audit. It published 1,582 schema2 source projections and retained
265 explicit failures. Every failure is repeated or aliased NiAVObject ownership;
none was converted into an empty graph.

The index is a source-coverage superset. No authoritative reached Firefight model
inventory exists yet, and these artifacts do not certify native processing,
rendering, ResourceTask completion, or GameReady.

`index.json` schema2 `by_source` is authoritative. Its canonical lowercase slash
path plus source SHA selects the exact converted pairing. Fifty-two paths form
21 groups of identical source SHA values; each successful alias has its own
path-hashed artifact. Historical `graphs` retains only the first canonical path
per SHA and must not select a different alias's converted model.

Artifacts were written to a private staging directory, checked against their
hashes, and then installed. The index was replaced atomically only after every
referenced artifact existed. The original three schema2 graph files have exactly
the same bytes as before this batch:

- GBRCorps: `4b50b78973ee9b4872f1014fb3627ab953e1b2463417ffe61751d4fdcd16b72d`
- GBRCorpsPack: `926cd4050ad96759bbc6283d3b7830e3e6f2e0374bfdb2d3e1ec460c6862d72b`
- GBRDepotOil: `92f79f02818f2024bbcf30632bf613b54269db2785320c920da6cf9433f094d9`

The independent exhaustive verifier checks the published index, every artifact
digest, source/converted/material-audit pairing, canonical alias filenames,
geometry/property/texture allocation identities, node ownership, and every
converted mesh's source-node name, hierarchy and vertex count.

850 projections have no currently known structural exclusions in that inventory.
Overlapping exclusions include 604 controller graphs, 225 skin graphs, 168 other
node classes, 69 processing tags, 51 internal textures, and 38 NIF versions outside
the current client loader's admitted version. Unknown classes remain represented
as their actual names and retain the runtime's explicit required boundaries.
The batch receipt's looser 1,131 `static_scope_candidates` count only tests active
controller bits; the independent 850 count also excludes every controller chain.
Neither count is an executed Model backend acceptance count.

The 51 internal-texture projections retain the raw descriptor filename while the
source texture owner is marked internal (`filename: null`). This requires the
actual internal-texture loader and the current backend refuses it. The first
verifier logs preserve discovery of that mismatch; the final verifier reports it
as an explicit exclusion instead of asserting external-texture identity rules.
External descriptor/source filenames still must match exactly.

Inputs remain unchanged in the original and converted asset directories. No
source archive or proprietary asset integration was performed. The generated
raw graph/index data lives only in the existing local `assets-cache` area.

Evidence:

- `raw-model-batch.json`: all 1,847 per-source outcomes, exact producer/input pins.
- `raw-model-coverage.json`: independent exhaustive checks and exclusion rows.
- `raw-model-batch-run01.log`: successful generation/publication (140.72 seconds).
- `raw-model-verification03.log`: final exhaustive validation PASS.
- `raw-model-verification01.log` and `02.log`: retained internal-texture discovery.
- `raw-model-batch-freeze.json`: current source/evidence/index pins.

Separately, the bounded read of unprocessed 87AA10/879A90 after the BT child proof
identified additional mandatory material/alpha derived-extra and graph visitor
work in the client metadata implementation. That gap was sent to its owner; this
source-projection checkpoint does not close it.
