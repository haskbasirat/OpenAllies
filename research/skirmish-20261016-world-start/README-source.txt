Statistic source projection and ActorData tracking inputs, 2026-10-16
=================================================================

Implementation:
  crates/aa-rules/src/native_statistics.rs (native::statistics)
  ActorCatalog::native_statistics_catalog(Precision)
  ActorCatalog::native_statistics_tracking_definition(canonical_actor, Precision)
  ActorCatalog::native_statistics_technology_properties(canonical_actor, Precision)
  tests/native_statistics_source.rs

Scoring source producer
----------------------
58E790/58E1A0 construct the 62 built-in statistic IDs in native ordinal order.
58E590 and58E6A0 construct the type/display enums. 4B5340 copies the62 names
into fresh StatisticData via4B4B10 and4B4E60. 4B5020 reuses existing named
definitions or creates a new one before invoking4B5440. 4B4960 sets production
base62, rate base62+resource_count, and derived base62+2*resource_count.

Data/Templates/template_scoring.tgi declares [list Statistic], string stat_ids,
fixed factor=1. Data/Scoring/Scoring.tgi supplies fourderived rows after seven
production and seven rate rows. 4B5440 traverses5B4220/42BE10 ordinal items,
stores the5B5130 factor return asbinary32, resolves the namedstat using4AE200,
and PREPENDS each allocated term at+1C (4B57FF..4B5816). 58F420 subsequently
reads that head-first linked list. Reversing authored order is necessary.

Shipped registry has80 rows; derived rows and head-first terms:
76 unit_limit_available: (73,-1), (72,1)
77 military_score: (21,10), (3,5.75), (34,5.75), (9,2.5), (4,12.35)
78 economic_score: (63,0.1), (64,0.1), (62,0.05)
79 overall_score: (78,1), (77,1)
Decimal spellings above describe source; API exposes actual binary32 bits.

native_statistics_source.py runs original parser5BD080, original enum andstat
manager constructors, original manager dispatch and4B5440 for the entire shipped
Scoring.tgi atPC24,PC53,PC64. Resource Find is a recorded fixture seam returning
the source-registered resource ordinal. Localization is the existing fixture
string-table seam; names do not affect numeric formulas. Registry allocation,
stat lookup and linked-list construction are unhooked original code.
Four extraPC53 cases execute native rejection: unknown referencedstat, STRING
favorite asnumericterm, forward score reference, resource row order mismatch.
Every executed native bytewindow is equal between original andv14 binaries.

ActorData tracking source
------------------------
43C: StructureComponent.TechnologyProperty. 46797F..4679A7 invokes4C4CF0;
the helper uses5B4220/42BE10, GetString(property_IDS), Property Find463FA0,
then prepends each canonical property. native_technology_source.py executes
original parser and4C4CF0 for empty, one, three, repeated/case-varied andmissing
references. Property Find is an explicit canonical fixture seam. A missing
property throws after earlier list insertions; projection returns error and
does not claim native partial-object restoration.
4D0: construction predecessor list. Existing complete source Upgrade census
proves no explicit Upgrade nodes in installed data. Nonempty graphs remain
unsupported; no missing callback is converted into a false/default fact.
4E0: ConstructionComponent.required_object_ids -> deferred4606D0 reference.
The existing native_collision_definition validator supplies canonical identity,
MARKER category, soft-shape/tile-lock consistency and empty-upgrade proof.
Its conservative Settlement-wall restriction remains. Actual source census
passed all1028 standardactors with no errors: fourtechnology lists, zero
required-object refs, zero predecessor lists. This subset thereforedoes not
block thecurrentlyinstalled actors; future mods canrequire additional binding.

Bounds and intentional differences
----------------------------------
- Fresh source projection, not full data-manager error/reload state emulation.
  Duplicate/reloaded Statistic nodes, Statistic translations, and Property
  translations in tracking inputs remain explicit unsupported errors.
- Require a native-order Resource registry; never infer a stat index from an
  arbitrary all-files catalog or alphabetic name order.
- The constructor's display3 remains on unloaded compiled stats.
- Derived formula count uses growableVec. The old10-score allocation cap in
  4B5440 is not carried into Rust; native parity cases use the shipped4rows.
- No snapshot arithmetic or gameplay callbacks implemented here; see parent's
  README.txt/findings.json and aa-economy::statistics_live for those ports.
- No claim of playable battle, multiplayer, rendering or campaign parity.

Validation
----------
CARGO_TARGET_DIR=target/s-stats-source
AA_GAME_ROOT=E:/Games/Axis and Alies
cargo test -p aa-rules --test native_statistics_source --release --locked
  --offline -j4 -- --include-ignored --nocapture
source-tests.log records the final3tests, including installed-data census.
Full aa-rules / aa-metagame compatibility is coordinated byroot toavoid
concurrent independent data-export runs. No generic parser was edited.

Backup: native.rs.before-statistics is the pre-export source snapshot.
Native game files, saves, profiles and release folders were not changed.
