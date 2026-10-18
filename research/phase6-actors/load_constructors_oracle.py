"""Complete original saved-load constructors, without creation substitutions."""
from pathlib import Path
import json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine,P,V,H

WORDS=[0x39,0x3a,0x3c,0x3d,0x3e,0x42,0x43,0x44,0x45,0x46,0x47,0x48,0x49,0x4a,0x4b,0x4c,0x4d,0x4e,0x4f,0x50,0x51,0x52,0x53,0x54,0x55,0x56,0x57,0x58,0x59,0x5a,0x5b,0x5c,0x5d,0x5e]
def string(m,p):return list(struct.unpack('<%dH'%m.r(p-12),m.u.mem_read(p,m.r(p-12)*2)))
def main():
 rows=[];seen={}
 for kind,entry in [('Thing',0x4ef020),('Object',0x4ea690),('Actor',0x4d6d70)]:
  for saved in [1,0x12340007,0xffffffff]:
   m=Machine(0x27f);m.u.mem_write(m.obj,b'\xcd'*512);m.call(0x4efb70,[saved])
   # Native load-tag argument is unused by these constructors; definition is
   # second, the by-value GTID wrapper is third. All three ctors RET0x0c.
   m.invoke(entry,m.obj,[0,m.definition,saved])
   result={'flags':m.r(m.obj+8),'word_c':m.r(m.obj+12),'word_10':m.r(m.obj+16),'id':m.r(m.obj+20),'name_utf16':string(m,m.r(m.obj+24)),'word_1c_untouched':m.r(m.obj+28),'position_bits':[m.r(m.obj+32+4*i)for i in range(3)],'facing_bits':[m.r(m.obj+44+4*i)for i in range(3)]}
   if kind!='Thing':result['object']={'associated_handle':m.r(m.obj+0x38),'scale_bits':m.r(m.obj+0x3c),'variant':m.r(m.obj+0x40),'word_44':m.r(m.obj+0x44),'word_48':m.r(m.obj+0x48),'byte_4c':m.u.mem_read(m.obj+0x4c,1)[0],'words_50_to_5c':[m.r(m.obj+0x50+4*i)for i in range(4)]}
   if kind=='Actor':result['actor']={'components':[m.r(m.obj+0x60+4*i)for i in range(31)],'state':[m.r(m.obj+0x100),m.r(m.obj+0x104)],'string_dc_utf16':string(m,m.r(m.obj+0xdc)),'string_e0_utf16':string(m,m.r(m.obj+0xe0)),'words':{str(i*4):m.r(m.obj+i*4)for i in WORDS}}
   assert m.r(m.obj+28)==0xcdcdcdcd
   rows.append({'kind':kind,'saved_id':saved,'constructor':result,'registry':m.snapshot()});seen.update(m.visited)
 spans=[]
 for a,n in sorted(seen.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 pins=[]
 for a,z in spans:
  raw=P.range(a,z-a);assert raw==V.range(a,z-a);pins.append({'start':hex(a),'end':hex(z),'sha256':H(raw)})
 p=HERE/'load-constructors-native.json';p.write_text(json.dumps({'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'native_windows':pins,'seams':['Successful string-pool allocation is a leaf, with real original ctor CString assignment and registration instructions.','Synthetic object memory is prefilledCD to distinguish untouched bytes from initialized zero.','Saved generation is explicitly seeded before ctor as a stand-in for already-loaded ID-manager metadata.','No virtualLoad body, derived component codec, owner resolution or world publication is invoked.']},indent=2)+'\n');print(H(p.read_bytes()))
if __name__=='__main__':main()
