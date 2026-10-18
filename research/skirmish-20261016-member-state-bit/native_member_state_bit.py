"""Execute real 4D1080 on original/v14, all PCs; no replaced instructions/seams."""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,ORIGINAL,ACCEPTED,MODES,pin,sha

def main():
    rows=[]
    words=sorted(set([0,0xffffffff,0xfffffffb,0x55555555,0xaaaaaaaa]+[1<<i for i in range(32)]))
    for image in ['original','v14']:
        for pc,cw in MODES:
            for high in words:
                m=Machine(cw)
                if image=='v14':
                    old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
                    if new>old:m.uc.mem_map(ORIGINAL.base+old,new-old)
                    for rva,size,offset,_ in ACCEPTED.sections:
                        if size:m.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
                actor=m.alloc(0x108);m.put(actor+0x100,high^0xffffffff);m.put(actor+0x104,high)
                before=bytes(m.uc.mem_read(actor,0x108));eax,_=m.call(0x4d1080,[],actor)
                assert bytes(m.uc.mem_read(actor,0x108))==before
                assert eax in [0,1]
                rows.append({'image':image,'precision':pc,'high':high,'low':high^0xffffffff,'result':bool(eax)})
    out={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
        'producer_sha256':sha(Path(__file__).read_bytes()),'harness_sha256':sha((ROOT/'research/phase8-economy/harness.py').read_bytes()),
        'range':pin({'4D1080':(0x4d1080,0x4d1098)}),'actor_bytes_unchanged':True,'cases':rows}
    path=HERE/'native-member-state-bit.json'
    if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert json.loads(path.read_text())==out
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
