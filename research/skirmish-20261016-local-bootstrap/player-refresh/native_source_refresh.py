"""Execute full7791F0 against actual accepted Rust-source launch cells.
This is a source-projection graph replay, not a claim that native AA loaded
the map live. Source hashes/typed constructor states are retained in input.
All reached selection/assignment/update functions execute original/v14 code;
only successful inherited OS/heap/CString/diagnostic leaves are seams.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_refresh import Machine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import UC_X86_REG_ESI
def string(units):return ''.join(chr(u)for u in units)
class Graph(Machine):
 def descriptor(self,kind,ids):
  if ids is None:return 0
  key=(kind,tuple(ids))
  if key not in self.descriptors:
   p=self.alloc(0x80);self.u.mem_write(p,b'\0'*0x80);self.cs(p+8,string(ids))
   self.descriptors[key]=p;self.pointer_ids[p]=ids
  return self.descriptors[key]
 def parameters(self,source,refs):
  params=self.alloc(0x1c);records=self.alloc(max(1,len(source['kingdoms']))*0x6c)
  self.w(params+0x10,records);self.w(params+0x14,len(source['kingdoms']));self.w(params+0x18,len(source['kingdoms']))
  for i,(row,ref)in enumerate(zip(source['kingdoms'],refs,strict=True)):
   p=records+i*0x6c;self.u.mem_write(p,b'\0'*0x6c)
   for off,key in ((0,'id'),(4,'display_key'),(0xc,'string_0c'),(0x14,'string_14'),(0x2c,'string_2c'),(0x58,'string_58')):
    assert row[key]is not None,(i,key);self.cs(p+off,string(row[key]))
   self.cs(p+8,'') #Unused companion, not read by this reached function.
   for off,key in ((0x10,'byte_10'),(0x28,'byte_28'),(0x29,'byte_29'),(0x31,'byte_31'),(0x32,'byte_32')):self.b(p+off,row[key])
   for off,kind in ((0x18,'nation'),(0x1c,'faction'),(0x24,'handicap')):self.w(p+off,self.descriptor(kind,ref[kind]))
  return params,records
def main():
 capture_path=HERE/'actual-source-input.json';capture=json.loads(capture_path.read_text());data=capture['input'];rows=[];visited={}
 assert data['parameters']==data['base_parameters'],'initial selected/header value copy differs'
 for image in ('original','v14'):
  m=Graph(image);del m.seams[0x778d10]
  for pc in PC:
   m.fpu(pc);m.begin(1,list(range(16)),0);m.descriptors={};m.pointer_ids={}
   info=m.alloc(0x58);m.call(0x77e5e0,this=info);m.w(m.S+0x14,info)
   m.call(0x76ec70,this=m.S);m.first_player(string(data['player']['name']),'source-fixture-identity')
   p=m.r(m.r(m.S+0xbc));assert m.r(p+0x20)==data['player']['id']
   for off,key in ((0x24,'kingdom'),(0x30,'text_30'),(0x34,'text_34')):m.cs(p+off,string(data['player'][key]))
   for off,key in ((0x2c,'flag_2c'),(0x38,'flag_38')):m.b(p+off,int(data['player'][key]))
   for off,key in ((0x3c,'field_3c'),(0x40,'field_40')):m.w(p+off,data['player'][key])
   actual,actual_records=m.parameters(data['parameters'],data['references'])
   base,base_records=m.parameters(data['base_parameters'],data['references'])
   creator=m.alloc(0x54);m.u.mem_write(creator+4,bytes(m.u.mem_read(actual,0x1c)))
   header=m.alloc(0x68);rules=m.alloc(0x1c);m.w(rules+4,0);m.w(header+4,rules)
   m.w(header+8,base);m.w(m.S+0x68,header);m.w(m.S+0x70,creator)
   m.w(m.S+0x58,2);m.w(m.S+0x54,2);m.w(m.S+0x60,0xa5a5a5a5)
   m.u.reg_write(UC_X86_REG_ESI,m.S+0x54);m.u.emu_start(0x76b47e,0x76b485,count=1);assert m.r(m.S+0x60)==0
   m.b(m.S+0x169,0);m.b(m.S+0xc8,int(data['changed_c8']))
   table=m.alloc(4);descriptor=m.alloc(12);payload=m.alloc(2);m.w(table,descriptor);m.w(descriptor+8,payload);m.b(payload,0);m.w(0xb3b198,table)
   m.w(creator+0x24,0)
   m.call(0x7791f0,this=m.S)
   state=[]
   for i in range(len(data['parameters']['kingdoms'])):
    record=actual_records+i*0x6c
    def ids(off):
     ptr=m.r(record+off);return None if ptr==0 else m.pointer_ids[ptr]
    state.append({'nation':ids(0x18),'faction':ids(0x1c),'handicap':ids(0x24),'modified68':bool(m.rb(record+0x68))})
   after={'parameter_state':state,'player':{'kingdom':m.rs(p+0x24),'flag_2c':bool(m.rb(p+0x2c)),
    'flag_38':bool(m.rb(p+0x38)),'field_3c':m.r(p+0x3c),'field_40':m.r(p+0x40)},'changed_c8':bool(m.rb(m.S+0xc8))}
   assert after==capture['rust_after'],(image,pc,after,capture['rust_after'])
   rows.append({'image':image,'precision':pc,'after':after});visited.update(m.visited)
 out={'passed':True,'scope':__doc__,'input':data,'input_capture_sha256':hashlib.sha256(capture_path.read_bytes()).hexdigest(),
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),
  'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-source-refresh.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'actual-source native refresh rows')
if __name__=='__main__':main()
