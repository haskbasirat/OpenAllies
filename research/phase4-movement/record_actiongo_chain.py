"""Source-only continuation map; does not implement or execute path planning."""
from pathlib import Path
import hashlib, json, sys
ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/phase3-terrain'))
from terrain_uv_oracle import Pe, ORIGINAL_SHA, ACCEPTED_SHA
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

RANGES = {
    'action_go': (0x5300a0, 0x5303c8),
    'move_path_event_setup': (0x532170, 0x532230),
    'formation_path_event_setup': (0x532310, 0x5323bb),
    'path_node_copy': (0x52d000, 0x52d078),
    'path_owned_copy': (0x52d0f0, 0x52d174),
    'path_count': (0x51e8d0, 0x51e8ea),
    'path_clear': (0x5189f0, 0x518a33),
    'result_plan_solver_call': (0x533a39, 0x533a87),
}


def main():
    original, v14 = Pe(GAME / 'AA.exe', ORIGINAL_SHA), Pe(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', ACCEPTED_SHA)
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    ranges = {}
    for name, (a, z) in RANGES.items():
        data = original.at(a, z-a); assert data == v14.at(a, z-a)
        path = Path(__file__).parent / f'actiongo-{name}-assembly.txt'
        path.write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in md.disasm(data, a)) + '\n')
        ranges[name] = {'start': hex(a), 'end_exclusive': hex(z), 'sha256': hashlib.sha256(data).hexdigest(), 'unchanged_v14': True}
    report = {
        'schema': 1, 'read_only': True, 'path_planning_implemented': False,
        'original_sha256': ORIGINAL_SHA, 'v14_sha256': ACCEPTED_SHA,
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'ranges': ranges,
        'ownership': [
            'The UAI main component owns its move-state subobject at +0x7c (5321f6/532386).',
            'MoveState+4 refers to UAI; UAI+4 refers to actor. Shared helper529c30 follows two +4 links; its semantic result depends on caller type.',
            'MoveState embeds a path collection at +0x58, equivalent to UAI+0xd4; first node is collection+4 / MoveState+0x5c.',
            'Path collection+8 is last node. Nodes have eight 32-bit payload words, previous pointer+0x20, next pointer+0x24.',
            '52d0f0 clears destination nodes then allocates/copies nodes in source-next order via52d000/85c680, using poolB27AE0. It copies collection metadata +c/+10/+18 and marks+14.',
            'EventMovePath532170 and EventMoveFormationPath532310 activate MoveState, copy the supplied path into the owned collection, and call52ce00 to remove trailing type2 nodes. They do not merely retain a foreign node pointer.'
        ],
        'node_fields_consumed_by_action_go': {'0x00': 'waypoint x', '0x04': 'waypoint y', '0x08': 'direction x', '0x0c': 'direction y',
                                             '0x10': 'node kind; zero requests instant turning', '0x14..0x1c': 'retained payload; semantics not recovered in this slice'},
        'chain': [
            {'entry': '0x61e830', 'role': 'Move-order processing constructs actor command type5; generic component routing4d9b60 remains a separate boundary.'},
            {'entry': '0x52df60', 'role': 'UAIStateMove activation stores destination, movement/precision mode, requested speed and auxiliary direction fields before state activation52a810.'},
            {'entry': '0x5336b0', 'role': 'ResultPlan considers retry/status, target/organization geometry and calls the world path solver.'},
            {'entry': '0x53e1f0', 'role': 'Called through GWorld+0x238 with output MoveState+0x58, actor XY, target XY, collision/query context and player policy. Solver internals not ported.'},
            {'entry': '0x531520', 'role': 'Replan/update path uses53c170 and time gating; successful result calls ActionGo.'},
            {'entry': '0x5300a0', 'role': 'Consumes a prepared path node; handles already-reached/behind-node, turn mode, speed/formation choices, then calls the Mover.'},
            {'entry': '0x5773c0', 'role': 'Receives node direction, waypoint minus direction*MoveState+0x54, effective speed and factor. This bounded operation is implemented.'},
            {'entry': '0x530379..0x5303bc', 'role': 'After issuing the mover operation, reads native game RNG via4f02b0/45d400 to schedule the next UAI callback; timing must not be guessed.'},
        ],
        'unresolved_dependencies': [
            {'entry': '0x52a810 and virtual state methods', 'need': 'UAI activation/deactivation and ownership state transitions'},
            {'entry': '0x52e6a0', 'need': 'Single-node special turn/placement predicate'},
            {'entry': '0x52ff00', 'need': 'Formation movement mode3 conditions/factor'},
            {'entry': '0x4ce6c0 ->0x44dea0', 'need': 'Actor spatial/zone policy used to stop or continue'},
            {'entry': '0x553ba0 /0x553ce0 /0x53c170 /0x53e1f0', 'need': 'Collision, low-path and region-path policies, ordering, geometry and tie-breaks'},
            {'entry': '0x4f02b0 ->0x45d400', 'need': 'Correct shared RNG stream and scheduling consumer, not only isolated RNG arithmetic'},
            {'entry': '0x5324a0 and Mind virtual callbacks', 'need': 'Arrival state machine, formations, target updates and retry/failure transitions'},
        ],
        'limits': ['Instruction/source inspection only for this continuation map; no path/ActionGo behavior tests or implementation claimed.',
                   'Decompiler signatures are advisory; raw argument setup and member offsets ground the listed ownership.',
                   'Do not wire MoveOrder destination directly into Mover and call that original pathfinding.'],
    }
    path = Path(__file__).parent / 'actiongo-chain.json'; path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__': main()
