"""Full original/v14 code identity for saved topology and its world-load gate."""
from pathlib import Path
import json,sys,hashlib
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
RANGES={'load':(0x7aa9b0,0x7aac90),'region_constructor':(0x7ba4f0,0x7ba620),'region_load':(0x7bbaf0,0x7bbc70),'rectangle_load':(0x7bacf0,0x7baee0),'side_load':(0x7b94a0,0x7b9560),'region_rectangle_link':(0x7b99b0,0x7b99f0),'primary_island':(0x7aa2a0,0x7aa3d0),'world_region_lookup':(0x7aa3d0,0x7aa470),'rectangle_region_lookup':(0x7b9300,0x7b9320),'rectangle_type':(0x7b9320,0x7b9360),'tile_type':(0x7a9ac0,0x7a9b00),'region_area':(0x7b9900,0x7b9940),'adjacency':(0x7bca60,0x7bccc0),'side_iterator':(0x7ba050,0x7ba0a0),'prepare':(0x7abc30,0x7abd40),'worldload_rebuild_gate':(0x4fc875,0x4fc8f4)}
def main():
 md=Cs(CS_ARCH_X86,CS_MODE_32);pins={}
 for name,(a,z)in RANGES.items():
  raw=original.range(a,z-a);assert raw==modified.range(a,z-a)
  (OUT/(name+'-full-assembly.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}'for i in md.disasm(raw,a))+'\n')
  pins[name]={'start':hex(a),'end_exclusive':hex(z),'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True}
 (OUT/'native-provenance.json').write_text(json.dumps({'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'ranges':pins,'scope':'Complete routine windows include alignment bytes. prepare7ABC30 evidence identifies external startup work; bottleneck/cache preparation is not claimed implemented.'},indent=2)+'\n')
 print({'passed':True,'ranges':len(pins)})
if __name__=='__main__':main()
