"""Build research/phase8-zones/inventory.json: the original zone, detection and
fog-of-war code of AA.exe with role, port status and oracle evidence.

Status values:
  proven      ported in crates/aa-zones and executed (>= 2 instructions of its
              body) by a fixture that a Rust test replays
  ported      ported, not executed by any fixture (see notes)
  seam        a named world service in aa-zones (owned elsewhere or later)
  partial     part of the body ported, the rest a seam (notes)
  not-ported  identified and documented only (notes give the blocker)
Sizes come from functions.tsv, else from a capstone scan to the int3 padding
(vtable-only functions Ghidra did not export). Read-only inputs.
"""
import json, re, struct
from pathlib import Path
import tools

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
FIXTURES = ['native-zone-sets.json', 'native-zone-queries.json', 'native-zone-tiles.json',
            'native-zone-moves.json', 'native-zone-flood.json', 'native-detection.json',
            'native-detection-component.json', 'native-zone-enter.json', 'native-kingdom-masks.json']

S = 'Sim/Zones.cpp KKC_Zone + zone sets'
V = 'Solver/ZoneSolver.cpp'
M = 'Sim/Map tile zone/fog words'
G = 'Sim/GObject.cpp actor detection'
D = 'detection component (actor+0xB8)'
C = 'zone creators / movers (other components)'
T = 'triggers / cheats / SAI hooks'

