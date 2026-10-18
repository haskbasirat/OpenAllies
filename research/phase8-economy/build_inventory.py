"""Build research/phase8-economy/inventory.json (read-only over native exports).

Discovery is reproducible and conservative:
  1. curated roles below (read from pseudocode/instructions in this phase);
  2. every caller of the Kingdom stockpile/flow primitives;
  3. every function referencing an economy gameplay string;
  4. GKingdom/economy-updatable vtable slots from the constructor 0x4E4DC0;
  5. direct callees of curated entry points that are not shared utilities.
Port status comes from address tokens in crates/**/*.rs; oracle evidence from
research/** native harnesses/fixtures. "proven" requires both a crate reference
and an oracle reference; seams intercepted by an oracle are listed separately in
the referenced README and are NOT counted as executed here when they are marked
seam in CURATED.
"""
from pathlib import Path
import json, re, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_index import FUNCTIONS, CALLERS, CALLEES, STRINGS  # noqa: E402
from disasm import at, BASE, SECTIONS  # noqa: E402
import address_scan  # noqa: E402

ROOT = HERE.parents[1]

# address: (subsystem, role, note). note "seam" = treated as external callback by
# every current port; "phase8" = ported/proven in this phase.
CURATED = {
    # Kingdom resource arrays and tick (GKingdom economy subobject at +0x180)
    0x4E1690: ('kingdom-economy', 'Debit stockpile vector (no clamp/afford check)', ''),
    0x4E1F60: ('kingdom-economy', 'Upkeep shortage charge add/remove over STOCKPILED rows', ''),
    0x4E20A0: ('kingdom-economy', 'Can afford: STOCKPILED rows stock >= cost', ''),
    0x4E3940: ('kingdom-economy', 'Adjust one provider with shortage rebalance and limited cap', ''),
    0x4E39D0: ('kingdom-economy', 'Credit stockpile vector, clamp [0, EconomyStockpileResourceMax]', ''),
    0x4E48C0: ('kingdom-economy', 'Adjust consumption vector (remove/mutate/reapply per row)', ''),
    0x4E4930: ('kingdom-economy', 'Adjust provider vector (ascending 4E3940 calls)', ''),
    0x4E9C40: ('kingdom-economy', 'Economy updatable Tick: flow, overdraw clamp, upkeep-shortage ratio events 3/4, recruit-single regen, faction XP income, victory checks, stats', 'phase8'),
    0x4E12D0: ('kingdom-economy', 'Recruit-item-single regeneration (one charging item at a time, ActorData+2CC rate)', 'phase8'),
    0x4CDFD0: ('commander-experience', 'Commander (kingdom+0x230) XP add, clamp [0, FactionData+3C], score 44', 'phase8'),
    0x4E0FD0: ('commander-experience', 'Kingdom wrapper: Commander XP add with multiplier *(cache+0x17C)', 'phase8'),
    0x4E37D0: ('statistics', 'Kingdom statistics snapshot timer (60 s accumulator, 58F530 snapshot)', 'phase8'),
    0x58ED00: ('statistics', 'Statistics production totals += gross production; score32 += dt*score52', 'phase8'),
    0x58E870: ('statistics', 'Ordinary statistics score increment (binary32); aa_economy::tick::add_score', 'phase8'),
    0x58F530: ('statistics', 'Statistics snapshot constructor (0x160 bytes)', 'seam'),
    0x45F240: ('statistics', 'Pointer vector push_back (snapshot list +0x338)', 'seam'),
    0x4E9960: ('victory', 'Team victory check on economy tick (world time >= 1.0)', 'seam'),
    0x4E9A40: ('victory', 'Domination/percentage victory check on economy tick', 'seam'),
    0x42D4B0: ('presentation', 'Event/notification dispatch (manager b39d8c)', 'seam'),
    0x42D490: ('presentation', 'Event/notification dispatch variant', 'seam'),
    0x96CC4C: ('crt', '_ftol truncation of x87 st0', 'phase8'),
    0x96D5E0: ('crt', 'CRT floor (x87 path when the SSE2 flag at 0xB4B340 is clear)', 'phase8'),
    0x4E0EC0: ('kingdom-economy', 'Recruit single virtual +8: available = 1 (not a Ghidra function start)', 'phase8'),
    0x4E0EA0: ('kingdom-economy', 'Recruit single virtual +4: take one if available (not a Ghidra function start)', ''),
    0x45D5B0: ('actor-data', 'ActorData recruit-single regeneration rate getter (+2CC or 5014E0)', 'phase8'),
    0x4E5DF0: ('kingdom-economy', 'GKingdom::Init: reset flow vectors (stockpiles kept)', ''),
    0x4E8550: ('kingdom-economy', 'GKingdom::Load: wire decoded by aa-mapworld (phase6/7); economy semantics ported as pure aa-economy functions (statistics load, property record loop 4E8A09)', ''),
    0x4E6DB0: ('kingdom-economy', 'Start-mission kingdom setup: stockpile reset, Commander/profile/start technologies, credits, faction check, Commander starting XP', 'phase8'),
    0x4CDF10: ('commander-experience', 'Commander experience set with [0, max] clamp', 'phase8'),
    0x4E3690: ('statistics', 'Promote loaded statistics (+0x330/+0x344) to live (+0x32C/+0x340)', 'phase8'),
    0x58EA00: ('statistics', 'Reset mission scores 47..55 at world start', 'phase8'),
    0x58EE80: ('statistics', 'Ordinary statistics load (v0 60 words, v1 66 words)', 'phase8'),
    0x58DBE0: ('statistics', 'Relation statistics load (v0 discarded, v1/v2 4/5 groups per family)', 'phase8'),
    0x4F1E10: ('statistics', 'World start economy parts: statistics promotion/reset, actor+0x13C = 0x38 and owner statistics tracking, forced initial snapshots', 'phase8'),
    0x58F010: ('statistics', 'Statistics tracking add (routed to aa-economy by combat; seam here)', 'seam'),
    0x4E2080: ('kingdom-economy', 'Stockpile row add then 4E1630 clamp [0,max]', 'phase8'),
    0x4E1630: ('kingdom-economy', 'Stockpile row store with <=0 -> +0 and max clamp', 'phase8'),
    0x4E16D0: ('kingdom-economy', 'Stockpile row subtract, no clamp', 'phase8'),
    0x473520: ('kingdom-economy', 'Primary resource: first STOCKPILED registry entry (cached registry+0x844)', 'phase8'),
    0x4E24B0: ('special-operations', 'Faction/general ability affordability (Commander XP <= balance, then 4E20A0)', 'phase8'),
    0x4CDD60: ('special-operations', 'Commander XP affordability: FILD def+0x74 <= balance', 'phase8'),
    0x4E2590: ('special-operations', 'Faction ability payment (XP debit, stats, resource debit)', ''),
    0x4CDD90: ('special-operations', 'Faction XP debit used by ability payment', ''),
    0x4BEB50: ('special-operations', 'Ability resource cost vector getter (seam in phase8 oracle)', 'seam'),
    0x4C9550: ('special-operations', 'Ability use path that pays via 4E2590', ''),
    # Kingdom technology / properties
    0x4E3AD0: ('research', 'GKingdom::AddProperty: allowed-list gate, property state add, propagate to kingdom actors, event 5', 'phase8'),
    0x4E1890: ('research', 'Kingdom technology allowed-list membership', ''),
    0x4E2310: ('research', 'Kingdom in-progress kingdom-level research list push (property kind 2)', 'phase8'),
    0x4E2360: ('research', 'Kingdom in-progress research list remove (kind 2)', 'phase8'),
    0x4E2390: ('research', 'Kingdom in-progress research contains (kind 2)', 'phase8'),
    0x4E0FB0: ('research', 'Kingdom has property via 5032D0 (seam)', 'seam'),
    # Research component
    0x584790: ('research', 'ResearchComponent research request: blocked gate, afford, debit, enqueue, kingdom reserve', 'phase8'),
    0x5845F0: ('research', 'Research affordability (copy ResearchCost, 4E20A0, notify research_error_affording)', 'phase8'),
    0x5844A0: ('research', 'Research blocked reason (state 0x23, queued, in-progress, has property, requirements)', 'phase8'),
    0x5842F0: ('research', 'Research requirements (4C5630 requirement graph, 4E1890 allowed list)', 'phase8'),
    0x584030: ('research', 'ResearchComponent completed list contains record', 'phase8'),
    0x588A40: ('research', 'Structure queue contains research record (kind 1)', 'phase8'),
    0x4F1430: ('research', 'Singly linked list remove-first-by-value (used by 4E2360)', 'phase8'),
    0x4CED40: ('research', 'Actor organization getter (seam)', 'seam'),
    0x581280: ('research', 'Organization property container lookup via 5032D0 (seam)', 'seam'),
    0x4CEF10: ('research', 'Apply property to actor/organization (seam)', 'seam'),
    0x42A590: ('research', 'Property WhenResearched trigger on def+0x54 (seam)', 'seam'),
    0x4C5630: ('research', 'Requirement evaluation wrapper over 4C5250', ''),
    0x584120: ('research', 'Research completion: score2, completed list, event 0xB, apply property by kind, release reservation', 'phase8'),
    0x4C3D70: ('research', 'Property ResearchCost vector copy', 'phase8'),
    0x589F00: ('production', 'Structure enqueue research item (alloc, ctor, append, start head)', 'phase8'),
    0x589830: ('production', 'Research queue item constructor (ID, kind 1, property, cost)', 'phase8'),
    # Recruitment / production queue (phase6)
    0x5836E0: ('production', 'Recruit request', ''),
    0x582AE0: ('production', 'Recruit pricing/payment/reservation', ''),
    0x582950: ('production', 'Recruit affordability', ''),
    0x589E00: ('production', 'Recruit queue item create/append', ''),
    0x5895B0: ('production', 'Recruit queue item constructor', ''),
    0x5896E0: ('production', 'Recruit-location queue item constructor', ''),
    0x589420: ('production', 'Start head item', ''),
    0x588D60: ('production', 'Queue item duration', ''),
    0x589F90: ('production', 'Queue process (completion/pause)', ''),
    0x5894C0: ('production', 'Queue item remove', ''),
    0x588CD0: ('production', 'Queue item OnRemovedFromQueue (upkeep removal)', ''),
    0x589BD0: ('production', 'Queue cancel by item ID', ''),
    0x5888F0: ('production', 'Queue item SetCurrentUpkeep', ''),
    0x581D40: ('production', 'Refund wrapper', ''),
    0x581D90: ('production', 'Refund wrapper', ''),
    0x583F30: ('research', 'Research refund: credit paid cost, release kingdom reservation (phase6 seamed 4E2360)', 'phase8'),
    0x582E80: ('production', 'RecruitComponent::OnProcessedByQueue (fulfil)', ''),
    0x5838B0: ('production', 'Recruit deletion from queue path', ''),
    0x583A70: ('production', 'RecruitComponent command dispatch', ''),
    0x58A470: ('production', 'Structure queue item load', ''),
    # Economy component (actor contributions)
    0x56F2B0: ('actor-economy', 'EconomyComponent::Tick (supply state refresh)', 'phase8'),
    0x56F310: ('actor-economy', 'Remove upkeep contribution', ''),
    0x56F340: ('actor-economy', 'Remove provided contribution', ''),
    0x570020: ('actor-economy', 'Refresh actor resource contributions', ''),
    0x56F4D0: ('actor-economy', 'Economy component ownership-change transfer', ''),
    0x56F9A0: ('actor-economy', 'Provider predicate', ''),
    0x56F960: ('actor-economy', 'Supply predicate', ''),
    0x56FA30: ('actor-economy', 'Actor cost/upkeep vector', ''),
    0x56FB10: ('actor-economy', 'Organization upkeep vector', ''),
    0x56FC20: ('actor-economy', 'Production vector (member cache)', ''),
    # Construction / builder / settlement
    0x562FD0: ('construction', 'Builder construct affordability (no owner -> true; construct_error_affording)', 'phase8'),
    0x563160: ('construction', 'BuilderComponent::BuildActor: 6939B0 placement, 562FD0, 4E1910, enemy footprint, site lookup, create, quote+debit after creation', 'phase8'),
    0x566110: ('construction', 'Upgrade gate + affordability (structure/state 0x15, flags bit21/27, def+0x174 bit15, 4E1910, 565770, quote)', 'phase8'),
    0x566360: ('construction', 'Upgrade start: gate, second quote stored at +0x1C and debited, trigger, modifiers, +0x28, state 0x1D', 'phase8'),
    0x5879B0: ('construction', 'Settlement structure affordability', 'phase8'),
    0x587B60: ('construction', 'SettlementComponent::CanBuildStructure', 'phase8'),
    0x587FD0: ('construction', 'Settlement build structure (quote, create, debit reread owner, paid store, register)', 'phase8'),
    0x5656B0: ('construction', 'Construction cancel: unless destroyed credit+zero +0x10 then destroy actor', 'phase8'),
    0x565700: ('construction', 'Upgrade cancel: credit+zero +0x1C, base modifiers, +0x28=0, state 0x1B', 'phase8'),
    0x565770: ('construction', 'ActorData+0x4C0 upgrade list contains definition', 'phase8'),
    0x45F180: ('construction', 'Float vector resize-and-zero (used to clear paid vectors)', 'phase8'),
    0x637820: ('construction', 'Float vector release', 'phase8'),
    0x45F4D0: ('construction', 'Definition cost quote with actor modifier (phase7 port in aa-actors; seam here)', 'seam'),
    0x54BFC0: ('construction', 'Construction-begin trigger (seam)', 'seam'),
    0x561D40: ('construction', 'Apply upgrade modifiers on actor+0x60 (seam)', 'seam'),
    0x4F6760: ('construction', 'World actor destruction (seam)', 'seam'),
    0x5658C0: ('construction', 'Sell refund: salvage 565500, credit, zero paid, event 0x17', 'phase8'),
    0x565B00: ('construction', 'ConstructionComponent::OnConstructionFinished (settlement founding, CreateComponents, kingdom completion, events, triggers; creation and components are world seams)', 'phase8'),
    0x566400: ('construction', 'ConstructionComponent::OnUpgradeFinished (mode-4 replacement actor through world seams; refund 565700 on failure)', 'phase8'),
    0x566AC0: ('construction', 'Construction/upgrade completion dispatch (bit 21 -> 565B00, bit 29 -> 566400, else event 8)', 'phase8'),
    # Denizen / HQ / transport purchases
    0x56D5A0: ('purchases', 'Denizen air attack on target purchase', 'phase8'),
    0x56D7C0: ('purchases', 'Denizen air attack at location purchase', 'phase8'),
    0x56DA00: ('purchases', 'Denizen air recon purchase', 'phase8'),
    0x574A10: ('purchases', 'HQ attach affordability (hq_error_attach_affording)', 'phase8'),
    0x574C10: ('purchases', 'HQ attach payment (debit)', 'phase8'),
    0x58BBA0: ('purchases', 'Transport unload refund (credit)', 'phase8'),
    # Team commands / abilities moving resources
    0x509840: ('transfers', 'KKC_TeamCommand::Process; types 3 give and 4 tribute ported; 0/1/2/7 flares, 5 resource request (event 0x21 + SAI 7A3240), 6 event 0x22 + SAI 7A4B10, 9 chat/report 40BF70 are presentation/AI', 'phase8'),
    0x432650: ('transfers', 'Display rounding floor(amount+0.5) (via 96D5E0)', 'phase8'),
    0x509440: ('transfers', 'KKC_TeamCommand::TributeActor (type 4; events 0x1F/0x20)', 'phase8'),
    0x509790: ('transfers', 'Team command name table', ''),
    0x4CA680: ('transfers', 'KKC_AbilityEffectStealResources::Use', 'phase8'),
    0x4C8330: ('transfers', 'Ability use context target getter (+0x1C, alive virtual +0xB4)', 'phase8'),
    0x4E1910: ('construction', 'Kingdom/team actor restriction gate (team profile, then 4E1810 kingdom profile; world+0x12C string maps)', 'phase8'),
    0x4E1810: ('construction', 'Kingdom actor restriction (team then kingdom+0x348 profile, map +0x34)', 'phase8'),
    0x4E1890: ('research', 'Kingdom technology restriction (team then kingdom profile, map +0x44)', 'phase8'),
    0x5667F0: ('construction', 'ConstructionComponent command dispatch (Ghidra missed the function start): 0x13 cancel 565860, 0x14 sell 565A20, 0x15 upgrade 566360, 0x16 disable 564DC0', 'phase8'),
    0x566880: ('construction', 'KKC_ConstructionComponent::Tick (Ghidra missed the function start): dissolving owner states 0x16/0x14, 4D6660, auto-upgrade 501570/566110/566360, state 0x20 window', 'phase8'),
    0x564DC0: ('construction', 'Disable structure (command 0x16): state 0x1E, 4D1350, settlement queue clearing 584DE0 and refresh 584E10', 'phase8'),
    0x565400: ('construction', 'Enable structure (cancel command when disabled; ChangeKingdom 4D8450): state 0x1B, 565140 command recompute, settlement refresh 584E10', 'phase8'),
    0x565140: ('construction', 'Construction command availability recompute (actor+0xF0 bits 0x13..0x16)', 'phase8'),
    0x4D64F0: ('actor-state', 'Actor state set/clear (aa-actors owner; seam here)', 'seam'),
    0x4CEDA0: ('actor-state', 'Settlement center of the actor or its virtual +0x100 ancestors', 'seam'),
    0x589CC0: ('production', 'Structure queue clear (cancel every item via 589BD0)', 'seam'),
    0x4D6660: ('construction', 'Actor state refresh called by ConstructionComponent::Tick', 'seam'),
    0x4F9C10: ('construction', 'World actor creation from parameters (type 0xA25B28)', 'seam'),
    0x555830: ('construction', 'Map site lookup for ActorData+0x4E0 (BuildActor)', 'seam'),
    0x46EE00: ('construction', 'Footprint grid snap used by 6939B0', 'seam'),
    0x54EFC0: ('construction', 'Tile enemy test (owner, 5) for footprints', 'seam'),
    0x554DD0: ('construction', 'Footprint tile iterator begin (554330 advance)', 'seam'),
    0x4F0B70: ('construction', 'Zone owner at location (settlement zone check)', 'seam'),
    0x585500: ('construction', 'SettlementComponent find structure (center then list, 5890C0 match)', 'phase8'),
    0x586B40: ('construction', 'SettlementComponent AddStructure (limit, append, 585F60 attach seam)', 'phase8'),
    0x585F60: ('construction', 'Settlement structure attach (component links, refreshes)', 'seam'),
    # Capture
    0x5625C0: ('capture', 'BodyComponent::DealDamage (CaptureHealthFactor capture path)', ''),
}

