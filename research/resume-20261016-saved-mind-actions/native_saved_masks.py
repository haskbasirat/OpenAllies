"""Original530510 on source-observed tutorial01 saved owned components.

Native bit stores and Performer capability loops execute. Properties4CF110
executes on real pointer presence; virtual action queries read the live native
mask. Mobile predicates are explicit source-observed leaves separately proven
by the552-case575BB0/575C00 suite. No page-zero memory is mapped.
"""
from pathlib import Path
import copy, hashlib, json, sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-mind'))
import native_action_mask as baseline
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP

sha=lambda b:hashlib.sha256(b).hexdigest()
class Machine(baseline.MaskMachine):
 def hook(self,u,pc,size,data):
  if pc==baseline.ASTATE:
   actor=u.reg_read(UC_X86_REG_ECX);index=self.r(u.reg_read(UC_X86_REG_ESP)+4)
   assert index<64
   bits=self.r(actor+0xf0)|(self.r(actor+0xf4)<<32)
   self.events.append(['live_action',baseline.aid(actor),index])
   self.ret(int(bool(bits&(1<<index))),4)
  elif pc==0x4cf110:
   # Execute this original getter, not MaskMachine's old player seam.
   self.visited[pc]=bytes(u.mem_read(pc,size))
  else: super().hook(u,pc,size,data)

def case(row,precision):
 v=row['input'];c=copy.deepcopy(next(baseline.cases()))
 c['name']=f"tutorial01_{row['id']}_{precision}"
 c['precision']=precision
 a=copy.deepcopy(v['actor'])
 a.update(element=201 if a['element'] is not None else None,
  organization=202 if a['organization'] is not None else None,
  performer=301 if a['performer'] is not None else None,
  componentac=203 if a['componentac'] is not None else None,
  action_bits=v['before'])
 c['actors']['101']=a
 c['actors']['102']=copy.deepcopy(a)
 c['elements']={'201':102 if v['attached'] is not None else None}
 if v['leader'] is not None:
  leader=v['leader'];other=copy.deepcopy(leader['actor'])
  other.update(element=None,organization=202,performer=None,componentac=None,action_bits=0)
  c['actors']['102']=other
  c['organizations']['202'].update(link=leader['link'])
  c['registry']={str(leader['link']):None if leader['resolved'] is None else 102}
 else:c['registry']={}
 c['abilities']={'301':v['abilities'] or []}
 c['players']={'401':v['properties'] or 0}
 c['actor_players']={'101':None if v['properties'] is None else 401,'102':None}
 c['setting108']=v['fog']['Ok']
 if v['mobile'] is not None:
  c['ac']={str(0x575bb0):v['mobile']['unpack']['Ok'],str(0x575c00):v['mobile']['pack']['Ok']}
 c['mutations']=[]
 return c

def main():
 source=Path(sys.argv[1])
 document=json.loads(source.read_text())
 rows=[];visited={}
 for observed in document['minds']:
  assert observed['result']=={'Ok':None},observed['result']
  for precision in ['Single24','Double53','Extended64']:
   for image_name,image in [('original',baseline.original),('v14',baseline.modified)]:
    c=case(observed,precision);m=Machine(c)
    old=(baseline.original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
    if new>old:m.u.mem_map(image.base+old,new-old)
    for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
    # Reload image before restoring test globals (image sections include them).
    m.w(0xb39ab0,baseline.CACHE)
    m.w(0xb364c0,baseline.REG)
    m.u.mem_unmap(0,4096)
    m.w(baseline.ap(101)+0x78,0 if observed['input']['properties'] is None else baseline.pl(401)-0x1c)
    m.w(baseline.ap(102)+0x78,0)
    try: result=m.run_mask()
    except Exception:
     from unicorn.x86_const import UC_X86_REG_EIP
     print(c['name'],image_name,'fault',hex(m.u.reg_read(UC_X86_REG_EIP)))
     raise
    assert result['actions']['101']==observed['actions'],(c['name'],result,observed)
    visited.update(m.visited)
    rows.append({'image':image_name,'source_actor':observed['id'],'input':c,'output':result})
 for address,code in visited.items():
  assert code==baseline.original.range(address,len(code))==baseline.modified.range(address,len(code)),hex(address)
 output={'scope':__doc__,'source_report':str(source),'source_report_sha256':sha(source.read_bytes()),
  'original_sha256':sha(baseline.original.b),'v14_sha256':sha(baseline.modified.b),
  'producer_sha256':sha(Path(__file__).read_bytes()),'cases':rows,
  'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]}
 target=HERE/'native-saved-masks.json';target.write_text(json.dumps(output,indent=2)+'\n')
 print(json.dumps({'cases':len(rows),'instruction_sites':len(visited),'sha256':sha(target.read_bytes())}))
if __name__=='__main__':main()
