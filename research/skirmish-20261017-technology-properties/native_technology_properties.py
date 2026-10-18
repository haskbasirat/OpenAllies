"""Native parser4C4CF0 producer and45ECA0 head/iterator over the same ActorData.
Canonical Property Find is an explicit fixture identity service. The original
parser, getters, prepend/error prefix and45ECA0 stores execute both images/allPC.
"""
from pathlib import Path
import json,sys,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-world-start'))
import native_technology_source as old
import trace_registry as mapped
from unicorn.x86_const import UC_X86_REG_FPCW,UC_X86_REG_EIP
ORIGINAL=old.P;V14=old.V
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
def run(names,image,pc,canonical_properties=None):
 mapped.P=ORIGINAL if image=='original' else V14
 try:m=old.TechMachine()
 finally:mapped.P=ORIGINAL
 if canonical_properties is not None:
  m.properties={name:m.alloc(0x20)for name in canonical_properties}
 m.visited.clear();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
 source='[Thing template=Thing]\n{ IDS = subject\n [StructureComponent]\n {\n'+''.join('[TechnologyProperty]\n property_IDS = '+n+'\n' for n in names)+'}\n}\n'
 m.parse_file('template.tgi',old.TEMPLATE.encode());parsed=m.parse_file('subject.tgi',source.encode())
 actor=m.children(parsed['root'])[0];component=m.thiscall(0x5b4410,actor,[m.cstring('StructureComponent'),0])['eax']
 data=m.alloc(0x500);dst=data+0x43c;m.w(dst,0);m.w(dst+4,0)
 label=m.alloc(4);m.w(label,m.cstring('TechnologyProperty'))
 try:m.cdecl(0x4c4cf0,[component,label,dst])
 except AssertionError:assert m.u.reg_read(UC_X86_REG_EIP)==0x96bef5 and m.last_throw
 reverse={v:k for k,v in m.properties.items()};values=[];p=m.r(dst);last=0
 while p:values.append(reverse[m.r(p)]);last=p;p=m.r(p+4)
 assert m.r(dst+4)==last
 # Use45ECA0 even on a partial source list, preserving prior successful stores.
 iterator=m.alloc(8);m.w(iterator,0xcccccccc);m.w(iterator+4,0xcccccccc)
 m.thiscall(0x45eca0,data,[iterator]);assert m.r(iterator)==dst and m.r(iterator+4)==m.r(dst)
 observed=[];p=m.r(iterator+4)
 while p:observed.append(reverse[m.r(p)]);p=m.r(p+4)
 assert observed==values
 output=dict(values=values,throw=m.last_throw,events=m.recorded,iterator_list_is_actual_data43c=True,iterator_starts_at_actual_head=True)
 return output,{a:bytes(m.u.mem_read(a,n))for a,n in m.visited.items()if ORIGINAL.base<=a<ORIGINAL.base+ORIGINAL.image_size}
def main():
 rows=[];pins={}
 for pc,names in itertools.product(PC,[[],['first'],['first','second','third'],['first','FIRST','second'],['first','missing','third']]):
  a,pa=run(names,'original',pc);b,pb=run(names,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,names=names,output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==V14.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())])
 path=HERE/'native-technology-properties.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
