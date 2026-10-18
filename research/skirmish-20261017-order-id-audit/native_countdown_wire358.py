"""Focused actual766D00 descriptor field/count ->40C7C0 ->40D420 wire producer.
Native32-bit payload40D420 executes afterward as at76CBF0. The authority is the
genuine fullCRT358 audit, not a hand-picked ordinal. No Session/Node/launch
service, Ack handler or whole native launch path is claimed by this slice.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
AUTH=ROOT/'research/skirmish-20261017-order-registry-correction/native-registry-audit.json'
assert n.sha(AUTH.read_bytes())=='7d0ef97a4ea1a9a565016ecc25fe1c2fe27dff756910c71990268cd038989b76'
regs=json.loads(AUTH.read_text())['results'][0]['registrations'];source=next(r for r in regs if r['descriptor']=='0xb40b2c')
assert source['name']=='TGC_LaunchCountdownProgressOrderRequest'and source['id']==326

def run(image,pc,value):
 m=n.Machine(pc);old=(n.original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,off in image.sections:m.u.mem_write(image.base+rva,image.b[off:off+size])
 manager=m.alloc(8);m.w(manager+4,len(regs));m.w(0xb27a84,manager)
 m.w(int(source['descriptor'],16)+4,source['id'])
 storage=m.alloc(72);m.w(storage,1);m.w(storage+4,64)
 writer=m.alloc(16);m.w(writer,0xa39da8);m.w(writer+4,storage+8);m.w(writer+8,0);m.w(writer+12,3)
 m.call(0x766d00,0,[writer,int(source['descriptor'],16)],pop=0)
 payload=m.alloc(4);m.w(payload,value);m.call(0x40d420,0,[writer,payload,32],pop=0)
 byte=m.r(writer+8);bit=m.r(writer+12);bits=byte*8+bit
 out=dict(bytes=list(m.u.mem_read(storage+8,(bits+7)//8)),end=dict(byte=byte,bit=bit))
 reader=m.alloc(16);m.w(reader,0xa39da4);m.w(reader+4,storage+8);m.w(reader+8,0);m.w(reader+12,3)
 decoded=m.alloc(4);m.call(0x40d5d0,0,[reader,decoded,9],pop=0);assert m.r(decoded)==source['id']
 m.call(0x40d5d0,0,[reader,decoded,32],pop=0);assert m.r(decoded)==value
 return out,m.visited

def main():
 rows=[];pins={}
 for pc,value in itertools.product(n.PRECISIONS,[0,1,42,0x80000000,0xffffffff]):
  a,pa=run(n.original,pc,value);b,pb=run(n.modified,pc,value);assert a==b
  assert a['end']==dict(byte=5,bit=4)
  pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,value=value,output=a))
 for at,bits in pins.items():assert n.original.range(at,len(bits))==n.modified.range(at,len(bits))==bits
 doc=dict(scope=__doc__,source=source,registry_count=len(regs),authority_sha256=n.sha(AUTH.read_bytes()),rows=rows,native_executions=len(rows)*2,instructions=n.instructions(pins),original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b))
 out=HERE/'native-countdown-wire358.json'
 if '--write'in sys.argv:
  with out.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=n.sha(out.read_bytes()))))
if __name__=='__main__':main()
