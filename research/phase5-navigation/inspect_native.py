"""Record original/v14 identical instruction windows; no native execution."""
from pathlib import Path
import hashlib, json, struct, sys
ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe, ORIGINAL_SHA, ACCEPTED_SHA
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
OUT=Path(__file__).parent
RANGES={'path_node':(0x52d000,0x52d073),'path_assign':(0x52d0f0,0x52d176),
 'path_clear':(0x5189f0,0x518a25),'path_prune':(0x52ce00,0x52ce72),
 'move_event':(0x532170,0x532310),'formation_event':(0x532310,0x5324a0),
 'activate':(0x52df60,0x52e0e1),'action_go':(0x5300a0,0x5303c8),
 'active_state':(0x52a000,0x52a020),'can_activate':(0x529c00,0x529c1e),
 'owner_chain':(0x529c30,0x529c37),'random_float':(0x45d400,0x45d423),
 'random_world':(0x4f02b0,0x4f02e9),'catchup_speed':(0x577090,0x57709e),
 'owner_handle_selection':(0x4ce6c0,0x4ce6dd),'related_handle_valid':(0x44dea0,0x44ded7),
 'catchup_setting':(0x506e9a,0x506eda),'group_setting':(0x5085af,0x5085e5),
 'path_count':(0x51e8d0,0x51e8ea),'stop_action':(0x52d660,0x52d6b4)}
def main():
 a,b=Pe(GAME/'AA.exe',ORIGINAL_SHA),Pe(GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA)
 md=Cs(CS_ARCH_X86,CS_MODE_32); rows={}
 for name,(start,end) in RANGES.items():
  data=a.at(start,end-start); assert data==b.at(start,end-start)
  (OUT/(name+'-assembly.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in md.disasm(data,start))+'\n')
  rows[name]={'start':hex(start),'end_exclusive':hex(end),'sha256':hashlib.sha256(data).hexdigest(),'v14_identical':True}
 constants={}
 for x in [0xa34770,0x9f32dc,0xa34d40,0x9f32e0,0xb279c0,0xb279c4]:
  data=a.at(x,4);assert data==b.at(x,4),hex(x)
  constants[hex(x)]={'bits':struct.unpack('<I',data)[0],'f32':struct.unpack('<f',data)[0],'v14_identical':True}
 doubles={}
 for x in [0x9fdaa0,0x9fda98]:
  data=a.at(x,8);assert data==b.at(x,8),hex(x)
  doubles[hex(x)]={'bits':struct.unpack('<Q',data)[0],'v14_identical':True}
 settings={}
 for addr,cache in [(0xa2f774,0x190),(0xa30494,0x48)]:
  raw=a.at(addr,100);name=raw.decode('utf-16-le').split('\0')[0];payload=a.at(addr,len(name)*2+2);assert payload==b.at(addr,len(payload))
  settings[name]={'string_address':hex(addr),'cache_offset':hex(cache),'string_sha256':hashlib.sha256(payload).hexdigest()}
 (OUT/'native-ranges.json').write_text(json.dumps({'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'ranges':rows,'constants':constants,'double_constants':doubles,'settings':settings},indent=2)+'\n')
 print(json.dumps(constants))
if __name__=='__main__': main()
