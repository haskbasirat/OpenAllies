# Owned SAI startup dependencies

This package continues the separately reviewed outer startup in phase7-readiness.
It does not replace any original executable, save, asset or release file.

## Concrete operations

`aa_navigation::sai_state::State::register_kingdom` implements original 79E170:
regions in descending order, sparse Kingdom+18C state creation by 7BC130, exact
inclusive signed rectangle iteration by 7B9240, then initialized AI controllers
in slot order through 7A48F0. Existing region/Kingdom entries are retained. They
are published before tile counters are incremented. Tile occupancy uses the
low16 mask at tile+18; explored and visible use tile+14 with the original masked
shift, category exception and evaluated fog setting. These live masks cannot be
inferred from the saved terrain u16 alone.

The owned state includes 7BA0D0 seen-record head insertion (duplicates retained),
4EA430 lookup, concrete GActor virtual 4EA3D0 and seen-table creation 4EA3E0,
79E300 component classification and 4CF120 with the false argument used by SAI.
A contained object's virtual+20 result remains an explicit input to 4CF120.
It is not substituted by a generic true result.

Controller7A5750/auxiliary7CCDC0 and foreign Kingdom goals7BE920/7A66E0 are real
owned state, including native defaults, buckets, pending-list ordering and
null/nonmajor/self filtering. No later AI-player readiness follows merely from
constructing its controller.

## Fresh AI player

`aa_navigation::ai_startup::initialize(&Inputs, &RegionGraph, &sai_state::State)`
implements the nondiagnostic work of complete 7B4830 for a fresh unpublished
AIPlayer. It creates the script words and controller, excludes the final
major-list entry exactly as native does, registers per-region AI records and
three region goals, creates ability/recruitment/unit goals, publishes or removes
goals through their concrete 7A4F00 virtual predicates, then builds the actual
7C8F70 neighbor lists through the ordered bounded region traversal. Finally the
AIPlayer ready field becomes true. No guessed A* or replacement AI decision
policy is introduced.

Inputs must come from live, resolved native data:

- Logical player/controller identities and the native player index.
- Major Kingdom slots, own Kingdom, native IDs/categories/indexes and actual
  registered generation-bearing IDs. The native null registry slot is empty.
- Profile ability counts at +464/+528/+5EC; zero or negative counts create no
  goals. Game definition+570/+28 supplies the script-word count.
- Profile+3F4 supplies region-neighbor depth. Profile+884 supplies recruitment
  eligibility indexed by the option definition+C. Kingdom+230 contains the
  optional recruitment list. Null items and absent definitions are retained in
  the input and skipped at native sites.
- The Kingdom+1FC/+18 list, including null entries, supplies the final unit goal
  references. The initializer iterates this list backwards. These are logical
  references to the original list objects, not guessed actor selections.

The result owns all goal records, per-region player records and region/Kingdom
references. Controller group entries are local indexes into the owned goal
vector; they are not actor GTIDs. Memory addresses, allocator capacities and
wall-clock timing are not parity claims. This is a fresh candidate factory,
not a mutation/restart of an existing live player's graph. Errors cannot
silently authorize the outer SAI startup. The runtime must bind and publish
these owned records under the actual player index and retain its failure latch.

## Evidence

`state_oracle.py` performs 21 source-topology sequences with 168 original
79E170 registrations, plus 88 actor classifications and 48 complete seen lookups.
`controller_oracle.py` performs six ordered constructor/notification sequences.
`actor_oracle.py` performs 16 seen-table creations and 192 activity predicates.
`ai_oracle.py` performs 18 complete original 7B4830 startups, including recruitment
0/1/many selection, stale Kingdom goal deletion, sparse lists, negative ability
counts, disabled/disconnected/overlapping saved topologies and depth variations.

All native executions check stack cleanup and nonvolatile registers. The
original and modified-v14 executed blocks, full key routines, vtables and the
visible-bit constant are pinned. Native AI fixtures use only allocation/free,
stream/version transport and diagnostic name/log seams. Activity fixtures have
an explicit contained-object boolean virtual service. Native producers never
patch game bytes. Five Rust tests compare owned results with these native traces.
`record_evidence.py` regenerates fixtures, checks the focused Rust tests and
strict scoped Clippy, then records all producer/source/evidence hashes.

## Remaining boundaries

The profile/Kingdom/visibility inputs still require real runtime source binding;
the fixtures do not claim that an original map has completed live AI startup.
Outer7A0D40 special-actor registration7A0C20/7A0390/79F6E0 updates further actor,
army, region and AI aggregates; those effects are not replaced by 79E300's
classification predicate. Seen-record identity/context/position and actual
visibility population also remain live world services. Goal updates after
initial construction, old-world save restoration, repeat initialization,
full match behavior and multiplayer are outside this package.
