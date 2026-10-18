"""Hash and disassemble immutable original/v14 spatial function/data ranges."""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe, ORIGINAL_SHA, ACCEPTED_SHA
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
OUT=Path(__file__).parent
RANGES={
 'move_ground':(0x4eeb40,0x4eebd4),
 'terrain_query':(0x4f0e50,0x4f107a),
 'commit_position':(0x4ee5c0,0x4ee6f9),
 'snap_half_cell':(0x46ee00,0x46ee65),
 'old_shape_a':(0x4cf210,0x4cf263),
 'old_shape_b':(0x4ea0f0,0x4ea143),
 'render_notify':(0x50c250,0x50c26d),
 # Sampler entry ABI evidence only: these two calls are explicit oracle seams.
 'height_sample_a':(0x550190,0x550205),
 'height_sample_b':(0x550210,0x550219),
}
DATA={'neighbors':(0xb322e0,72),'neighbor_next':(0xab7804,36),
 'half':(0x9fbdb8,4),'one':(0x9f32e0,4),'milli':(0x9f4580,4)}
def main():
 p=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA)
 q=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA)
 cs=Cs(CS_ARCH_X86,CS_MODE_32);rows=[];data=[]
 for name,(a,z) in RANGES.items():
  b=p.at(a,z-a);assert b==q.at(a,z-a)
  (OUT/(name+'.asm.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in cs.disasm(b,a))+'\n')
  rows.append({'name':name,'start':hex(a),'end':hex(z),'sha256':hashlib.sha256(b).hexdigest()})
 for name,(a,n) in DATA.items():
  b=p.at(a,n);assert b==q.at(a,n)
  data.append({'name':name,'start':hex(a),'end':hex(a+n),'sha256':hashlib.sha256(b).hexdigest(),'bytes':b.hex()})
 assert p.at(0xb322e0,72)==struct.pack('<18i',0,0,-1,-1,-1,0,-1,1,0,1,1,1,1,0,1,-1,0,-1)
 assert p.at(0xab7804,36)==struct.pack('<9i',0,2,3,4,5,6,7,8,1)
 (OUT/'native-ranges.json').write_text(json.dumps({'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'ranges':rows,'data':data},indent=2)+'\n')
if __name__=='__main__':main()
