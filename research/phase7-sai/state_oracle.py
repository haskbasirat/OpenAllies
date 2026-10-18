"""Actual region/Kingdom allocation and tile counters; no counter callback hooks."""
from pathlib import Path
import copy,json,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase7-navigation'))
from topology_oracle import Machine as Base,H,original,modified
from unicorn.x86_const import *
class Machine(Base):
 def call(self,pc,ecx,args=()):
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  super().call(pc,ecx,args)
  assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+(0 if pc==0x79e300 else 4*len(args)),(hex(pc),'esp')
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)],hex(pc)
 def owned(self,case):
  self.call(0x7aa9b0,self.graph,[self.stream-0x3c,self.grid])
  sv=self.alloc(0x200);value=self.alloc(4);self.write(0xb39ab0,sv);self.write(sv+0x108,value);self.write(value,int(case['settings']['fog_enabled']));self.write(0xa11740,case['settings']['visible_bit_offset'])
  for i,t in enumerate(case['tiles']):self.write(self.tiles+i*48+0x14,t['word14']);self.u.mem_write(self.tiles+i*48+0x18,struct.pack('<H',t['word18']))
  sai=self.alloc(0x100);kingdom_ptrs={};states=[]
  for k in case['kingdoms']:
   kp=self.alloc(0x300);self.write(kp+0x14,k['native_id']);self.write(kp+0x18c,k['major_index']);self.write(kp+0x190,k['category']);kingdom_ptrs[kp]=k['object'];self.call(0x79e170,sai,[kp]);snapshot=[]
   for ri in range(self.read(self.graph+0x18)):
    region=self.read(self.read(self.graph+0x14)+ri*4);rows=[]
    for ki in range(self.read(region+0x14)):
     p=self.read(self.read(region+0x10)+ki*4)
     if p:
      self.call(0x7b8ff0,region,[kp]);expected_lookup=self.u.reg_read(UC_X86_REG_EAX)
      # Exact7BA0D0 repeated IDs and native head order on one record.
      if case.get('append_seen')and ki==k['major_index']:
       for value in [17,32,17]:self.call(0x7ba0d0,p,[value])
      nodes=[];n=self.read(p+0x18)
      while n:nodes.append(self.read(n));n=self.read(n+4)
      rows.append({'region':ri,'kingdom':kingdom_ptrs[self.read(p+4)],'explored':self.read(p+8),'occupied':self.read(p+12),'visible':self.read(p+16),'word14':self.read(p+20),'seen_records':nodes})
     else:rows.append(None)
    snapshot.append({'kingdoms':rows})
   states.append({'regions':snapshot})
  return states

def main():
 source=json.loads((ROOT/'research/phase7-navigation/synthetic-topology.json').read_text())['fixtures'];fixtures=[];visited={}
 for topo in source[:7]:
  for fog,offset in [(False,16),(True,16),(True,7)]:
   ks=[{'object':i+1,'native_id':0x10001+i,'major_index':idx,'category':cat}for i,(idx,cat)in enumerate([(0,0),(2,0),(2,0),(15,0),(16,0),(31,0),(32,0),(5,1)])]
   tiles=[{'word14':((i*0x172593)^0xa5a518c3)&0xffffffff,'word18':((i*197)^0x59a3)&0xffff}for i in range(64)]
   case={'name':topo['name']+str(fog)+'_'+str(offset),'input_hex':topo['input_hex'],'layout':topo['layout'],'world_version':28,'settings':{'fog_enabled':fog,'visible_bit_offset':offset},'tiles':tiles,'kingdoms':ks,'append_seen':True}
   m=Machine(bytes.fromhex(case['input_hex']),case['layout'],[0x200]*64);got=m.owned(case);fixtures.append({'input':case,'native':got});visited.update(m.executed)
 # Exact79E300 broad finite truth table, original4EA3D0 and4EA430 complete lookup.
 m=Machine(bytes.fromhex(source[0]['input_hex']),source[0]['layout'],[0x200]*64);actor=m.alloc(0x200);vtable=m.alloc(0x200);m.write(actor,vtable);m.write(vtable+0x8c,0x4ea3d0);classes=[]
 for flags in [0,1<<23,1<<22,(1<<22)|(1<<23)]:
  for wall in [False,True]:
   for mask in [0]+[1<<i for i in range(9)]+[511]:
    fields={'flags8':flags,'component90':wall,'components':[bool(mask&(1<<i))for i in range(9)]};m.write(actor+8,flags);m.write(actor+0x90,int(wall))
    for i,off in enumerate([0x7c,0x80,0x84,0x88,0x94,0x98,0xa0,0xa8,0xac]):m.write(actor+off,int(fields['components'][i]))
    m.call(0x79e300,0,[actor]);classes.append({'input':fields,'native':bool(m.u.reg_read(UC_X86_REG_EAX)&255)})
 seen=[];kp=m.alloc(0x200);arr=m.alloc(0x100);m.write(actor+0x5c,arr)
 for count in [-1,0,1,17]:
  m.write(actor+0x58,count)
  for category in [0,1,0xffffffff]:
   for idx in [0,1,15,32]:
    m.write(kp+0x190,category);m.write(kp+0x18c,idx);m.write(arr+4*idx,0 if idx==1 else idx+5)
    m.call(0x4ea430,actor,[kp]);seen.append({'count58':count,'category':category,'index':idx,'records':[None if i==1 else i+5 for i in range(33)],'native':m.u.reg_read(UC_X86_REG_EAX)or None})
 visited.update(m.executed);pins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),'fixtures':fixtures,'classifications':classes,'seen_lookups':seen,'native_basic_block_pins':pins,'scope':'Actual7AA9B0 load,79E170 no AI slots,7BC130 allocation,7B9240 tile masks,7B8FF0 lookup,7BA0D0 prepend; allocator/version/stream leaves only. Actual79E300 and4EA430→GActor4EA3D0. No initialized-AI notifications, actual fog creation or wholeSAI startup claim.'}
 (OUT/'native-state.json').write_text(json.dumps(doc,indent=2)+'\n');print({'passed':True,'region_sequences':len(fixtures),'registration_calls':len(fixtures)*8,'classifications':len(classes),'seen':len(seen)})
if __name__=='__main__':main()

