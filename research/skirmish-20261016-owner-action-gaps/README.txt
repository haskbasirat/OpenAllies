Owner action rebuild gap audit, 2026-10-16

Result
  No new native core is justified for the first reached Organization owner.
  Existing aa_actors::ability_flags::rebuild ports4D1350, Organization57B390,
  Element571640 and their ordered mask/eligibility reads. Existing
  aa_actors::action_publication::changed ports4CFCC0 and529D70. Root/adapter
  author is connecting that existing coordinator to the owner restore context.
  This audit changed NO production or test source, manifests, saves or releases.

Native/source checks
  Compared original4D1350 and4CFCC0 pseudocode and actual instructions to the
  current Rust. Native instruction windows4D1350..4D1488,4CFCC0..4CFD2C and
 529D70..529D78 match acceptedv14 bytes. See native-instructions.txt.
  Re-ran existing native_ability_flags (4tests) and native_action_publication
  (2tests):6passed. Reused fixture counts are290 mask/member-scan inputs and
 100 publication inputs; this audit does not count them as fresh native runs.
  No old producer was executed because action_publication.py unconditionally
  overwrites its historical fixture. Native byte checks and Rust replay were
  read-only with respect to those source fixtures.

Actual prior checkpoint and bounded inference
  qa-output/public-build-options-20261016/element-attach-final.json contains the
  current pre-owner-binding stopped prefix: owner4099, source ordinal434,
  regiment_usa_hq_infantry_unpacked. It has ONLY Properties and Organization.
  Thing flags2156158976; state low134217728/high4; current masks F0=1048576,
  F4=2; parent Kingdom3172; layout regiment_hq; members[0,0,0,0,4100].
  This owner's4D1350 path therefore has no Element, Flare, Recruit, Research,
  Settlement, Structure, Denizen, Construction, MobileDeployment or Mind
  contribution. Properties and Experience/rank are NOT direct4D1350 contributors.
  Organization57B390 still must use real member masks, resupply/Kingdom singles,
  selected-member pointer, actual ordered locked-group abilities and canonical
  recruitment state. These callbacks already have bindings in SavedActions.
  Publication is conditional on changed masks. The actual loaded Object+4C
  constructor field is0 and Thing+C is0; provided those actual fields remain
  unchanged,76C7F0/UI and renderer publication are skipped by native gates.
  This is a source/checkpoint inference, not a newly executed owner-call result.
  The adapter author's fresh actual-map test must certify current reachability.

Precise remaining required services
  SavedActions's only unbound direct contributor dispatch slots are Flare+C8
  virtual+38 and Settlement+98/585AE0. The latter has bounded existing pure
  logic in aa_combat::walls::refresh_flag13, but no saved storage adapter.
  The 'walls' module's historical naming does not change actual slot identity.
  Neither slot is present in the owner4099 checkpoint. Do not add a default
  success or port unrelated contributor code to work around a reached error.

  Other already-connected contributors may stop at their real dependencies:
  * Organization579580 can reach nonempty Kingdom HQ eligibility573E50 and
   574BF0/574810. Constructor-empty actual HQ vectors return natively.
  * Recruit582C40 can reach5827B0 requirements/eligibility; this remains required.
  * Construction can reach585F00 Settlement upgrade readiness or actual Wall/
    Settlement reference payloads. Research/Denizen/Structure have explicit
    payload/identity requirements. These cannot be treated as absent slots.
  * Mind+B4 has actual owned core/state/command/Ability handling, settings,
    Properties modifier and MobileDeployment predicates. Missing arena/source
    state remains an explicit error. Its typed Actor+108 check must use current
    actor_list_108 presence, not resurrect a duplicate numeric pointer word.

 4CFCC0 publication:
  * Element's parent is an Actor. Native4CFCD4 reads that Actor+7C Organization;
   529D70 reads its+4 owner and synchronously jumps4D1350. It is not scheduled.
    SavedActions reads actual Organization.header.owner before rebuilding.
  * If virtual+108 returns nonnull and signed Object+4C>0, it rereads virtual+108
    and calls76C7F0, forwarding even a second null owner. Saved owner-view
    publication remains explicitly unbound.
  * Then it rereads Thing+C and its+54 flags; bit17 calls50C8E0 selected-view
    publication. Nonnull saved renderer flags and selected-view publication
    remain unbound. They must be bound to actual world/UI/render ownership when
    those gates are reached. Mask writes remain committed on a later failure.

Scope limits
  This is a gap audit plus existing core regression replay. No new native leaf,
  full owner-call native proof, active UI/world service or completed battle is
  claimed. Init/PostCreate, complete publication and deterministic battle ticks
  remain separate work. No SDK implementation was consulted or copied.
