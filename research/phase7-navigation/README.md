# Original saved-map region topology

`aa_navigation::topology` turns the native TERRAIN auxiliary prefix into real
source-bound routing input. Every one of the 80 inspected original maps has
enabled serialized topology and GWORLD version21..28. In `4FC875..4FC8F4`,
the original world loader rebuilds static topology only below version17.
This implementation rejects that older-world path; it does not substitute a
guessed flood-fill or infer connectivity from render geometry.

## Runtime interface

`Topology::decode_prefix(bytes, GridLayout, world_version)` returns topology and
the consumed byte count before the saved water cache. Pass the untouched
`TerrainChunk::pending_auxiliary_bytes`. `GridLayout` contains original GMap
width, height, coordinate shift and the stored binary32 world extents (the
constructor's integer ceiling converted back to binary32).

`region_at([x_bits,y_bits])` implements `7AA3D0/7B9300`: original finite bounds
checks, truncation/shift, tile rectangle and saved region identity. Out-of-map
points return None; a native invalid/null in-grid rectangle returns an error.

`prepare_graph()` supplies the actual `region_search::RegionGraph`, with source
centers, ordered adjacency, island categories, last-duplicate cached distances
and initialized visit state. It deliberately returns `ready=false`. The runtime
owner must supply its actual completed SAI startup readiness; topology alone
does not implement `7A0D40`. The existing concrete flag0 region route then feeds
`region::prepass` and the existing tile solver. Ordinary prepass calls flag0,
so flag1 route caching is not required for that path.

`post_load(&[StaticTile { flags20, definition_type }], precision)` implements the
native primary-island selection and final stored map diagonal. Supply the actual
effective terrain definition for each tile, including carried missing-definition
fallbacks. Disabled topology returns None because native leaves existing manager
fields untouched; it does not invent a new default value.

## Native ordering retained

`7AA9B0` reads regions through `7BBAF0`, preserving centers and distance-cache
pairs. Rectangles read through `7BACF0` preserve region/island words, four signed
inclusive bounds and four ordered side lists (`7B94A0`). Every side names the
original adjacent rectangle. Tile publication runs in source rectangle order;
overlapping records retain the last rectangle, exactly as native writes do.
Region rectangle membership is prepended by `7B99B0`, reversing source order.

`7BCA60` traverses those region rectangles, then side0..3 and each side's original
list order. It deduplicates first occurrences while seeding self in its temporary
list. It removes self, then reverses the output adjacency array. This ordering is
observable in route ties and is not replaced by a sorted set. Area calculations
retain signed16-bit subtraction/increment and signed32-bit wrapping arithmetic.

The loader groups regions into islands in descending region-ID order. Primary
island selection examines each island's first region/rectangle terrain type,
skips non-land groups, accumulates rectangle areas at explicit native precision
and compares against the previously stored binary32 best. A strict greater-than
comparison preserves ties. The diagonal uses the original Y-square then X-square
addition and final binary32 store.

## Evidence

- `topology_oracle.py`: complete original `7AA9B0`, real constructors, tile
  publication, island selection, adjacency, area/category helpers and original
  point lookup on all80 unchanged source maps. Only allocation/free, version
  framing and stream byte transport are intercepted. Original/v14 basic blocks
  are pinned; interrupted seam blocks do not prove execution beyond the hook.
- `synthetic_topology.py`:21 cases across PC24/53/64, including duplicate and
  ordered side links, source-overlap publication, islands, disconnected regions,
  disabled data, and native distance-hash growth/duplicate keys.
- `routes_oracle.py`:36 sequential native `53F320` searches after real loaded
  topology and adjacency, on Berlin/Crete/Midway under PC24/53/64. The SAI ready
  bit is explicit fixture setup. Exact routes, visit/predecessor state, epochs
  and busy/link state match Rust; this does not execute full SAI startup.
- Three Rust tests compare all source region fields, neighbor order, distance
  values, tile hashes, primary islands, lookup results, routes and consumed bytes.
  Every proper truncation of the small native fixtures is rejected.

Reproduce:

    python research/phase7-navigation/topology_oracle.py
    python research/phase7-navigation/synthetic_topology.py
    python research/phase7-navigation/routes_oracle.py
    python research/phase7-navigation/inspect_native.py
    python research/phase7-navigation/record_evidence.py

The actual-map tests require original local RMP files and never write them.
No full match, game save restoration or live object collision binding is claimed.
The native static-analysis builder `7ACB00`, dynamic topology edits, bottleneck
analysis, flag1 route-cache lifecycle and complete SAI/world startup remain
separate. The source-map route proof uses existing saved topology, not runtime
terrain changes. Original binaries/assets/saves and releases remain unchanged.
