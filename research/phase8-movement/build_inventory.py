"""Build research/phase8-movement/inventory.json.

Scope: every Ghidra-recognized function in the native movement/navigation
source clusters (identified by assertion/source-file strings and vtables)
plus the movement helpers they call outside those clusters.

Columns:
  ported   - "yes" (manual: verified Rust port), "boundary" (manual: Rust
             names it as a required world service), "cited" (auto: an
             address citation exists in that crate; read it before relying
             on it), "no" (no citation in crates/).
  proven   - research directories whose JSON fixtures record >= 2 distinct
             executed instruction addresses inside the function body (auto;
             seam entries alone do not count). Manual entries list the
             phase8 oracle files.
Ghidra's call table misses some direct calls from undefined code regions
(for example 0x55E62D -> 0x576B20); callers listed here are a lower bound.
"""
import json, os, re, struct, collections
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
NATIVE = ROOT.parent / 'reconstruction/research/native-aa'

CLUSTERS = [
    (0x51e000, 0x528000, 'Tai/CAI.cpp company AI (organization orders)'),
    (0x528000, 0x52d000, 'Tai/Mind.cpp + TAI shared'),
    (0x52d000, 0x539160, 'Tai/UAI.cpp unit AI move/engage states'),
    (0x539160, 0x539fd0, 'Solver/FloodFill.cpp + solver helpers'),
    (0x539fd0, 0x53d000, 'Solver/LowPathSolver.cpp'),
    (0x53d000, 0x540860, 'Solver/PathSolver.cpp + region pather'),
    (0x540860, 0x5411d0, 'Solver/ZoneSolver.cpp (detection zones)'),
    (0x54ea10, 0x559100, 'Sim/Map (tiles, heightfield, spatial queries, water)'),
    (0x55d900, 0x561790, 'Component/AirCraft.cpp (aircraft path, vtable 0xA3AF78)'),
    (0x5766a0, 0x577f20, 'Component/Mover (no source strings; vtable 0xA3CE88)'),
    (0x577f20, 0x57d950, 'Component/Organization.cpp (vtable 0xA3D434)'),
    (0x57d950, 0x57e200, 'Organization/Element tail (no source strings)'),
    (0x7a9000, 0x7be000, 'Sai/SAIRegion.cpp region topology'),
]
SINGLES = {
    0x445400: 'signed turn angle (asin/cross)', 0x4453a0: 'vector3 normalize, returns length',
    0x4454a0: 'rotate counterclockwise by table trig', 0x4454e0: 'rotate clockwise by table trig',
    0x85da10: 'quantized trig table lookup', 0x85da80: 'table cos', 0x85daa0: 'table sin',
    0x408d50: 'unit direction table fill', 0x408ed0: 'direction table constructor (2048 at 0xB39A90)',
    0x4091f0: 'direction entry: FCOS/FSIN + normalize', 0x409230: 'vector2 normalize',
    0x444a90: 'random table direction', 0x45d400: 'extended unit random draw',
    0x85d840: 'native LCG step', 0x4f02b0: 'GWorld RNG accessor (sync log)',
    0x4ee5c0: 'actor position commit', 0x4eeb40: 'move on ground (height + commit)',
    0x4eebe0: 'actor facing setter', 0x4f0e50: 'actor terrain height query',
    0x46ee00: 'half-cell snap', 0x46ef30: 'transformed circle vs shape', 0x46ed10: 'circle distance',
    0x46e400: 'circle-box overlap', 0x4ea0f0: 'soft shape descriptor', 0x46f100: 'soft shape range',
    0x4cede0: 'actor moving predicate', 0x4d03d0: 'Properties+1C bit1 movement suspension',
    0x4d67f0: 'overlapping-thing list refresh', 0x4d10a0: 'actor eligibility predicate',
    0x45d6f0: 'definition speed with modifier context', 0x5014c0: 'modifier multiply (+38)',
    0x4c11e0: 'TERRAIN_SPEED_CAP / SPEED_MAX modifier parse', 0x46593e: 'MoverComponent data loader',
    0x464bd0: 'actor component data loader (Mover/Aircraft fields)', 0x576920: 'Mover constructor',
    0x4cfc10: 'activate component', 0x4cfc80: 'deactivate/stop component', 0x4cf290: 'organization slot world position',
    0x4ce6c0: 'actor or leader select', 0x44dea0: 'generation handle validate',
    0x4d64f0: 'actor state/property flag setter', 0x4d64a0: 'landing handler', 0x50ac00: 'schedule updatable',
    0x50b250: 'cancel updatable schedule', 0x4db090: 'Mover deactivate thunk', 0x4db0e0: 'component activate thunk',
    0x4d04b0: 'actor left map', 0x4d7ea0: 'actor entered map', 0x550190: 'heightfield nearest sample',
    0x550210: 'heightfield interpolated sample', 0x551e60: 'heightfield chunk reader', 0x5500f0: 'heightfield writer',
}
MANUAL = {
    # phase8 ports (this round): original x86 oracle + Rust replay tests.
    0x577e50: ('Mover updatable Tick dispatch', 'yes', 'aa-movement::mover::tick', ['phase8-movement/native-mover.json']),
    0x5768a0: ('Mover arrival stop', 'yes', 'aa-movement::mover::arrival_stop', ['phase8-movement/native-mover.json']),
    0x5768f0: ('Mover failure stop (Mind+128)', 'yes', 'aa-movement::mover::failure_stop', ['phase8-movement/native-mover.json']),
    0x577330: ('stopped-near-destination predicate', 'yes', 'aa-movement::mover::near_destination', ['phase8-movement/native-mover.json']),
    0x4d03d0: ('Properties+1C bit1 movement suspension', 'yes', 'aa-movement::mover::properties_suspend_movement', ['phase8-movement/native-mover.json']),
    0x5767f0: ('Mover stop turning', 'yes', 'aa-movement::mover::stop_turning', ['phase8-movement/native-mover.json']),
    0x576b20: ('Mover requested speed/factor setter', 'yes', 'aa-movement::mover::set_requested_speed', ['phase8-movement/native-mover.json']),
    0x576c70: ('Mover maximum speed refresh', 'yes', 'aa-movement::mover::refresh_max_speed', ['phase8-movement/native-mover.json']),
    0x576850: ('Mover turn radius/rate refresh', 'yes', 'aa-movement::mover::refresh_turn_rate', ['phase8-movement/native-mover.json']),
    0x45d6f0: ('definition speed with modifier context', 'yes', 'aa-movement::mover::refresh_max_speed (inline)', ['phase8-movement/native-mover.json']),
    0x5014c0: ('modifier multiply (+38)', 'yes', 'aa-movement::mover::refresh_max_speed (inline)', ['phase8-movement/native-mover.json']),
    0x576700: ('Mover post-load falling fixup (vtable+8)', 'yes', 'aa-movement::mover::restore_falling', ['phase8-movement/native-mover.json']),
    0x577380: ('ballistic launch speed helper', 'yes', 'aa-movement::mover::launch_speed', ['phase8-movement/native-mover.json']),
    0x576740: ('begin unaimed fall', 'yes', 'aa-movement::falling::begin_fall', ['phase8-movement/native-mover.json']),
    0x5770a0: ('begin aimed fall to ground point', 'yes', 'aa-movement::falling::begin_aimed_fall', ['phase8-movement/native-mover.json']),
    0x577a70: ('falling step', 'yes', 'aa-movement::falling::fall_step', ['phase8-movement/native-mover.json']),
    0x576bb0: ('falling pose commit', 'yes', 'aa-movement::falling::fall_step (commit_pose)', ['phase8-movement/native-mover.json']),
    0x5766a0: ('map interior predicate', 'yes', 'aa-movement::falling::inside_map', ['phase8-movement/native-mover.json']),
    0x552640: ('map interior clamp', 'yes', 'aa-movement::falling::clamp_to_interior; aa-navigation::endpoint::clamp_to_interior', ['phase8-movement/native-mover.json', 'phase6-navigation']),
    0x4453a0: ('vector3 normalize, returns length', 'yes', 'aa-movement::falling::normalize', ['phase8-movement/native-mover.json']),
    0x5797e0: ('Organization ComputeSlotPosition', 'yes', 'aa-movement::formation::compute_slot_position', ['phase8-movement/native-formation.json']),
    0x578250: ('formation offset rotation by facing', 'yes', 'aa-movement::formation::rotate_offset', ['phase8-movement/native-formation.json']),
    0x57bef0: ('compute all slot positions (+ reassignment)', 'yes', 'aa-movement::formation::compute_all_slot_positions', ['phase8-movement/native-formation.json']),
    0x57ba80: ('formation slot reassignment (pairwise swap)', 'yes', 'aa-movement::formation::reassign_slots', ['phase8-movement/native-formation.json']),
    0x579a00: ('organization set leader', 'yes', 'aa-movement::formation::set_leader', ['phase8-movement/native-formation.json']),
    0x444a90: ('random table direction', 'yes', 'aa-movement::formation::random_direction', ['phase8-movement/native-formation.json']),
    0x45d400: ('extended unit random draw', 'yes', 'aa-movement::formation::unit_draw', ['phase8-movement/native-formation.json']),
    0x408d50: ('unit direction table fill', 'yes', 'aa-movement::formation::direction (embedded PC53 table)', ['phase8-movement/native-direction-table.json']),
    0x408ed0: ('direction table constructor (2048 at 0xB39A90)', 'yes', 'aa-movement::formation::direction', ['phase8-movement/native-direction-table.json']),
    0x4091f0: ('direction entry: FCOS/FSIN + normalize', 'yes', 'aa-movement::formation::direction (table data)', ['phase8-movement/native-direction-table.json']),
    0x409230: ('vector2 normalize', 'yes', 'aa-movement::formation::direction (table data)', ['phase8-movement/native-direction-table.json']),
    0x4cf290: ('organization slot world position', 'yes', 'aa-movement::formation::slot_world_position', []),
    0x52ff00: ('UAIStateMove formation speed controller (ActionGo mode-3 gate)', 'yes', 'aa-navigation::formation_follow::formation_speed', ['phase8-movement/native-follow.json']),
    0x52e400: ('formation slot lag along path', 'yes', 'aa-navigation::formation_follow::slot_lag', ['phase8-movement/native-follow.json']),
    0x52e300: ('organization minus member remaining path length', 'yes', 'aa-navigation::formation_follow::path_length_difference', ['phase8-movement/native-follow.json']),
    0x52ce70: ('path length from a point', 'yes', 'aa-navigation::formation_follow::path_length', ['phase8-movement/native-follow.json']),
    0x46b000: ('vector2 inequality', 'yes', 'aa-navigation::formation_follow::vectors_differ', ['phase8-movement/native-follow.json']),
    0x45d880: ('vector2 squared distance', 'yes', 'aa-navigation::formation_follow::squared_distance', ['phase8-movement/native-follow.json']),
    0x576d20: ('Mover requested speed clamp + factor', 'yes', 'aa-navigation::formation_follow::set_mover_speed', ['phase8-movement/native-follow.json']),
    0x535e00: ('Element UAI Mind+44 path accessor (lea +D4)', 'yes', 'aa-navigation::formation_follow (FollowOrganization/FollowLeader views)', []),
    0x5247a0: ('Regiment CAI Mind+44 path accessor (lea +9C)', 'yes', 'aa-navigation::formation_follow (FollowOrganization view)', []),
    0x555620: ('all-overlaps spatial query', 'yes', 'aa-spatial::overlaps::all_overlaps', ['phase8-movement/native-overlaps.json']),
    0x554dd0: ('shape tile iterator start', 'yes', 'aa-spatial::overlaps (start_cell/advance)', ['phase8-movement/native-overlaps.json']),
    0x5541f0: ('shape tile iterator advance', 'yes', 'aa-spatial::overlaps (advance)', ['phase8-movement/native-overlaps.json']),
    0x4efb10: ('intrusive query mark list init', 'yes', 'aa-spatial::query (mark/clear)', ['phase7-spatial', 'phase8-movement/native-overlaps.json']),
    0x4efb20: ('intrusive query mark', 'yes', 'aa-spatial::query::mark', ['phase7-spatial', 'phase8-movement/native-overlaps.json']),
    0x4efb50: ('intrusive query mark clear', 'yes', 'aa-spatial::query::clear', ['phase7-spatial', 'phase8-movement/native-overlaps.json']),
    0x4d67f0: ('actor overlapping-thing list refresh', 'yes', 'aa-spatial::overlaps::refresh_overlaps', ['phase8-movement/native-overlaps.json']),
    0x79e3c0: ('singly linked list node removal', 'yes', 'aa-spatial::overlaps::refresh_overlaps (ordered removal)', ['phase8-movement/native-overlaps.json']),
    0x520320: ('CAIStateMove activate: move to actor', 'yes', 'aa-navigation::company_move::activate_to_actor', ['phase8-movement/native-company.json']),
    0x520440: ('CAIStateMove activate: move to location', 'yes', 'aa-navigation::company_move::activate_to_location', ['phase8-movement/native-company.json']),
    0x51e9b0: ('CAIStateMove destination + clamp', 'yes', 'aa-navigation::company_move::set_destination', ['phase8-movement/native-company.json']),
    0x51d840: ('group formation destination offset', 'yes', 'aa-navigation::company_move::set_destination (group_offset callback)', ['phase8-movement/native-company.json']),
    0x51d7f0: ('group formation active test', 'yes', 'aa-navigation::company_move::think (group_formation_active callback)', ['phase8-movement/native-company-think.json']),
    0x5525d0: ('map extent clamp', 'yes', 'aa-navigation::company_move::clamp_to_extent', ['phase8-movement/native-company.json']),
    0x523080: ('CAIStateMove Enter (vtable+18)', 'yes', 'aa-navigation::company_move::enter', ['phase8-movement/native-company.json']),
    0x51d7b0: ('CAIStateMove Leave (vtable+1C)', 'yes', 'aa-navigation::company_move::leave', ['phase8-movement/native-company.json']),
    0x5205e0: ('regiment speed broadcast to members', 'yes', 'aa-navigation::company_move::set_speed', ['phase8-movement/native-company.json']),
    0x526940: ('CAIStateMove move tries / fail timeout', 'yes', 'aa-navigation::company_move::increment_tries', ['phase8-movement/native-company.json', 'phase8-movement/native-company-think.json']),
    0x52cd90: ('path reset', 'yes', 'aa-navigation::company_move::reset_path', ['phase8-movement/native-company.json']),
    0x52cbd0: ('formation slot path offset', 'yes', 'aa-navigation::company_move::offset_path_for_slot', ['phase8-movement/native-company.json']),
    0x445280: ('line intersection', 'yes', 'aa-navigation::company_move::line_intersection', ['phase8-movement/native-company.json']),
    0x52d230: ('translated path copy', 'yes', 'aa-navigation::company_move::translate_path', ['phase8-movement/native-company.json']),
    0x52e9d0: ('Element return to formation slot', 'yes', 'aa-navigation::company_move::return_to_slot', ['phase8-movement/native-company.json']),
    0x52e900: ('Element move to slot point', 'yes', 'aa-navigation::company_move::move_to_slot', ['phase8-movement/native-company.json']),
    0x5276b0: ('CAIStateMove Think (regiment movement)', 'yes', 'aa-navigation::company_move::think', ['phase8-movement/native-company-think.json']),
    0x51ea20: ('CAIStateMove plan path (Solve from leader)', 'yes', 'aa-navigation::company_move::plan_path', ['phase8-movement/native-company-think.json']),
    0x52a1f0: ('arrival within 0.5 (squared 0.25)', 'yes', 'aa-navigation::company_move::arrived', ['phase8-movement/native-company-think.json']),
    0x525740: ('member path dispatch by formation', 'yes', 'aa-navigation::company_move::dispatch_member_paths', ['phase8-movement/native-company-think.json']),
    0x523130: ('send slot formation path to member', 'yes', 'aa-navigation::company_move::send_member_path', ['phase8-movement/native-company-think.json']),
    0x523240: ('send plain formation path / order', 'yes', 'aa-navigation::company_move::send_plain_member_path', ['phase8-movement/native-company-think.json']),
    0x5233f0: ('send scatter formation path / order', 'yes', 'aa-navigation::company_move::send_scatter_member_path', ['phase8-movement/native-company-think.json']),
    0x579a90: ('organization owner state 0x16 clear', 'yes', 'aa-navigation::company_move::dispatch_member_paths (callback)', ['phase8-movement/native-company-think.json']),
    0x52ca90: ('path node payload build', 'yes', 'aa-navigation::company_move::send_scatter_member_path (inline)', ['phase8-movement/native-company-think.json']),
    0x52d180: ('path push front', 'yes', 'aa-navigation::company_move::send_scatter_member_path (inline)', ['phase8-movement/native-company-think.json']),
    0x4364c0: ('organization member iterator', 'yes', 'aa-navigation::company_move (next_member callback semantics)', ['phase8-movement/native-company-think.json']),
    0x554850: ('free placement search (rings x 8 rotations)', 'yes', 'aa-spatial::placement::find_free_placement', ['phase8-movement/native-placement.json']),
    0x554ad0: ('placement search definition wrapper (radius floor)', 'yes', 'aa-spatial::placement::wrapper_radius (+ find_free_placement)', ['phase8-movement/native-placement.json']),
    0x552f70: ('shape query constructor', 'yes', 'aa-spatial::placement::find_free_placement (query build)', ['phase8-movement/native-placement.json']),
    0x554b30: ('placement search within distance (rings 1..20)', 'yes', 'aa-spatial::placement::find_placement_within', ['phase8-movement/native-placement.json']),
    0x7acb00: ('static SAI topology builder', 'yes', 'aa-navigation::topology_builder::build', ['phase8-movement/native-topology-builder.json']),
    0x7a9b00: ('next square size', 'yes', 'aa-navigation::topology_builder::next_square_size', ['phase8-movement/native-topology-builder.json']),
    0x7a9df0: ('square free and single terrain type', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7a9ea0: ('square growth (column/row alternation)', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7a9ac0: ('tile terrain type', 'yes', 'aa-navigation::topology_builder::terrain_type', ['phase8-movement/native-topology-builder.json']),
    0x7abd40: ('rectangle split/publish/region creation', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7bac90: ('rectangle constructor', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7aacd0: ('rectangle side runs', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7b9b80: ('side list insert', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7ba4f0: ('region constructor', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7b99b0: ('region rectangle add', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7ab7a0: ('island flood', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7aa8b0: ('island flood recursion (explicit stack)', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7aa2a0: ('primary island', 'yes', 'aa-navigation::topology_builder (and topology::post_load)', ['phase8-movement/native-topology-builder.json']),
    0x7ab320: ('small rectangle merge', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7b9900: ('merge helper', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7b8e50: ('merge helper', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7bc590: ('region center and distances', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7ba3c0: ('region distance hash insert', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x606750: ('hash growth (2n+1 prime-ish)', 'yes', 'aa-navigation::topology_builder', ['phase8-movement/native-topology-builder.json']),
    0x7aa180: ('max rectangle width (CRT pow/floor)', 'no', 'input to topology_builder (x87 FYL2X/F2XM1 not modelled)', []),
    0x7ac9a0: ('ClearStaticMapAnalysis', 'no', 'precondition of topology_builder::build', []),
    0x5612f0: ('AircraftComponent::MoveTick (Aircraft vtable 0xA3AF78 +38)', 'yes', 'aa-movement::aircraft::move_tick', ['phase8-movement/native-aircraft.json']),
    0x55e2f0: ('aircraft path advance along segments', 'yes', 'aa-movement::aircraft::advance_path', ['phase8-movement/native-aircraft.json']),
    0x55dca0: ('adaptive curve arc length', 'yes', 'aa-movement::aircraft::arc_length', ['phase8-movement/native-aircraft.json']),
    0x55da10: ('cubic segment evaluation', 'yes', 'aa-movement::aircraft::evaluate_curve', ['phase8-movement/native-aircraft.json']),
    0x560050: ('holding circle plan', 'yes', 'aa-movement::aircraft::plan_holding_circle', ['phase8-movement/native-aircraft.json']),
    0x55fe60: ('holding circle leg', 'yes', 'aa-movement::aircraft::plan_holding_circle', ['phase8-movement/native-aircraft.json']),
    0x561080: ('follow formation leader', 'yes', 'aa-movement::aircraft::follow_leader', ['phase8-movement/native-aircraft.json']),
    0x5685e0: ('air formation slot offset', 'yes', 'aa-movement::aircraft::formation_offset', ['phase8-movement/native-aircraft.json']),
    0x4cf1b0: ('formation offset helper', 'yes', 'aa-movement::aircraft::formation_offset', ['phase8-movement/native-aircraft.json']),
    0x55ebe0: ('reset formation member paths', 'yes', 'aa-movement::aircraft::reset_member_paths', ['phase8-movement/native-aircraft.json']),
    0x55e1b0: ('clear aircraft path', 'yes', 'aa-movement::aircraft::clear_path', ['phase8-movement/native-aircraft.json']),
    0x5685b0: ('air group leader lookup', 'yes', 'aa-movement::aircraft::group_leader', ['phase8-movement/native-aircraft.json']),
    0x567ff0: ('air group leader test', 'yes', 'aa-movement::aircraft::group_leader_is', ['phase8-movement/native-aircraft.json']),
    0x516b10: ('following-path predicate', 'yes', 'aa-movement::aircraft::following_path', ['phase8-movement/native-aircraft.json']),
    0x55f230: ('aircraft leg builder (planner)', 'no', 'callback of aa-movement::aircraft', []),
    0x560a70: ('aircraft follow planner', 'no', 'callback of aa-movement::aircraft', []),
    0x55edc0: ('air formation teardown', 'no', 'callback of aa-movement::aircraft', []),
    0x55e8e0: ('Aircraft component constructor', 'no', 'callback of aa-movement::aircraft', []),
    0x55ea70: ('Aircraft component destructor', 'no', 'callback of aa-movement::aircraft', []),
    0x554350: ('actor query context constructor', 'yes', 'aa-spatial::actor_context::prepare', ['phase7-spatial-context', 'phase8-movement/native-overlaps.json']),
    # SAI route cache (helper session, native_route_cache.py).
    0x53e610: ('region strategy early test (flag-1 cache lookup)', 'yes', 'aa-navigation::route_cache::search_cached', ['phase8-movement/native-route-cache.json']),
    0x53f230: ('region strategy mark (flag-1 cache record)', 'yes', 'aa-navigation::route_cache::search_cached', ['phase8-movement/native-route-cache.json']),
    0x53f120: ('route cache record', 'yes', 'aa-navigation::route_cache (record)', ['phase8-movement/native-route-cache.json']),
    0x53eac0: ('route cache hop byte', 'yes', 'aa-navigation::route_cache (hop)', ['phase8-movement/native-route-cache.json']),
    0x53ebd0: ('region strategy finish (flag 0 / flag 1)', 'yes', 'aa-navigation::region_search / route_cache', ['phase8-movement/native-route-cache.json']),
    0x53e920: ('route cache path rebuild', 'yes', 'aa-navigation::route_cache (rebuild)', ['phase8-movement/native-route-cache.json']),
    0x7b90c0: ('region adjacency slot of a neighbour', 'yes', 'aa-navigation::route_cache', ['phase8-movement/native-route-cache.json']),
    0x53e5b0: ('region path cost sum', 'yes', 'aa-navigation::route_cache::path_cost', ['phase8-movement/native-route-cache.json']),
    0x53f0a0: ('route cache build helper', 'yes', 'aa-navigation::route_cache::RouteCache::build', ['phase8-movement/native-route-cache.json']),
    0x53ed50: ('route cache rows constructor', 'yes', 'aa-navigation::route_cache::RouteCache::build', ['phase8-movement/native-route-cache.json']),
    0x7abc30: ('SAI manager route cache create', 'yes', 'aa-navigation::route_cache::RouteCache::build', ['phase8-movement/native-route-cache.json']),
    0x5d8560: ('ResetPathPlanner command', 'yes', 'aa-navigation::route_cache::reset_path_planner', ['phase8-movement/native-route-cache.json']),
    0x53e680: ('region list push front', 'yes', 'aa-navigation::route_cache (rebuild)', ['phase8-movement/native-route-cache.json']),
    0x53e6d0: ('region list push back', 'yes', 'aa-navigation::route_cache (rebuild)', ['phase8-movement/native-route-cache.json']),
    0x53e750: ('region list insert after', 'yes', 'aa-navigation::route_cache (rebuild)', ['phase8-movement/native-route-cache.json']),
    # GWorld placement and UAI collision (helper sessions).
    0x4f3c40: ('GWorld placement test', 'yes', 'aa-spatial::gworld_placement::placement_result', ['phase8-movement/native-gworld-placement.json']),
    0x4f22c0: ('GWorld structure crossing test', 'yes', 'aa-spatial::gworld_placement::structure_crossing', ['phase8-movement/native-gworld-placement.json']),
    0x54ecf0: ('tile region word', 'yes', 'aa-spatial::gworld_placement::tile_region', ['phase8-movement/native-gworld-placement.json']),
    0x53ce40: ('path endpoint acceptable (0x53D340 leaf)', 'yes', 'aa-spatial::gworld_placement::endpoint_acceptable', ['phase8-movement/native-gworld-placement-callers.json']),
    0x4f46e0: ('GWorld nearest placeable position', 'yes', 'aa-spatial::gworld_placement::nearest_placement', ['phase8-movement/native-gworld-placement-callers.json']),
    0x534a40: ('UAIStateMove ResultAvoidStatic', 'yes', 'aa-navigation::uai_collision::avoid_static', ['phase8-movement/native-uai-collision.json']),
    0x533140: ('UAI avoid option attempt', 'yes', 'aa-navigation::uai_collision (option)', ['phase8-movement/native-uai-collision.json']),
    0x52e210: ('UAI avoid option distance', 'yes', 'aa-navigation::uai_collision (option)', ['phase8-movement/native-uai-collision.json']),
    0x46dab0: ('descriptor alignment cache', 'yes', 'aa-navigation::uai_collision (alignment)', ['phase8-movement/native-uai-collision.json']),
    0x46dc50: ('descriptor extent selection', 'yes', 'aa-navigation::uai_collision (alignment)', ['phase8-movement/native-uai-collision.json']),
    0x46dcb0: ('descriptor extent selection', 'yes', 'aa-navigation::uai_collision (alignment)', ['phase8-movement/native-uai-collision.json']),
    0x46e4f0: ('closest point on a placed shape', 'yes', 'aa-navigation::uai_collision::closest_point', ['phase8-movement/native-uai-collision.json']),
    0x46e150: ('closest point helper', 'yes', 'aa-navigation::uai_collision::closest_point', ['phase8-movement/native-uai-collision.json']),
    0x51d510: ('Manhattan distance < 0.0002', 'yes', 'aa-navigation::uai_collision', ['phase8-movement/native-uai-collision.json']),
    0x4cefc0: ('actor is its organization leader', 'yes', 'aa-navigation::uai_collision', ['phase8-movement/native-uai-collision.json']),
    0x536ad0: ('UAI collision with a thing at a distance', 'yes', 'aa-navigation::uai_collision::avoid_thing', ['phase8-movement/native-uai-collision.json']),
    0x46eb30: ('swept rectangle shape', 'yes', 'aa-navigation::uai_collision::swept_shape', ['phase8-movement/native-uai-collision.json']),
    0x46e0e0: ('shape constructor (width, length)', 'yes', 'aa-navigation::uai_collision::swept_shape', ['phase8-movement/native-uai-collision.json']),
    0x46eb60: ('square shape', 'yes', 'aa-navigation::uai_collision::square_shape', ['phase8-movement/native-uai-collision.json']),
    0x4cf210: ('actor placed-shape descriptor', 'yes', 'aa-navigation::uai_collision::actor_descriptor', ['phase8-movement/native-uai-collision.json']),
    0x4ea0f0: ('actor soft-shape descriptor', 'yes', 'aa-navigation::uai_collision::soft_descriptor', ['phase8-movement/native-uai-collision.json']),
    0x52f3e0: ('tile placed-shape descriptor', 'yes', 'aa-navigation::uai_collision::tile_descriptor', ['phase8-movement/native-uai-collision.json']),
    # Water-area flood fill (helper session, native_water_areas.py).
    0x539e30: ('KKC_FloodFillWaterAreaGoal construct + solve', 'yes', 'aa-spatial::water_areas::flood_fill_water_area', ['phase8-movement/native-water-areas.json']),
    0x53c570: ('solver goal base constructor (vtable 0xA365C8)', 'yes', 'aa-spatial::water_areas::flood_fill_water_area', ['phase8-movement/native-water-areas.json']),
    0x4f4050: ('generic flood solver core (busy byte 0xB3D43D)', 'yes', 'aa-spatial::water_areas::flood_fill_water_area', ['phase8-movement/native-water-areas.json']),
    0x539d60: ('flood solver constructor (vtable 0xA36048)', 'yes', 'aa-spatial::water_areas::FloodSolver', ['phase8-movement/native-water-areas.json']),
    0x53f8d0: ('flood solver visit neighbour', 'yes', 'aa-spatial::water_areas::flood_fill_water_area', ['phase8-movement/native-water-areas.json']),
    0x53f890: ('flood solver expand (paints the popped tile)', 'yes', 'aa-spatial::water_areas::flood_fill_water_area', ['phase8-movement/native-water-areas.json']),
    0x4f0550: ('flood solver reach', 'yes', 'aa-spatial::water_areas::flood_fill_water_area', ['phase8-movement/native-water-areas.json']),
    0x4f0500: ('flood solver queue exhausted', 'yes', 'aa-spatial::water_areas::flood_fill_water_area', ['phase8-movement/native-water-areas.json']),
    0x539ac0: ('flood FIFO reset (vtable 0xA3602C)', 'yes', 'aa-spatial::water_areas (queue)', ['phase8-movement/native-water-areas.json']),
    0x539db0: ('flood FIFO push (tile +0x24 link)', 'yes', 'aa-spatial::water_areas (queue)', ['phase8-movement/native-water-areas.json']),
    0x539d20: ('flood FIFO push helper', 'yes', 'aa-spatial::water_areas (queue)', ['phase8-movement/native-water-areas.json']),
    0x53f5d0: ('flood FIFO pop', 'yes', 'aa-spatial::water_areas (queue)', ['phase8-movement/native-water-areas.json']),
    0x539df0: ('flood FIFO pointer to cell', 'yes', 'aa-spatial::water_areas (queue)', ['phase8-movement/native-water-areas.json']),
    0x554ed0: ('tile neighbour iterator (direction table 0xB322E0)', 'yes', 'aa-spatial::water_areas (neighbours)', ['phase8-movement/native-water-areas.json']),
    0x4f2000: ('neighbour iterator first', 'yes', 'aa-spatial::water_areas (neighbours)', ['phase8-movement/native-water-areas.json']),
    0x4f2760: ('neighbour iterator next', 'yes', 'aa-spatial::water_areas (neighbours)', ['phase8-movement/native-water-areas.json']),
    0x4f14a0: ('neighbour validity (map bounds, IsIndexBlocked)', 'yes', 'aa-spatial::water_areas (neighbours)', ['phase8-movement/native-water-areas.json']),
    0x539b10: ('flood reach (stamp, arrival direction)', 'yes', 'aa-spatial::water_areas (reach)', ['phase8-movement/native-water-areas.json']),
    0x53c630: ('solver reach helper', 'yes', 'aa-spatial::water_areas (reach)', ['phase8-movement/native-water-areas.json']),
    0x53c430: ('tile visited test', 'yes', 'aa-spatial::water_areas (reach)', ['phase8-movement/native-water-areas.json']),
    0x539b70: ('flood base height', 'yes', 'aa-spatial::water_areas::flood_fill_water_area', ['phase8-movement/native-water-areas.json']),
    0x53c410: ('solver start (advance visit stamp)', 'yes', 'aa-spatial::water_areas::advance_visit_stamp', ['phase8-movement/native-water-areas.json']),
    0x552440: ('GMap visit stamp advance (restart at 0x7FFFFFFF)', 'yes', 'aa-spatial::water_areas::advance_visit_stamp', ['phase8-movement/native-water-areas.json']),
    0x539c40: ('FloodFillWaterArea IsIndexBlocked', 'yes', 'aa-spatial::water_areas (is_blocked)', ['phase8-movement/native-water-areas.json']),
    0x539c00: ('flood paint tile', 'yes', 'aa-spatial::water_areas (paint)', ['phase8-movement/native-water-areas.json']),
    0x552150: ('remove tile from water area', 'yes', 'aa-spatial::water_areas::remove_tile_from_area', ['phase8-movement/native-water-areas.json']),
    0x552110: ('add tile to water area', 'yes', 'aa-spatial::water_areas::add_tile_to_area', ['phase8-movement/native-water-areas.json']),
    0x550f20: ('heightfield vertex area add', 'yes', 'aa-spatial::water_areas (vertex areas)', ['phase8-movement/native-water-areas.json']),
    0x5507c0: ('heightfield vertex area remove/rebuild', 'yes', 'aa-spatial::water_areas (vertex areas)', ['phase8-movement/native-water-areas.json']),
    0x54edc0: ('tile centre (binary32)', 'yes', 'aa-spatial::water_areas (cell_centre)', ['phase8-movement/native-water-areas.json']),
    0x552c90: ('first free water area id (1..=254)', 'yes', 'aa-spatial::water_areas::free_area_id', ['phase8-movement/native-water-areas.json']),
    0x42a530: ('tile at point', 'yes', 'aa-spatial::water_areas::tile_at_point', ['phase8-movement/native-water-areas.json']),
    0x5559f0: ('water stroke + flood', 'yes', 'aa-spatial::water_areas::stroke_and_flood', ['phase8-movement/native-water-areas.json']),
    0x555c10: ('KKC_GMap::PaintTerrain', 'yes', 'aa-spatial::water_areas::paint_terrain', ['phase8-movement/native-water-areas.json']),
    0x5526c0: ('brush stroke stamp', 'no', 'callback StrokeWorld of aa-spatial::water_areas', []),
    0x559fb0: ('terrain change batch commit', 'no', 'callback StrokeWorld of aa-spatial::water_areas', []),
    0x5528f0: ('brush margin stage', 'no', 'callback PaintTerrainStages of aa-spatial::water_areas', []),
    0x551140: ('water surface paint stage', 'no', 'callback PaintTerrainStages of aa-spatial::water_areas', []),
    0x551b40: ('water bed height stage', 'no', 'callback PaintTerrainStages of aa-spatial::water_areas', []),
    0x5972f0: ('random-map water flood caller', 'no', 'caller of aa-spatial::water_areas (input contract only)', []),
    0x597530: ('random-map water area caller', 'no', '', []),
    # Earlier phases (manual confirmation from crate docs and research READMEs).
    0x5773c0: ('Mover begin ground motion', 'yes', 'aa-movement::begin_ground_motion', ['phase4-movement']),
    0x5767c0: ('Mover stop', 'yes', 'aa-movement::stop_ground_motion', ['phase4-movement']),
    0x5774a0: ('Mover ground step', 'yes', 'aa-movement::ground_step', ['phase4-movement']),
    0x576ad0: ('Mover set target/range', 'yes', 'aa-movement::target::set_target', ['phase7-ability-motion']),
    0x576920: ('Mover constructor', 'yes', 'aa-actors::ground_states::MoverState::new', []),
    0x445400: ('signed turn angle (asin/cross)', 'yes', 'aa-trig::signed_angle', ['phase4-trig', 'phase4-trig-angle']),
    0x4454a0: ('rotate counterclockwise by table trig', 'yes', 'aa-trig::rotate', ['phase4-trig']),
    0x4454e0: ('rotate clockwise by table trig', 'yes', 'aa-trig::rotate', ['phase4-trig']),
    0x5300a0: ('UAI ActionGo', 'yes', 'aa-navigation::action_go', ['phase5-navigation']),
    0x52df60: ('UAI activate move', 'yes', 'aa-navigation::activate_move', ['phase5-navigation']),
    0x532170: ('UAI EventMovePath', 'yes', 'aa-navigation::move_path_event', ['phase5-navigation']),
    0x532310: ('UAI EventMoveFormationPath', 'yes', 'aa-navigation::move_path_event', ['phase5-navigation']),
    0x53d340: ('path endpoint adjustment', 'yes', 'aa-navigation::endpoint::adjust', ['phase6-navigation']),
    0x53d500: ('path materialization', 'yes', 'aa-navigation::materialize', ['phase6-navigation']),
    0x53cf40: ('same-cell path finalizer', 'yes', 'aa-navigation::materialize', ['phase6-navigation']),
    0x53e1f0: ('KKC_PathSolver::Solve', 'yes', 'aa-navigation::solver::solve', ['phase6-navigation']),
    0x5336b0: ('UAIStateMove::ResultPlan', 'yes', 'aa-navigation::result_plan', ['phase6-navigation']),
    0x53dc00: ('region path prepass', 'yes', 'aa-navigation::region::prepass', ['phase6-region']),
    0x53f320: ('region graph search entry', 'yes', 'aa-navigation::region_search', ['phase6-region', 'phase7-navigation']),
    0x53ede0: ('region graph search core', 'yes', 'aa-navigation::region_search', ['phase6-region']),
    0x53cb70: ('wall/terrain-change path cost', 'yes', 'aa-navigation::cost', ['phase6-pathcost']),
    0x531520: ('UAIStateMove low-pathing continuation', 'yes', 'aa-navigation::low_path::continue_current', ['phase8-movement/native-low-path-continuation.json', 'phase8-movement/native-low-path-result-plan.json']),
    0x53c170: ('KKC_LowPathSolver::Solve', 'yes', 'aa-navigation::low_path::solve', ['phase8-movement/native-low-path-solve.json']),
    0x53bb90: ('LowPathSolver constructor', 'yes', 'aa-navigation::low_path::LowPathSolver::new', ['phase8-movement/native-low-path-solve.json']),
    0x53a490: ('low-path window constructor (64x64 sub-cells)', 'yes', 'aa-navigation::low_path::Window::new', ['phase8-movement/native-low-path-solve.json']),
    0x53be30: ('low-path core loop', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53b0c0: ('low-path solver expand candidate', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x7c7840: ('low-path solver examine (always 3)', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53a790: ('low-path solver queue exhausted', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53bc80: ('low-path neighbour iterator', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53bb40: ('low-path neighbour iterator next', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53b3c0: ('low-path neighbour validity', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53a660: ('low-path limit (depth 32 / ExaminedMax)', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53b1f0: ('low-path window placement', 'yes', 'aa-navigation::low_path::Window::place', ['phase8-movement/native-low-path-solve.json']),
    0x53aac0: ('low-path sub-cell centre', 'yes', 'aa-navigation::low_path::Window::center', ['phase8-movement/native-low-path-solve.json']),
    0x53a570: ('low-path world to sub-cell', 'yes', 'aa-navigation::low_path::Window::world_to_cell', ['phase8-movement/native-low-path-solve.json']),
    0x539e90: ('low-path window bounds', 'yes', 'aa-navigation::low_path::Window::contains', ['phase8-movement/native-low-path-solve.json']),
    0x53a6f0: ('low-path reachable (bounds + 0x4F3C40)', 'yes', 'aa-navigation::low_path (goal.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53a990: ('low-path goal clipping', 'yes', 'aa-navigation::low_path::Window::clip', ['phase8-movement/native-low-path-solve.json']),
    0x53a820: ('low-path goal clipping helper', 'yes', 'aa-navigation::low_path::Window::clip', ['phase8-movement/native-low-path-solve.json']),
    0x53a3a0: ('low-path goal clipping helper', 'yes', 'aa-navigation::low_path::Window::clip', ['phase8-movement/native-low-path-solve.json']),
    0x53adb0: ('low-path octile heuristic', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53ad00: ('low-path accumulated cost', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53af10: ('low-path cell position', 'yes', 'aa-navigation::low_path (goal.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53af70: ('low-path neighbour line test', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x53b720: ('LowPathGoal OnSolveStarted', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x53b490: ('low-path goal adjust', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x53ab90: ('low-path goal displacement past an actor', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x53a930: ('low-path displacement helper', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x43aa00: ('displacement helper', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x4325a0: ('shape radius select', 'yes', 'aa-navigation::low_path::ActorShape::radius_bits', ['phase8-movement/native-low-path-solve.json']),
    0x53b960: ('LowPathGoal OnSolved path building', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x53ae60: ('low-path direct line', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x53a610: ('LowPathGoal Mark', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x539ee0: ('LowPathGoal IsVisited', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x539f10: ('LowPathGoal IsBlocked (0x54F370)', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x539f00: ('LowPathGoal OnFailed', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x53a5d0: ('LowPathGoal IsGoal', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x53a5f0: ('LowPathGoal GetStart', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x53a5c0: ('LowPathGoal direct-line flag', 'yes', 'aa-navigation::low_path::Goal', ['phase8-movement/native-low-path-solve.json']),
    0x539f60: ('LowPathStats update', 'yes', 'aa-navigation::low_path::LowPathStats::record', ['phase8-movement/native-low-path-solve.json']),
    0x5522b0: ('descriptor facing store', 'yes', 'aa-navigation::low_path (search.rs)', ['phase8-movement/native-low-path-solve.json']),
    0x52cf40: ('path pop kind-0 prefix', 'yes', 'aa-navigation::low_path::pop_front_kind0', ['phase8-movement/native-low-path-solve.json']),
    0x577090: ('Mover catch-up speed (SVar+48 x max speed)', 'yes', 'aa-navigation::action_go (inline)', ['phase5-navigation']),
}

def load_functions():
    funcs = {}
    for line in open(NATIVE / 'functions.tsv', encoding='utf-8'):
        p = line.rstrip('\n').split('\t')
        if p[0] == 'address':
            continue
        funcs[int(p[0], 16)] = int(p[3]) if p[3].isdigit() else 0
    return funcs

def load_calls():
    callers = collections.defaultdict(set)
    callees = collections.defaultdict(set)
    for line in open(NATIVE / 'calls.tsv', encoding='utf-8'):
        p = line.rstrip('\n').split('\t')
        if p[0] == 'caller' or p[1].startswith('EXTERNAL'):
            continue
        a, b = int(p[0], 16), int(p[1], 16)
        callers[b].add(a)
        callees[a].add(b)
    return callers, callees

def load_strings():
    refs = collections.defaultdict(list)
    for line in open(NATIVE / 'string-references.tsv', encoding='utf-8'):
        p = line.rstrip('\n').split('\t')
        if p[0] == 'stringAddress':
            continue
        try:
            refs[int(p[2], 16)].append(p[4][:80])
        except ValueError:
            pass
    return refs

def vtable_refs(funcs):
    data = (ROOT.parent / 'AA.exe').read_bytes()
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    n = struct.unpack_from('<H', data, pe + 6)[0]
    op = pe + 24
    base = struct.unpack_from('<I', data, op + 28)[0]
    sec = op + struct.unpack_from('<H', data, pe + 20)[0]
    out = collections.defaultdict(list)
    for i in range(n):
        name = data[sec + i * 40:sec + i * 40 + 8].rstrip(b'\0')
        vs, va, raw, off = struct.unpack_from('<IIII', data, sec + i * 40 + 8)
        if name != b'.rdata':
            continue
        for o in range(0, raw - 3, 4):
            v = struct.unpack_from('<I', data, off + o)[0]
            if v in funcs and funcs[v] > 0:
                out[v].append(base + va + o)
    return out

def citations():
    pat = re.compile(r'(?<![0-9A-Fa-f])(?:0x)?(?:00)?([4-9a-fA-F][0-9a-fA-F]{5})(?![0-9A-Fa-f])')
    cites = collections.defaultdict(list)
    for dp, dn, fn in os.walk(ROOT / 'crates'):
        for f in fn:
            if not f.endswith('.rs'):
                continue
            path = Path(dp) / f
            rel = path.relative_to(ROOT).as_posix()
            for number, line in enumerate(path.read_text(encoding='utf-8', errors='ignore').splitlines(), 1):
                for m in pat.finditer(line):
                    cites[int(m.group(1), 16)].append((rel, number, line.strip()[:160]))
    return cites

def executed(funcs):
    """research dir -> set of hex numbers appearing in its JSON files."""
    pat = re.compile(rb'(?<![0-9a-z])(?:0x)?(?:00)?([4-9a-f][0-9a-f]{5})(?![0-9a-z])')
    found = collections.defaultdict(set)
    for dp, dn, fn in os.walk(ROOT / 'research'):
        for f in fn:
            if not f.endswith('.json') or f == 'inventory.json':
                continue
            path = Path(dp) / f
            rel = path.relative_to(ROOT / 'research').parts
            key = rel[0] if rel[0] != 'phase8-movement' else 'phase8-movement/' + rel[-1]
            for m in pat.finditer(path.read_bytes().lower()):
                found[key].add(int(m.group(1), 16))
    return found

BOUNDARY = re.compile(r'boundar|required|service|seam|unported|not ported|unsupported|remain|intercept|callback|caller', re.I)

def main():
    funcs = load_functions()
    callers, callees = load_calls()
    strings = load_strings()
    vt = vtable_refs(funcs)
    cites = citations()
    ran = executed(funcs)
    ran_sorted = {k: sorted(v) for k, v in ran.items()}
    import bisect
    selected = {}
    for lo, hi, name in CLUSTERS:
        for a, size in funcs.items():
            if lo <= a < hi:
                selected[a] = name
    for a in list(SINGLES) + list(MANUAL):
        if a in funcs and a not in selected:
            selected[a] = 'helper outside clusters'
    rows = []
    for a in sorted(selected):
        size = funcs[a]
        role = None
        manual = MANUAL.get(a)
        names = [s for s in strings.get(a, []) if '::' in s and s.startswith('KKC')]
        if manual:
            role = manual[0]
        elif a in SINGLES:
            role = SINGLES[a]
        elif names:
            role = names[0]
        crate_cites = cites.get(a, [])
        crates = sorted({c[0].split('/')[1] for c in crate_cites})
        if manual:
            ported = manual[1]
            rust = manual[2]
        elif crate_cites:
            implemented = [c for c in crate_cites if not BOUNDARY.search(c[2]) and '/tests/' not in c[0]]
            ported = 'cited' if implemented else 'boundary'
            rust = '; '.join(sorted({f'{c[0]}:{c[1]}' for c in (implemented or crate_cites)})[:3])
        else:
            ported, rust = 'no', ''
        proven = []
        body = range(a, a + max(size, 1))
        for key, values in ran_sorted.items():
            i = bisect.bisect_left(values, a)
            hits = 0
            while i < len(values) and values[i] < a + max(size, 1):
                hits += 1
                i += 1
            if hits >= 2:
                proven.append(key)
        if manual:
            proven = sorted(set(manual[3]) | {p for p in proven if p.startswith('phase8')})
        if role is None:
            evidence = [c[2] for c in crate_cites[:1]]
            hints = strings.get(a, [])[:2]
            role = 'unclassified' + (f'; strings: {hints}' if hints else '') + (f'; cited: {evidence[0]}' if evidence else '')
        rows.append({
            'address': f'0x{a:06x}', 'size': size, 'cluster': selected[a], 'role': role,
            'ported': ported, 'rust': rust, 'crates_citing': crates, 'proven_by': proven,
            'vtable_slots': [f'0x{v:06x}' for v in vt.get(a, [])[:4]],
            'callers': [f'0x{c:06x}' for c in sorted(callers.get(a, ()))[:8]],
            'callees': [f'0x{c:06x}' for c in sorted(callees.get(a, ()))[:12]],
        })
    summary = collections.Counter((r['cluster'], r['ported']) for r in rows)
    by_cluster = collections.defaultdict(dict)
    for (cluster, status), count in summary.items():
        by_cluster[cluster][status] = count
    doc = {'schema': 1, 'description': __doc__, 'function_count': len(rows),
           'status_counts': dict(collections.Counter(r['ported'] for r in rows)),
           'proven_count': sum(1 for r in rows if r['proven_by']),
           'by_cluster': by_cluster, 'functions': rows}
    (HERE / 'inventory.json').write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps({k: doc[k] for k in ('function_count', 'status_counts', 'proven_count')}))
    for cluster, counts in by_cluster.items():
        print(cluster, counts)

if __name__ == '__main__':
    main()
