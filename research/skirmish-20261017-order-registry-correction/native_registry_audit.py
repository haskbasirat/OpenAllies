"""Audit the actual CRT table, then execute every direct order initializer.

No calls.tsv/function-boundary inventory is used for discovery. Calls made by
non-order initializers remain an explicit seam; this is not whole CRT execution.
"""
from pathlib import Path
import argparse, json, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
import recover_orders as recovery
import original_orders as base
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

CS = Cs(CS_ARCH_X86, CS_MODE_32)
REGISTER = 0x766e40

def audit(image):
    table = list(struct.unpack('<%dI' % ((recovery.TABLE_END-recovery.TABLE_BEGIN)//4),
                              image.range(recovery.TABLE_BEGIN, recovery.TABLE_END-recovery.TABLE_BEGIN)))
    starts = sorted(set(table)-{0})
    rows = []
    for start in starts:
        pending, seen, sites, indirect = [start], {}, [], []
        while pending:
            pc = pending.pop()
            while pc not in seen:
                assert image.base <= pc < image.base+image.image_size, hex(pc)
                ins = next(CS.disasm(image.range(pc, 15), pc, count=1), None)
                assert ins is not None, hex(pc)
                seen[pc] = bytes(ins.bytes)
                assert len(seen) < 100000, hex(start)
                target = int(ins.op_str, 16) if ins.op_str.startswith('0x') else None
                if ins.mnemonic == 'call':
                    if target == REGISTER: sites.append(pc)
                    elif target is None: indirect.append(pc)
                if ins.mnemonic.startswith('ret') or ins.mnemonic in ('int3', 'ud2'):
                    break
                if ins.mnemonic.startswith('j') or ins.mnemonic.startswith('loop'):
                    assert target is not None, (hex(start), hex(pc), ins.op_str)
                    if target == REGISTER: sites.append(pc)
                    else: pending.append(target)
                    if ins.mnemonic == 'jmp': break
                pc += ins.size
        rows.append({'initializer': hex(start), 'crt_slots': [hex(recovery.TABLE_BEGIN+4*i) for i,v in enumerate(table) if v==start],
                     'instructions': len(seen), 'instruction_sha256': recovery.sha(b''.join(struct.pack('<I',pc)+seen[pc] for pc in sorted(seen))),
                     'register_sites': [hex(pc) for pc in sorted(sites)], 'indirect_calls': [hex(pc) for pc in sorted(indirect)]})
    return table, rows

def execute(image, table, rows):
    callers = {int(row['initializer'],16) for row in rows if row['register_sites']}
    old_base, old_recovery, old_callers, old_table = base.PE0, recovery.PE0, recovery.CALLERS, recovery.TABLE
    try:
        base.PE0 = recovery.PE0 = image
        recovery.CALLERS, recovery.TABLE = callers, table
        m = recovery.OrdersMachine()
        m.call(0x96ec76, [0])
        count = m.r(m.order_manager+4)
        assert count == len(m.registrations)
        assert callers == {int(row['initializer'],16) for row in m.registrations}
        assert [row['id'] for row in m.registrations] == list(range(count))
        for i, row in enumerate(m.registrations):
            ptr = m.r(m.r(m.order_manager)+4*i)
            assert m.r(ptr+4)==i and m.text(m.r(ptr+8))==row['name']
        pins = [{'pc':hex(pc),'size':size,'sha256':recovery.sha(image.range(pc,size))}
                for pc,size in sorted(m.visited.items()) if image.base<=pc<image.base+image.image_size]
        return {'registrations':m.registrations, 'executed_pins':pins}
    finally:
        base.PE0, recovery.PE0, recovery.CALLERS, recovery.TABLE = old_base, old_recovery, old_callers, old_table

def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--write',action='store_true'); args=parser.parse_args()
    results=[]
    for label,image in [('original',recovery.PE0),('v14',recovery.PE1)]:
        table,rows=audit(image)
        result=execute(image,table,rows)
        results.append({'image':label,'sha256':recovery.sha(image.b),'crt_table_sha256':recovery.sha(image.range(recovery.TABLE_BEGIN,recovery.TABLE_END-recovery.TABLE_BEGIN)),
                        'audit':rows,**result})
        print(label,len(rows),'CRT initializers',len(result['registrations']),'registrations',flush=True)
    assert results[0]['audit']==results[1]['audit']
    assert results[0]['registrations']==results[1]['registrations']
    assert results[0]['executed_pins']==results[1]['executed_pins']
    exported={int(r.split('\t')[0],16) for r in (ROOT.parent/'reconstruction/research/native-aa/calls.tsv').read_text().splitlines()[1:] if r.split('\t')[1]=='00766e40'}
    actual={int(r['initializer'],16) for r in results[0]['registrations']}
    report={'schema':1,'producer_sha256':recovery.sha(Path(__file__).read_bytes()),'results':results,
            'missing_from_calls_tsv':[hex(x) for x in sorted(actual-exported)],'exported_but_not_registered':[hex(x) for x in sorted(exported-actual)],
            'scope':'All actual CRT initializer entry points are decoded through direct branch/tail-jump control flow. Direct CALL/JMP 766E40 discovery is independent of Ghidra functions/calls exports. The actual CRT loop plus all discovered order initializers and real registration append bodies execute separately in both PEs. Non-order callbacks, allocation, CString transport and diagnostics remain explicit seams. No whole CRT or arbitrary indirect registration discovery claim.'}
    path=HERE/'native-registry-audit.json'
    if args.write:path.write_text(json.dumps(report,indent=2)+'\n')
    else:assert json.loads(path.read_text())==report
    print(json.dumps({'count':len(actual),'missing':report['missing_from_calls_tsv'],'sha256':recovery.sha(path.read_bytes())}))

if __name__=='__main__':main()
