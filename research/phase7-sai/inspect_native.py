from pathlib import Path
import sys,hashlib,json
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
TARGETS={'register_kingdom':0x79e170,'region_kingdom_create':0x7bc130,'region_kingdom_get':0x7b8ff0,'region_count':0x7b9240,'seen_append':0x7ba0d0,'seen_lookup':0x4ea430,'classify':0x79e300,'tile_occupied':0x54f870,'tile_occupied_mask':0x559530,'tile_explored':0x54ed00,'tile_explored_mask':0x54ecb0,'tile_visible':0x54f3c0,'tile_visible_mask':0x54ec70,'start_ai':0x7b4830,'controller_new':0x7a5750,'notify_kingdom':0x7a48f0,'special_actor':0x7a0c20,'update_manager':0x54c280}
TARGETS.update({'controller_auxiliary':0x7ccdc0,'kingdom_goal':0x7be920,'goal_base':0x7a66e0,'goal_prepend':0x7a4580,'region_player_create':0x7bc220,'region_player_new':0x7ba130,'region_goals':0x7a8230,'region_goal_new':0x7c6920,'ability_goals':0x7a4970,'ability_goal_new':0x7c37f0,'recruitment_goal':0x7a4a60,'recruitment_new':0x7c40c0,'recruitment_arrays':0x7c4040,'unit_goals':0x7a47c0,'unit_primary_new':0x7c4720,'unit_secondary_new':0x7c4890,'finish_goals':0x7a5960,'publish_goals':0x7a4f00,'neighbor_goals':0x7c8f70,'bounded_region_walk':0x7bb1c0,'kingdom_goal_expired':0x7a7a90,'unit_goal_expired':0x7a7b30,'seen_count':0x4ea3d0,'seen_table_new':0x4ea3e0,'actor_activity':0x4cf120})
def main():
 sizes={}
 for row in (ROOT.parent/'reconstruction/research/native-aa/functions.tsv').read_text().splitlines():
  p=row.split('\t')
  try:sizes[int(p[0],16)]=int(p[3])
  except(ValueError,IndexError):pass
 sizes.update({0x7b4830:0x2a0,0x7a7a90:0x36,0x7a7b30:0x17,0x4ea3d0:11,0x4ea3e0:0x4e})
 pins={};md=Cs(CS_ARCH_X86,CS_MODE_32)
 for name,addr in TARGETS.items():
  raw=original.range(addr,sizes[addr]);assert raw==modified.range(addr,len(raw))
  (OUT/(name+'-asm.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}'for i in md.disasm(raw,addr))+'\n')
  pins[name]={'address':hex(addr),'length':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}
 for a,n in [(0xa273f0,0xb8),(0xa9e560,0x88),(0xa9cac8,0x90),(0xa9cb58,0x90),(0xa9cd80,0x88),(0xa9e950,0x88),(0xa9e9d8,0x88),(0xa9eb10,0x88),(0xa9eb98,0x88),(0xa11740,4)]:
  raw=original.range(a,n);assert raw==modified.range(a,n);pins[hex(a)]={'address':hex(a),'length':n,'sha256':hashlib.sha256(raw).hexdigest(),'bytes':raw.hex()}
 (OUT/'provenance.json').write_text(json.dumps({'original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'routines':pins},indent=2)+'\n')
if __name__=='__main__':main()
