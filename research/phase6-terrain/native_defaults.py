"""Original first-entry default pointer assignment; source ordering is pinned separately."""
from pathlib import Path
import json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_ESI

def main():
 ranges={'biome_loader':(0x4b8e80,0x4b97c1),'terrain_loader':(0x4b87a0,0x4b8e21)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a)
 cases=[]
 for count in [1,2,7]:
  for pointer in [0x2030000,0x2041000,0x2050000]:
   m=Machine();m.reset(0);obj,arr=0x2002000,0x2003000
   m.w(obj+0x6c,arr);m.w(obj+0x70,count);m.w(obj+0x24,arr);m.w(obj+0x28,count)
   for i in range(count):m.w(arr+4*i,pointer+i*0x100)
   m.u.reg_write(UC_X86_REG_EBP,obj);m.u.emu_start(0x4b97b9,0x4b97c1,count=20)
   biome=struct.unpack('<I',m.u.mem_read(obj+0x78,4))[0];assert biome==pointer
   m.u.reg_write(UC_X86_REG_ESI,obj);m.u.emu_start(0x4b8e12,0x4b8e21,count=20)
   terrain=struct.unpack('<I',m.u.mem_read(obj+0x30,4))[0];assert terrain==pointer
   cases.append({'count':count,'first_pointer':pointer,'biome_default':biome,'terrain_default':terrain})
 metadata=json.loads((HERE/'terrain-render-metadata.json').read_text())
 report={'schema':1,'passed':True,'original_sha256':sha(original.b),'accepted_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'ranges':{n:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for n,(a,b)in ranges.items()},'cases':cases,'biome_defaults':metadata['biome_defaults'],'source_metadata_sha256':sha((HERE/'terrain-render-metadata.json').read_bytes()),'scope':'Executes first-array-element pointer assignments only. TGI child enumeration and ordered append are statically inspected in pinned complete loader prefixes; this is not a full native TGI parser execution.'}
 (HERE/'native-defaults.json').write_text(json.dumps(report,indent=2));print(json.dumps({'cases':len(cases),'defaults':metadata['biome_defaults'],'metadata_sha256':report['source_metadata_sha256']}))
if __name__=='__main__':main()