# address: (cluster, role, status, rust, notes)
E = {
    # --- zone object
    0x516640: (S, 'KKC_Zone constructor (actor, flags=1<<type); state +0x16', 'proven', 'zone::Zone::new', ''),
    0x514c80: (S, 'KKC_Zone destructor (vtable 0xA3229C slot 0)', 'not-ported', '', 'Rust ownership; calls 4F1640'),
    0x514920: (S, 'KKC_Zone +4: state bit0', 'proven', 'zone::Zone::state_bit0', ''),
    0x514930: (S, 'KKC_Zone +8: state bit1 (fast circle builder eligible)', 'proven', 'zone::Zone::fast_eligible', ''),
    0x514cb0: (S, 'KKC_Zone +0xC: passes(blocker) (ZOC blocked by non-zero tile blocker)', 'ported', 'zone::Zone::passes', 'reached only by the flood solver 53F9F0'),
    0x515ba0: (S, 'KKC_Zone +0x10: zone removed from tile (hide/unoccupy)', 'proven', 'zone::on_tile_removed', ''),
    0x515ad0: (S, 'KKC_Zone +0x14: zone added to tile (actor effects, reveal, occupy)', 'proven', 'zone::on_tile_added', ''),
    0x514e40: (S, 'KKC_Zone +0x18: effect on an enemy actor (ZOG guard / ZOC components)', 'proven', 'zone::actor_effect', ''),
    0x5147e0: (S, 'zone base destructor (vtable 0xA322B8)', 'not-ported', '', 'memory only'),
    0x514910: (S, 'zone base vtable store', 'not-ported', '', 'memory only'),
    0x515940: (S, 'KKC_Zone set state bit', 'ported', 'zone::Zone::set_state_bit', 'byte shift; callers 566DB0/568040'),
    0x514cd0: (S, 'zone flags kept on a dead actor (flags & 0x42)', 'proven', 'zone::kept_on_dead_actor', ''),
    0x515970: (S, 'KKC_Zone::Move (tile x, y, radius)', 'proven', 'zone::move_to_tile', ''),
    0x515f40: (S, 'KKC_Zone move by world position (552640 clamp, _ftol, sar)', 'proven', 'zone::move_to_position', ''),
    0x514ce0: (S, 'KKC_Zone release', 'proven', 'zone::release', ''),
    0x5147c0: (S, 'zone effects walk when active (540690 on GWorld+0x244); callers 4F9970, 57C8E0, 57CBA0', 'proven', 'solver::effects_pass', ''),
    0x514da0: (S, 'zone effect on an actor entering coverage (enemy test, +0x80 redirect)', 'proven', 'zone::enemy_entered', ''),
    0x514e00: (S, 'type-3 zone effect (actor+0x90 component 58CEA0)', 'proven', 'zone::type3_effect', ''),
    0x514e20: (S, 'type-8 zone effect (actor+0xD4 component 564300)', 'proven', 'zone::type8_effect', ''),
    # --- zone sets
    0x516360: (S, 'leaf intern lookup/create (GWorld+0x19C)', 'proven', 'set::ZonePool::leaf', ''),
    0x5162d0: (S, 'leaf constructor', 'proven', 'set::ZonePool::leaf', ''),
    0x516160: (S, 'leaf unlink (destructor body)', 'proven', 'set::ZonePool::decref', ''),
    0x516340: (S, 'leaf deleting destructor', 'proven', 'set::ZonePool::decref', ''),
    0x515fa0: (S, 'leaf table insert (auto-grow)', 'proven', 'set (hash layout not modelled)', 'buckets unobservable'),
    0x514ef0: (S, 'leaf table rehash', 'proven', 'set (not modelled)', 'growth only, no cap'),
    0x515c90: (S, 'leaf table remove', 'proven', 'set::ZonePool::decref', ''),
    0x515c50: (S, 'hash table iterator init', 'not-ported', '', 'used by table destructors'),
    0x515e20: (S, 'hash table destructor', 'not-ported', '', 'memory only'),
    0x515e70: (S, 'hash table destructor', 'not-ported', '', 'memory only'),
    0x5161c0: (S, 'union intern lookup/create (cons)', 'proven', 'set::ZonePool::cons', ''),
    0x515ff0: (S, 'union constructor (refs children, depth, hash sum)', 'proven', 'set::ZonePool::cons', ''),
    0x5160d0: (S, 'union destructor (unlink, release head then tail)', 'proven', 'set::ZonePool::decref', ''),
    0x516270: (S, 'union deleting destructor', 'proven', 'set::ZonePool::decref', ''),
    0x515de0: (S, 'union table find', 'proven', 'set::ZonePool::cons', ''),
    0x515d20: (S, 'union table remove', 'proven', 'set::ZonePool::decref', ''),
    0x514e90: (S, 'union table insert', 'proven', 'set (not modelled)', ''),
    0x514800: (S, 'union table rehash', 'proven', 'set (not modelled)', ''),
    0x5154e0: (S, 'union structural equality (exact pair or swapped 2-element)', 'proven', 'set::ZonePool::cons', 'deep branch 514960 unreachable with unique actors'),
    0x514960: (S, 'list subset test (5154E0 deep branch)', 'ported', 'set::ZonePool::cons (proved unreachable)', 'never executed: needs a duplicated leaf'),
    0x516410: (S, 'leaf +0x48 add', 'proven', 'set::ZonePool::add', ''),
    0x516480: (S, 'union +0x48 add (OR in place or prepend)', 'proven', 'set::ZonePool::add', ''),
    0x516460: (S, 'leaf +0x4C remove (XOR, actor not compared)', 'proven', 'set::ZonePool::remove', ''),
    0x516560: (S, 'union +0x4C remove (XOR; absent actor = native memory corruption)', 'proven', 'set::ZonePool::remove', 'absent actor -> Error::NativeAbsentActor'),
    0x516290: (S, 'rebuild prefix onto a new tail', 'proven', 'set::ZonePool::rebuild', ''),
    0x5161b0: (S, 'leaf +0x50 hash (actor + flags)', 'proven', 'set (not modelled)', ''),
    0x5160c0: (S, 'union +0x50 hash sum', 'proven', 'set (not modelled)', ''),
    0x5160b0: (S, 'union +0x10 length', 'proven', 'set::ZonePool::len', ''),
    0x4f0500: (S, 'leaf +0x10 length 1 (shared return-1 stub)', 'proven', 'set::ZonePool::len', ''),
    0x516190: (S, 'leaf +0x08/+0x2C has actor', 'proven', 'query::has_actor', ''),
    0x969b10: (S, 'union +0x08 (return false; shared stub)', 'proven', 'set::ZonePool::add', ''),
    0x4e5240: (S, 'leaf +0x30/+0x44 identity (shared stub)', 'proven', 'query::contains_leaf/contains_part', ''),
    0x514bf0: (S, 'leaf +0x14 any owned by kingdom (mask, exclude)', 'proven', 'query::any_owned_by', ''),
    0x515420: (S, 'union +0x14', 'proven', 'query::any_owned_by', ''),
    0x514a70: (S, 'leaf +0x18/+0x34 contains(actor, mask)', 'proven', 'query::contains', ''),
    0x515040: (S, 'union +0x18', 'proven', 'query::contains', ''),
    0x515290: (S, 'union +0x34', 'ported', 'query::contains', 'same body as +0x18'),
    0x5149f0: (S, 'leaf +0x1C first zone (mask, exclude actor, exclude actor+4)', 'proven', 'query::first_zone', ''),
    0x5150f0: (S, 'union +0x1C', 'proven', 'query::first_zone', ''),
    0x514aa0: (S, 'leaf +0x20 first enemy zone (AI ZOC/ZOG)', 'proven', 'query::first_enemy_zone', ''),
    0x515150: (S, 'union +0x20', 'proven', 'query::first_enemy_zone', ''),
    0x514b10: (S, 'leaf +0x24 first owned zone', 'proven', 'query::first_owned_zone', ''),
    0x5151a0: (S, 'union +0x24', 'proven', 'query::first_owned_zone', ''),
    0x514b80: (S, 'leaf +0x28 enemy discovery zone with actor+0xB0', 'proven', 'query::enemy_discovery_with_b0', ''),
    0x5151f0: (S, 'union +0x28', 'proven', 'query::enemy_discovery_with_b0', ''),
    0x515240: (S, 'union +0x2C has actor', 'proven', 'query::has_actor', ''),
    0x5152f0: (S, 'union +0x30 leaf identity', 'proven', 'query::contains_leaf', ''),
    0x5150a0: (S, 'union +0x44 part identity', 'proven', 'query::contains_part', ''),
    0x514bb0: (S, 'leaf +0x38 any allied (AI friendly ZOS)', 'proven', 'query::any_allied', ''),
    0x515340: (S, 'union +0x38', 'proven', 'query::any_allied', ''),
    0x515ec0: (S, 'leaf +0x3C collect allied actors', 'proven', 'query::collect_allied', ''),
    0x5153b0: (S, 'union +0x3C (returns list EMPTY)', 'proven', 'query::collect_allied', ''),
    0x5155c0: (S, 'leaf +0x40 detects(kingdom): ZOD owner or RZOD reveal mask', 'proven', 'query::detects', ''),
    0x515490: (S, 'union +0x40', 'proven', 'query::detects', ''),
    0x515620: (S, 'leaf +0x54 actor entered: guard/control effects (514DA0), type 3/8 effects, settlement-marker discovery (type 2: explore size + MarkerRevealPadding, mark seen)', 'proven', 'zone::leaf_actor_entered', 'circle iteration 46E080/432730/554DD0 = seam discovery_tiles'),
    0x514fc0: (S, 'union +0x54 (all leaves)', 'proven', 'zone::actor_entered_tile', ''),
    0x5158a0: (S, 'leaf +0x58 visit (replay detection onto an actor)', 'proven', 'query::visit', ''),
    0x515000: (S, 'union +0x58', 'proven', 'query::visit', ''),
    0x514c30: (S, 'leaf +0x5C notify type (tail call 4F3D90)', 'proven', 'query::notify_type', '4F3D90 is a seam'),
    0x515580: (S, 'union +0x5C', 'proven', 'query::notify_type', ''),
    0x514940: (S, 'zone contained in set (+0x18 with zone actor/flags)', 'proven', 'query::tile_has_zone', ''),
    0x54ef10: (M, 'actor entered tile: gate + set slot +0x54', 'proven', 'zone::actor_entered_tile', ''),
    0x559600: (M, 'supply mask: bit k of allied major kingdoms', 'proven', 'tile::kingdom_masks', ''),
    # --- solver
    0x540860: (V, 'ZoneSolver add/move (fast branch 540020 or flood 5402E0)', 'proven', 'solver::add_zone / flood::add', ''),
    0x540590: (V, 'zone remove (fast branch or flood 4F4050 + functor 0xA368C0)', 'proven', 'solver::remove_zone / flood::remove', ''),
    0x5406d0: (V, 'type-1 reveal-mask change per kingdom (fast 5401A0 or flood functor 0xA36870); this = GWorld+0x244', 'proven', 'solver::kingdom_mask_changed / flood::kingdom_pass', ''),
    0x540690: (V, 'effects walk with functor 0xA36820 (54F7F0 on every zone tile)', 'proven', 'flood::effects_pass', ''),
    0x540020: (V, 'fast builder add/move (two circles)', 'proven', 'solver::fast_move', ''),
    0x5401a0: (V, 'fast builder per-kingdom reveal/hide', 'proven', 'solver::fast_kingdom_pass', ''),
    0x53fe90: (V, 'radius adjust (fmod(r, tile) < tile/4 -> raise)', 'proven', 'solver::adjust_radius', ''),
    0x53f730: (V, 'midpoint circle span rasteriser', 'proven', 'solver::circle_spans', ''),
    0x53fd30: (V, 'span: add zone (new circle minus old)', 'proven', 'solver::span', 'last map column excluded (native)'),
    0x53fde0: (V, 'span: remove zone (old circle minus new)', 'proven', 'solver::span', ''),
    0x53ff00: (V, 'span: per-kingdom reveal / hide', 'proven', 'solver::span', ''),
    0x53ff90: (V, 'span thunk -> 53FD30', 'proven', 'solver::span', ''),
    0x53ffc0: (V, 'span thunk -> 53FDE0', 'proven', 'solver::span', ''),
    0x53fff0: (V, 'span thunk -> 53FF00', 'proven', 'solver::span', ''),
    0x53f650: (V, 'tile centre in old circle', 'proven', 'solver::in_circle', ''),
    0x53f6c0: (V, 'tile centre in new circle', 'proven', 'solver::in_circle', ''),
    0x5402e0: (V, 'ZoneSolver run (Dijkstra over the 53CDF0 heap)', 'proven', 'flood::run_dijkstra', ''),
    0x5402a0: (V, 'ZoneSolver constructor (vtable 0xA36954, queue 53CDF0 at +0x44)', 'ported', 'flood::SolverState::new', 'object layout not modelled'),
    0x53f9f0: (V, 'ZoneSolver visit neighbour (cost, terrain/blocker tests, add/remove/push)', 'proven', 'flood::visit_dijkstra', ''),
    0x53f9b0: (V, 'functor 0xA36910 +0x14 reach (53C630, no +0x24 clear)', 'proven', 'flood::run_dijkstra', ''),
    0x53f970: (V, 'functor 0xA36910 +0x30 cost (FLT_MAX stays)', 'proven', 'flood::visit_dijkstra', ''),
    0x53f640: (V, 'functor 0xA36910 +0x1C -> 53C590 impassable', 'proven', 'flood::blocked', ''),
    0x53f820: (V, 'functor +4 start position (zone x, y)', 'proven', 'flood', ''),
    0x53f5b0: (V, 'functor 0xA36870/0xA368C0/0xA36820 +0x1C: tile lacks the zone', 'proven', 'flood::blocked', ''),
    0x53f600: (V, 'functor 0xA36820 +0x4C: 54F7F0', 'proven', 'flood::fifo_apply', ''),
    0x53f610: (V, 'functor 0xA36870 +0x4C: 54F670 / 54F690', 'proven', 'flood::fifo_apply', ''),
    0x53f880: (V, 'functor 0xA368C0 +0x4C: 54FA80', 'proven', 'flood::fifo_apply', ''),
    0x53f840: (V, 'flood solver constructor at GWorld+0x244 (vtable 0xA36048, FIFO 0xA3602C)', 'ported', 'flood::SolverState::new', 'same class as the aa-spatial water flood solver'),
    0x4f4050: (V, 'generic flood solver run (FIFO)', 'proven', 'flood::walk', 'aa-spatial water_areas ports the water-goal instance'),
    0x53c570: (V, 'goal/functor base constructor (vtable 0xA365C8)', 'proven', 'flood (functor flags)', ''),
    0x53c5c0: (V, 'step cost (sqrt2 diagonal) * tile + entry cost', 'proven', 'flood::visit_dijkstra', ''),
    0x53c630: (V, 'reach: stamp + arrival direction', 'proven', 'flood (Visits)', 'stamps solve-local'),
    0x53c590: (V, 'impassable test (tile +0x20 bit 0; functor +4 path 54F370 unused here)', 'proven', 'flood::blocked', ''),
    0x53c410: (V, 'functor +8: advance visit stamp', 'proven', 'flood::SolverState::advance', ''),
    0x53c430: (V, 'functor +0x18: visited this solve', 'proven', 'flood (Visits)', ''),
    0x552440: (V, 'GMap visit stamp advance (clear all at 0x7FFFFFFF)', 'proven', 'flood::SolverState::advance', ''),
    0x554ed0: (V, 'neighbour iterator init (functor flags +0x34/+0x3C/+0x40)', 'proven', 'flood::neighbours', ''),
    0x4f2000: (V, 'neighbour validity (opposite, bounds/impassable, +0x1C, corner test)', 'proven', 'flood::neighbour_valid', ''),
    0x4f2760: (V, 'neighbour iterator advance', 'proven', 'flood::neighbours', ''),
    0x4f14a0: (V, 'neighbour tile pointer (no bounds check)', 'proven', 'flood::linear', 'outside the tile array -> Error::TileOutside'),
    0x53cdf0: (V, 'open list constructor (limit 0x1F9C)', 'ported', 'flood::Queue / SolverState::queue_limit', 'cap removed in production (None); Some(0x1F9C) replays native'),
    0x53d2a0: (V, 'open list push(x, y, p, cost, h): priority f32(cost + h)', 'proven', 'flood::Queue::push', ''),
    0x53d090: (V, 'heap sift-up (ties move up), refuse when full', 'proven', 'flood::Queue::push', ''),
    0x53d150: (V, 'heap pop (left child only when strictly smaller)', 'proven', 'flood::Queue::pop', ''),
    0x53d2f0: (V, 'open list reset', 'proven', 'flood::run_dijkstra', ''),
    0x53ce20: (V, 'open list entry cell', 'proven', 'flood::run_dijkstra', ''),
    0x539ad0: (V, 'solver finish (clears the busy byte)', 'proven', 'flood', ''),
    0x4f0590: (V, 'solver failure (clears the busy byte)', 'ported', 'flood::run_dijkstra', 'start push refused only with a zero limit'),
    0x53f890: (V, 'flood solver +4: action on the popped start tile', 'proven', 'flood::walk', ''),
    0x53f8d0: (V, 'flood solver +0 visit: reach, +0x1C, +0x4C, FIFO push', 'proven', 'flood::walk', ''),
    0x539b10: (V, 'FIFO functor +0x14 reach + clear +0x24', 'proven', 'flood::walk', ''),
    0x539db0: (V, 'FIFO push (start)', 'proven', 'flood::walk', ''),
    0x539d20: (V, 'FIFO push (neighbour)', 'proven', 'flood::walk', ''),
    0x53f5d0: (V, 'FIFO pop through tile +0x24 links', 'proven', 'flood::walk', ''),
    0x539df0: (V, 'FIFO entry cell from tile pointer', 'proven', 'flood::walk', ''),
    0x539ac0: (V, 'FIFO reset', 'proven', 'flood::walk', ''),
    0x45bc70: (V, 'tile +0x1C GTID -> actor (blocker)', 'seam', 'flood::FloodWorld::tile_blocker', 'GTID registry owner'),
    0x855d30: (V, 'assertion: zone on impassable start tile (ZoneSolver.cpp 0x26E)', 'seam', 'flood::FloodWorld::assert_blocked_start', ''),
    # --- map tile words
    0x54ea50: (M, 'fog word: replace explored half', 'ported', 'tile::reset_explored', 'callers 4EBFB0/54F400 not ported'),
    0x54ea60: (M, 'fog word: replace visible half', 'ported', 'tile::reset_visible', ''),
    0x54ea80: (M, 'fog word: set explored', 'proven', 'tile::set_explored', ''),
    0x54eab0: (M, 'fog word: clear explored', 'proven', 'tile::set_explored', ''),
    0x54eae0: (M, 'fog word: set visible', 'proven', 'tile::set_visible', ''),
    0x54eb10: (M, 'fog word: clear visible', 'proven', 'tile::set_visible', ''),
    0x54eb40: (M, 'fog word: explored by a vision-sharing kingdom (+0x228)', 'ported', 'tile::shared_explored', 'used by actor detection 4EBC70'),
    0x54eb70: (M, 'fog word: visible to a vision-sharing kingdom', 'proven', 'tile::shared_visible', ''),
    0x54ec70: (M, 'fog word: visible to kingdom', 'proven', 'tile::visible', ''),
    0x54ecb0: (M, 'fog word: explored by kingdom', 'proven', 'tile::explored', ''),
    0x54ed00: (M, 'thunk -> 54ECB0 (tile+0x14)', 'ported', 'tile::explored', ''),
    0x54eba0: (M, 'shared-vision mask: bits k and k+16 of allied major kingdoms (kingdom +0x138)', 'proven', 'tile::kingdom_masks', ''),
    0x54f3c0: (M, 'fog disabled or tile visible to kingdom (AI bombard)', 'proven', 'tile::visible_or_no_fog', ''),
    0x54f3e0: (M, 'fog disabled or tile visible to a vision-sharing kingdom', 'proven', 'tile::shared_visible_or_no_fog', ''),
    0x54f4f0: (M, 'tile reveal (explore/visible) + SAI + actor +0x64', 'proven', 'tile::reveal', ''),
    0x54f670: (M, 'tile reveal visible only (54F4F0(k,0,1))', 'proven', 'tile::reveal', ''),
    0x54f5e0: (M, 'tile hide when no detecting zone + SAI + actor +0x68', 'proven', 'tile::hide', ''),
    0x54f690: (M, 'thunk -> 54F5E0', 'proven', 'tile::hide', ''),
    0x54f6a0: (M, 'tile supply bit clear unless another own ZOS', 'proven', 'tile::unoccupy', ''),
    0x54ed20: (M, 'tile supply bit set + SAI', 'proven', 'tile::occupy', ''),
    0x54f930: (M, 'fog restored in area: per-tile explored clear + actor +0x6C', 'proven', 'tile::restore_fog', 'trigger 542950 iteration is aa-spatial 554DD0/554330'),
    0x54f7f0: (M, 'zone effects on the tile\'s actors (types 3,4,5,8)', 'proven', 'zone::tile_actor_effects', ''),
    0x54fa10: (M, 'tile add zone (set +0x48, refcounts, zone +0x14)', 'proven', 'solver::tile_add / set::ZonePool::tile_add', ''),
    0x54fa80: (M, 'tile remove zone (set +0x4C, refcounts, zone +0x10)', 'proven', 'solver::tile_remove / set::ZonePool::tile_remove', 'null set -> Error::NativeNullSet'),
    0x54ef40: (M, 'tile has zone (54EF40)', 'proven', 'query::tile_has_zone', ''),
    0x54ef80: (M, 'tile first zone of type (excl actor, excl actor+4)', 'proven', 'query::tile_first_zone', ''),
    0x54efc0: (M, 'tile enemy zone of type (AI: 4 ZOC, 5 ZOG; settlement 5)', 'proven', 'query::tile_enemy_zone', ''),
    0x54eff0: (M, 'tile own zone of type', 'proven', 'query::tile_owned_zone', ''),
    0x54f020: (M, 'tile enemy discovery zone with +0xB0', 'proven', 'query::tile_enemy_discovery', ''),
    0x54f870: (M, 'tile own supply (bit or other own ZOS)', 'proven', 'query::tile_own_supply', ''),
    0x54f8b0: (M, 'tile friendly supply (shared mask or allied ZOS; AI ZOS)', 'proven', 'query::tile_friendly_supply', ''),
    0x54f8f0: (M, 'tile collect allied zone actors', 'proven', 'query::tile_collect_allied', ''),
    0x559530: (M, 'supply word: kingdom bit', 'proven', 'query::supply_bit', ''),
    0x559570: (M, 'supply word: set kingdom bit', 'proven', 'tile::occupy', ''),
    0x5595a0: (M, 'supply word: clear kingdom bit', 'proven', 'tile::unoccupy', ''),
    0x5595d0: (M, 'supply word: & kingdom +0x22C mask', 'proven', 'query::supply_shared', ''),
    0x54f400: (M, 'tile: reset explored to all + actor +0x94 (map-wide reveal 551F90/551FD0)', 'not-ported', '', 'cheat/reveal-all path'),
    0x54f4b0: (M, 'tile: reset visible to all + actor +0x98', 'not-ported', '', 'cheat/reveal-all path'),
    0x551f90: (M, 'all tiles 54F400', 'not-ported', '', ''),
    0x551fd0: (M, 'all tiles 54F400 + 54F4B0', 'not-ported', '', ''),
    0x552640: (M, 'clamp world position into the map', 'proven', 'zone::clamp_position', ''),
    # --- actor detection (GObject)
    0x4ec1b0: (G, 'actor +0x68 hide(k): multi-tile actors stay visible while a shape tile is visible to a sharing kingdom', 'proven', 'detection::hide', 'shape iteration 4EA0F0/554DD0/554330 = seam shape_tiles'),
    0x4ea0c0: (G, 'actor +0x6C unexplore(k): clear actor seen bit', 'proven', 'detection::unexplore', ''),
    0x4ea0d0: (G, 'actor seen by a vision-sharing kingdom (54EB40 on actor+0x54)', 'proven', 'detection::shared_explored', ''),
    0x4ea1d0: (G, 'actor visible to a vision-sharing kingdom or fog off', 'proven', 'detection::shared_visible', ''),
    0x4d3b80: (G, 'actor +0x70 visible to kingdom (carrier, own word, organization / +0x80 target)', 'proven', 'detection::visible_to', 'trigger 99 actor_visible_to_kingdom'),
    0x4d3c50: (G, 'actor +0x74 seen by kingdom', 'proven', 'detection::explored_by', 'trigger 100 actor_explored_to_kingdom'),
    0x4ebc70: (G, 'detection memory refresh: per kingdom create/delete ghost record', 'proven', 'detection::refresh_memory', ''),
    0x4ebb80: (G, 'detection memory clear + SAI 79DEB0', 'seam', 'detection::ActorDetectionWorld::clear_memory', ''),
    0x4ebc30: (G, 'detection memory clear when all empty', 'not-ported', '', ''),
    0x4eb9f0: (G, 'detection memory record constructor (0x38 bytes)', 'not-ported', '', ''),
    0x4eb490: (G, 'detection memory eligibility', 'seam', 'detection::ActorDetectionWorld::memory_eligible', ''),
    0x4ebe50: (G, 'KKC_GObject::Init (tile coords, 4EBC70)', 'not-ported', '', ''),
    0x4ebfb0: (G, 'actor: seen by all (54EA50 0xFFFF) + refresh', 'proven', 'detection::set_all_explored', ''),
    0x4ebfd0: (G, 'actor: visible to all (54EA60 0xFFFF) + refresh', 'proven', 'detection::set_all_visible', ''),
    0x4ec270: (G, 'GObject tile change: clear visible bits, refresh memory, replay zones under the actor (4EA150)', 'proven', 'detection::tile_changed', ''),
    0x4ec290: (G, 'actor: memory refresh on ownership change', 'not-ported', '', ''),
    0x4ec360: (G, 'actor detection copy (replace/upgrade)', 'not-ported', '', ''),
    0x4ec7a0: (G, 'GObject::Load (saved fog word, memory records)', 'not-ported', '', 'save format'),
    0x4ee700: (G, 'multi-tile actor test', 'seam', 'tile::ZoneWorld::actor_multi_tile', ''),
    0x4ea200: (G, 'tile under actor (+0x44/+0x48)', 'seam', 'tile::ZoneWorld::actor_tile', ''),
    0x4e0f90: (G, 'Kingdom: recompute shared vision +0x228 (54EBA0) and supply mask +0x22C (559600)', 'proven', 'tile::kingdom_masks', ''),
    0x4e27d0: (G, 'same for a list', 'not-ported', '', ''),
    0x4ec010: (G, 'actor +0x64 reveal(k, tile explored, visible): actor+0x54 word, container forward, memory refresh, first sighting', 'proven', 'detection::reveal', 'first-sighting tail (432E40/42D490/79EA90/7AD9A0) is the seam ActorDetectionWorld::first_sighting'),
    0x4364c0: (G, 'organization member iterator (org+0x28/+0x2C, nulls skipped)', 'proven', 'detection::ActorDetectionWorld::organization_members', ''),
    0x5733a0: (G, 'carrier visibility (carried actors)', 'seam', 'detection::ActorDetectionWorld::carrier_visible', ''),
    0x432e40: (G, 'GTID lookup of actor+0x50 (first-sighting tail)', 'seam', 'detection::ActorDetectionWorld::first_sighting', ''),
    0x4ea0f0: (G, 'actor shape descriptor for tile iteration', 'seam', 'detection::ActorDetectionWorld::shape_tiles', 'aa-spatial owns 554DD0/554330'),
    0x4ea150: (G, 'replay the zone sets under the actor onto it (slot +0x58 visit per occupied tile)', 'proven', 'detection::refresh_from_tiles', ''),
    0x4cf210: (G, 'actor occupancy shape descriptor', 'seam', 'detection::ActorDetectionWorld::occupancy_tiles', 'aa-spatial iterator'),
    0x4d6350: (G, 'GActor tile change (+0x124): 4EC270, 4D4130, org 579BA0, SAI 7A9D70, move every active zone (515F40), 4CFEA0, +0xCC notifications', 'partial', 'component::move_actor_zones', 'zone loop proven in context; other callees belong to their owners'),
    # --- detection component
    0x56eb40: (D, 'enable/disable a zone index with radius (create 4D69A0, move 515F40, release 514CE0)', 'proven', 'component::enable_zone', ''),
    0x56ec10: (D, 'RZOD (type 1) enabled: reveal mask, ActorData+0x1D0 > 0, actor checks', 'proven', 'component::rzod_enabled', ''),
    0x56edc0: (D, 'reveal sources: owners of engaging actors (+0x118, +0x130, +0xB0+0x24, 569300, +0x70 with SVar)', 'seam', 'component::DetectionComponentWorld::reveal_sources', 'lists owned by combat/AI'),
    0x56efc0: (D, 'RZOD reveal mask recompute (sources, flagged org members, enemy kingdoms with 4CF110 sign bit, minus owner) -> 5406D0', 'proven', 'component::update_reveal_mask', ''),
    0x56f140: (D, 'detection component tick (structure ZOD from +0x1D0; else ZOD, mask, RZOD)', 'proven', 'component::tick', ''),
    0x56ec90: (D, 'component +0x14 activate (structures; scheduled tick when +0x138; ZOD enable/disable)', 'proven', 'component::activate', ''),
    0x56ea30: (D, 'ZOD (type 6) enabled: radius > 0 and actor checks (+0x100 virtual, +0x94/+0x158, +0x90, +0x80)', 'proven', 'component::zod_enabled', ''),
    0x56ea10: (D, 'ZOD radius from ActorData+0x1CC (0x45D790; modifiers 0x501C30 with a context)', 'proven', 'component::set_zod_radius', 'modifier context is a seam'),
    0x56eab0: (D, 'detection component constructor (mask 0, radius 0x45D790(0))', 'ported', 'component::DetectionComponent', 'object construction belongs to the component factory'),
    0x45d790: (D, 'detection range: ActorData+0x1CC raw or through 0x501C30', 'proven', 'component::set_zod_radius', ''),
    0x4cf110: (D, 'kingdom +0x78 +0x1C byte (reveal enemy RZOD flag)', 'proven', 'component::DetectionComponentWorld::kingdom_reveal_byte', ''),
    0x4849f0: (D, 'pointer vector reserve (zone vector growth)', 'proven', 'component::create_zone', ''),
    0x50acc0: (D, 'updatable schedule (tick delay 1e-4)', 'seam', 'component::DetectionComponentWorld::schedule_tick', ''),
    0x4d69a0: (C, 'KKC_GActor::CreateZone (9-slot vector, 516640)', 'proven', 'component::create_zone', 'actor storage owner keeps ActorZones'),
    0x4d7230: (C, 'GActor activation: type-2 discovery zone (radius 4BB000)', 'seam', '', 'caller of 515F40'),
    0x4bb000: (C, 'discovery radius 2*tile + settings[+0x74] + 1e-4', 'not-ported', '', 'settings object [B39F58]'),
    0x566db0: (C, 'component 0xA3BBD4 +8: ZOC (type 4) creation', 'seam', '', ''),
    0x579dd0: (C, 'Organization +0x14: ZOC (type 4)', 'seam', '', ''),
    0x585940: (C, 'component 0xA3EEA0 +8: type 2', 'seam', '', ''),
    0x58b0e0: (C, 'SupplyComponent +8: ZOS (type 0)', 'seam', '', 'economy agent'),
    0x58ae10: (C, 'SupplyComponent tick: ZOS move/release', 'seam', '', 'economy agent'),
    0x568040: (C, 'type 5 ZOG creation', 'seam', '', ''),
    0x563e10: (C, 'type 8 creation', 'seam', '', ''),
    0x58cf50: (C, 'type 3 creation', 'seam', '', ''),
    0x4dcda0: (C, 'type 7 creation', 'seam', '', ''),
    0x50c4c0: (C, 'zone presentation (UI)', 'seam', 'zone::ZoneObjectWorld::presentation', ''),
    # --- triggers, cheats, SAI
    0x542950: (T, 'trigger action fog restored in area (554DD0 iteration + 54F930)', 'partial', 'tile::restore_fog', 'iteration in aa-spatial; aa-triggers seam fog_restore_tiles'),
    0x545ec0: (T, 'trigger element registrations (actor_visible/explored_to_kingdom, fog_restored_in_area)', 'not-ported', '', 'aa-triggers owns'),
    0x4f9970: (T, 'IsFogCheatEnabled / cheat fog', 'not-ported', '', ''),
    0x7a99a0: (T, 'SAI region occupied += 1', 'seam', 'tile::SaiHook::OccupiedInc', 'AI agent'),
    0x7a99d0: (T, 'SAI region occupied -= 1', 'seam', 'tile::SaiHook::OccupiedDec', ''),
    0x7a9a00: (T, 'SAI region explored += 1', 'seam', 'tile::SaiHook::ExploredInc', ''),
    0x7a9a30: (T, 'SAI region visible += 1', 'seam', 'tile::SaiHook::VisibleInc', ''),
    0x7a9a60: (T, 'SAI region visible -= 1 + white-fog timer 7B8E70', 'seam', 'tile::SaiHook::VisibleDec', ''),
    0x7a9a90: (T, 'SAI region explored -= 1', 'seam', 'tile::SaiHook::ExploredDec', ''),
    0x7b8e70: (T, 'white fog timer (ResetWhiteFogTimerPecent)', 'not-ported', '', 'AI agent'),
}


