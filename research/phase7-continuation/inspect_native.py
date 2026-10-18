from pathlib import Path
import sys,json,hashlib
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
TARGETS={name:(address,1) for name,address in [('continuation',0x5379c0),('waypoint_offset',0x52fd90),('near_waypoint',0x51d510),('abs_angle',0x43aa00),('forward_collision',0x537150),('collision_result',0x536f40),('back_to_path',0x532b50),('save_collision',0x52fd50),('collision_probe',0x531170),('avoid_actor',0x536840),('stop_distance',0x536ad0),('result_path',0x530f40),('wait',0x52e130),('collision_plan',0x534a40),('collision_location',0x52f3e0),('line_intersect',0x52ddb0),('normalize',0x409230),('node_construct',0x52ca90),('node_prepend',0x52d180),('zero_prefix_remove',0x52cf40),('acos',0x96dbb0) ]}

def main():
 sizes={}
 for row in (ROOT.parent/'reconstruction/research/native-aa/functions.tsv').read_text().splitlines():
  p=row.split('\t')
  try:sizes[int(p[0],16)]=int(p[3])
  except(ValueError,IndexError):pass
 md=Cs(CS_ARCH_X86,CS_MODE_32);pins={}
 for name,(a,n)in TARGETS.items():
  n=max(n,sizes.get(a,0));raw=original.range(a,n);assert raw==modified.range(a,n)
  (OUT/(name+'-asm.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}'for i in md.disasm(raw,a))+'\n');pins[name]={'address':hex(a),'length':n,'sha256':hashlib.sha256(raw).hexdigest()}
 constants={}
 for a,n in [(0x9f3228,4),(0x9f32dc,8),(0x9fbdb8,4),(0xa32b58,4),(0xa01458,4),(0xa34770,4),(0xa35320,8)]:
  data=original.range(a,n);assert data==modified.range(a,n);constants[hex(a)]={'length':n,'bytes':data.hex(),'sha256':hashlib.sha256(data).hexdigest()}
 (OUT/'constants.json').write_text(json.dumps(constants,indent=2)+'\n')
 (OUT/'provenance.json').write_text(json.dumps({'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'functions':pins},indent=2)+'\n')
if __name__=='__main__':main()
