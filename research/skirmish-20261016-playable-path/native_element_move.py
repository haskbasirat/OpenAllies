"""Re-execute original538340 with explicit effect seams and page-zero guard.

This is not battle execution: sleep/move/kill/bombard/follow, target acceptance,
Performer indirect fire and Updatable Schedule are named hooked boundaries.
"""
from pathlib import Path
import hashlib
import json
import sys
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase7-mind'))
from native_element_command import ElementMachine, cases, original, modified
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    rows = []; visited = {}; zero_reads = []
    for case in cases():
        machine = ElementMachine(case)
        def guard(uc, access, address, size, value, opaque):
            pc = uc.reg_read(UC_X86_REG_EIP)
            # Only an actual FS segment override can use the SEH page. Do not
            # silently turn null object reads into the synthetic SEH sentinel.
            if bytes(uc.mem_read(pc, 1)) != b'\x64':
                zero_reads.append({'pc': hex(pc), 'address': address, 'size': size})
                uc.emu_stop()
        machine.u.hook_add(UC_HOOK_MEM_READ, guard, begin=0, end=0xfff)
        result = machine.run()
        rows.append({'input': case, 'output': result})
        visited.update(machine.visited)
    assert not zero_reads, zero_reads
    for address, instruction in visited.items():
        assert original.range(address, len(instruction)) == instruction
        assert modified.range(address, len(instruction)) == instruction
    previous = ROOT / 'research/phase7-mind/native-element-command.json'
    assert rows == json.loads(previous.read_text())['cases']
    doc = {'schema': 1, 'original_sha256': sha(original.b), 'v14_sha256': sha(modified.b),
           'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows,
           'existing_fixture_sha256': sha(previous.read_bytes()),
           'scope': 'Original538340 actually executed at PC24/53/64; all reached instruction bytes equal v14. Effects listed below are seams, not completed gameplay.',
           'effect_seams': ['57E280 indirect fire', 'target virtual+16C', 'Updatable virtual+4 Schedule',
                            '52A810 Sleep activation', '52DF60 Move activation', '5351C0 Kill activation',
                            '538120 Bombard activation', '532930 Follow activation'],
           'page_zero_non_fs_reads': zero_reads,
           'instructions': [{'va': hex(a), 'bytes': b.hex()} for a, b in sorted(visited.items())]}
    path = HERE / 'native-element-move.json'
    path.write_text(json.dumps(doc, indent=2) + '\n')
    print(json.dumps({'cases': len(rows), 'page_zero_violations': len(zero_reads),
                      'precisions': sorted(set(c['input']['precision'] for c in rows)),
                      'native_fixture_sha256': sha(path.read_bytes())}))


if __name__ == '__main__':
    main()