def extent(a):
    s = tools.fsize(a)
    if s:
        return s, 'functions.tsv'
    o = tools.off(a)
    b = tools.B
    i = o
    while i < o + 0x2000:
        if b[i] in (0xc3, 0xc2) and b[i + (1 if b[i] == 0xc3 else 3):i + (1 if b[i] == 0xc3 else 3) + 2] == b'\xcc\xcc':
            return i - o + (1 if b[i] == 0xc3 else 3), 'scan'
        i += 1
    return 0x10, 'scan-fallback'


def main():
    executed = {}
    for name in FIXTURES:
        p = HERE / name
        if not p.exists():
            continue
        d = json.loads(p.read_text())
        executed[name] = sorted(int(x['va'], 16) for x in d.get('instructions', []))
    cites = {}
    for rs in (ROOT / 'crates/aa-zones').rglob('*.rs'):
        for m in re.finditer(r'0x([0-9A-Fa-f]{6})', rs.read_text(encoding='utf-8')):
            cites.setdefault(int(m.group(1), 16), set()).add(str(rs.relative_to(ROOT)).replace('\\', '/'))
    rows = []
    for a in sorted(E):
        cluster, role, status, rust, notes = E[a]
        size, src = extent(a)
        need = 1 if size <= 8 else 2
        ev = [n for n, ins in executed.items() if sum(1 for x in ins if a <= x < a + size) >= need]
        if status == 'ported' and ev and rust:
            status = 'proven'
        if status == 'proven' and not ev:
            status = 'ported'
            notes = (notes + '; ' if notes else '') + 'no fixture executes it'
        rows.append({'address': '0x%06x' % a, 'size': size, 'size_source': src, 'cluster': cluster, 'role': role,
                     'status': status, 'rust': rust, 'proven_by': ev,
                     'cited_in': sorted(cites.get(a, [])), 'notes': notes})
    counts = {}
    for r in rows:
        counts[r['status']] = counts.get(r['status'], 0) + 1
    out = {'schema': 1, 'description': __doc__.strip(), 'function_count': len(rows), 'status_counts': counts,
           'removed_caps': [
               {'what': 'none in the ported code', 'detail': 'zone-set intern tables grow without bound natively (auto-grow rehash 514800/514EF0); Rust uses HashMap/Vec'},
               {'what': 'kingdom bits 16 (0xA11740)', 'kept': True, 'detail': 'data layout of the 32-bit fog word (16 explored + 16 visible), the 16-bit supply word and saved games; a rule of the format, not an engine buffer'},
               {'what': 'zone types 9 (actor+0x148 vector size, 9-bit query scan)', 'kept': True, 'detail': 'gameplay rule (zone type set)'}],
           'functions': rows}
    (HERE / 'inventory.json').write_text(json.dumps(out, indent=1))
    print(len(rows), counts)


if __name__ == '__main__':
    main()