PRIMITIVES = [0x4E1690, 0x4E39D0, 0x4E20A0, 0x4E3940, 0x4E48C0, 0x4E4930, 0x4E2590, 0x4E3AD0]
STRING_PATTERN = re.compile(
    r'(econom|stockpil|upkeep|bankrupt|research|technolog|supply|captur|construct|recruit|'
    r'purchase|income|tribut|steal_resources|give_resources|resources_given|resources_received|'
    r'faction_ability|recharge|insufficient stockpile|queue item|QueueItem)', re.I)
STRING_EXCLUDE = re.compile(r'(NiResource|ResourceManager|resource cache|ResourceCache|texture|shader|'
                            r'audio|video|KF resource|\.\\\\Source\\\\resource|Depot|DataFile|font|Film|'
                            r'Unable to create|GameSpy|tooltip|ui_|UI|VFX|View|button|Button|Display|Panel|'
                            r'metagame|MetaGame|Editor|editor|Toggle|color|Color|Glyph|floater|Floater|SAI|'
                            r'sai_|_sai)', re.I)
UTILITY_FAN_IN = 40


def code_range():
    rva, size, _ = SECTIONS[0]
    return BASE + rva, BASE + rva + size


def vtable(address, start, end, limit):
    """Slots until a non-code value, UTF-16 text, or the next known vtable."""
    slots = []
    while len(slots) < limit:
        value = struct.unpack('<I', at(address + 4 * len(slots), 4))[0]
        if not (start <= value < end) or value & 0xFF00FF00 == 0:
            return slots
        slots.append(value)
    return slots


