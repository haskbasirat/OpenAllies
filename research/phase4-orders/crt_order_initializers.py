"""Discover direct registration callers from actual CRT entries, not exports."""
import struct
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

TABLE_BEGIN=0xb21000
TABLE_END=0xb27964
REGISTER=0x766e40

def audit(image, sha):
    cs=Cs(CS_ARCH_X86,CS_MODE_32)
    table=list(struct.unpack('<%dI'%((TABLE_END-TABLE_BEGIN)//4),image.range(TABLE_BEGIN,TABLE_END-TABLE_BEGIN)))
    rows=[]
    for start in sorted(set(table)-{0}):
        pending,seen,sites,indirect=[start],{},[],[]
        while pending:
            pc=pending.pop()
            while pc not in seen:
                assert image.base<=pc<image.base+image.image_size,hex(pc)
                ins=next(cs.disasm(image.range(pc,15),pc,count=1),None)
                assert ins is not None,hex(pc)
                seen[pc]=bytes(ins.bytes)
                assert len(seen)<100000,hex(start)
                target=int(ins.op_str,16) if ins.op_str.startswith('0x') else None
                if ins.mnemonic=='call':
                    if target==REGISTER:sites.append(pc)
                    elif target is None:indirect.append(pc)
                if ins.mnemonic.startswith('ret') or ins.mnemonic in ('int3','ud2'):break
                if ins.mnemonic.startswith('j') or ins.mnemonic.startswith('loop'):
                    assert target is not None,(hex(start),hex(pc),ins.op_str)
                    if target==REGISTER:sites.append(pc)
                    else:pending.append(target)
                    if ins.mnemonic=='jmp':break
                pc+=ins.size
        rows.append({'initializer':hex(start),'crt_slots':[hex(TABLE_BEGIN+4*i)for i,v in enumerate(table)if v==start],
                     'instructions':len(seen),'instruction_sha256':sha(b''.join(struct.pack('<I',pc)+seen[pc]for pc in sorted(seen))),
                     'register_sites':[hex(pc)for pc in sorted(sites)],'indirect_calls':[hex(pc)for pc in sorted(indirect)]})
    return table,rows