def main():
    lo, hi = code_range()
    entries = {}

    def add(address, source):
        entries.setdefault(address, set()).add(source)

    for a in CURATED:
        add(a, 'curated')
    for p in PRIMITIVES:
        for c in CALLERS.get(p, ()):
            add(c, f'caller-of-{p:06X}')
    for a, values in STRINGS.items():
        hits = [v for v in values if STRING_PATTERN.search(v) and not STRING_EXCLUDE.search(v)]
        if hits:
            add(a, 'economy-string')
    kingdom_vt = vtable(0xA299D8, lo, hi, (0xA29B50 - 0xA299D8) // 4 - 1)
    updatable_vt = vtable(0xA29B50, lo, hi, 64)
    for i, a in enumerate(kingdom_vt):
        add(a, f'GKingdom-vtable-A299D8[{i}]')
    for i, a in enumerate(updatable_vt):
        add(a, f'GKingdom+180-vtable-A29B50[{i}]')
    for a in list(CURATED):
        for c in CALLEES.get(a, ()):
            if len(CALLERS.get(c, ())) <= UTILITY_FAN_IN:
                add(c, f'callee-of-{a:06X}')

    pinned = {}
    for fixture in sorted(HERE.glob('native-*-fixtures.json')):
        for name, pin_ in json.loads(fixture.read_text(encoding='utf-8'))['pins'].items():
            start = int(pin_['start'], 16)
            if lo <= start < hi:
                pinned.setdefault(start, name)
                add(start, f'oracle-pin-{fixture.stem}')
    starts = address_scan.function_starts()
    crates = address_scan.crate_references(starts)
    research = address_scan.research_references(starts)

    rows = []
    for a in sorted(entries):
        subsystem, role, note = CURATED.get(a, ('unclassified', '', ''))
        if a not in CURATED:
            s = sorted(set(STRINGS.get(a, [])))
            role = ('strings: ' + ' | '.join(x[:60] for x in s[:4])) if s else 'discovered via call graph/vtable; role not yet analysed'
        crate_refs = sorted(crates.get(a, ()))
        research_refs = sorted(research.get(a, ()))
        if note == 'seam':
            status = 'seam (external callback in current ports)'
        elif note == 'phase8' or a in pinned:
            status = 'phase8-port'
            if a not in CURATED:
                role = f'executed unchanged by a phase8 oracle ({pinned[a]})'
        elif crate_refs and research_refs:
            status = 'ported+oracle-referenced'
        elif crate_refs:
            status = 'referenced-in-crates (no oracle reference found)'
        elif research_refs:
            status = 'research-only (probe/seam, no crate port)'
        else:
            status = 'missing'
        row = FUNCTIONS.get(a)
        rows.append({
            'address': f'0x{a:06X}',
            'size': int(row['bodyBytes']) if row else None,
            'ghidra_function': bool(row),
            'subsystem': subsystem,
            'role': role,
            'status': status,
            'discovered_by': sorted(entries[a]),
            'crate_refs': crate_refs[:6],
            'research_refs': research_refs[:6],
        })
    summary = {}
    for r in rows:
        summary.setdefault(r['subsystem'], {}).setdefault(r['status'], 0)
        summary[r['subsystem']][r['status']] += 1
    doc = {
        'schema': 1,
        'generated_by': 'research/phase8-economy/build_inventory.py',
        'native': 'E:/Games/Axis and Alies/AA.exe (image base 0x400000); pseudocode/calls from reconstruction/research/native-aa',
        'status_legend': {
            'phase8-port': 'ported in this phase with an original-code Unicorn oracle (see README.txt for exact scope)',
            'ported+oracle-referenced': 'crate cites the address and an earlier research oracle cites it; scope per that README',
            'referenced-in-crates (no oracle reference found)': 'cited in Rust, no oracle evidence located automatically',
            'research-only (probe/seam, no crate port)': 'appears only in research scripts (often as an intercepted seam)',
            'seam (external callback in current ports)': 'deliberately left as an explicit callback; not executed by Rust',
            'missing': 'identified, no port',
        },
        'kingdom_vtable_A299D8': [f'0x{a:06X}' for a in kingdom_vt],
        'kingdom_economy_updatable_vtable_A29B50': [f'0x{a:06X}' for a in updatable_vt],
        'summary': summary,
        'functions': rows,
    }
    (HERE / 'inventory.json').write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8')
    print(len(rows), 'functions;', json.dumps(summary, indent=1))


if __name__ == '__main__':
    main()
