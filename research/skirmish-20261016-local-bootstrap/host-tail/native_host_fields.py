"""Actual75B20E..75B274 Host input/state/options writer slice.
Whole earlier Host prefix inherited. Caller inputs are explicit UI fixture
values, not original UI defaults or a certificate of map/settings selection.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent/'host'))
from native_local_host import HostMachine,PE0,PE1,PC,NODE,proof_ranges
from unicorn.x86_const import *
def main():
 rows=[];visited={}
 for image in ('original','v14'):
  for p in PC:
   for kind,mode,game_type,multi,countdown in [(12,2,2,False,0),(6,2,2,False,0),(7,0,3,True,0),(12,2,5,False,1)]:
    m=HostMachine(image);m.fpu(p);assert m.setup(0x41800000,0)==0x75b20e
    frame=m.u.context_save();args=m.u.reg_read(UC_X86_REG_EDI)
    m.call(0x40f7d0,this=args);m.w(args,kind);m.w(args+0xc,0x11223344);m.w(args+0x10,4);m.w(args+0x18,12)
    m.w(args+0x2c,mode);m.w(args+0x30,game_type);m.b(args+0x34,multi)
    launch=m.alloc(0x28);m.call(0x40f7d0,this=launch);m.w(m.S+0x80,launch)
    m.call(0x77b7b0,this=NODE+0xcc);m.cs(NODE+0xcc,'User game\x00tail');m.cs(NODE+0xd0,'secret')
    m.w(NODE+0xd4,100);m.w(NODE+0xd8,0x12345678);m.b(NODE+0xdc,0)
    m.w(m.S+0x118,countdown);m.u.context_restore(frame)
    m.u.emu_start(0x75b20e,0x75b274,count=1_000_000)
    assert m.u.reg_read(UC_X86_REG_EIP)==0x75b274
    out={'mode':m.r(m.S+0x58),'game_type':m.r(m.S+0x54),'state':m.read_state(launch),'multiplayer':bool(m.rb(m.S+0xf4)),
      'options':{'name':m.rs(m.S+0xcc),'password':m.rs(m.S+0xd0),'max_players':m.r(m.S+0xd4),'field_d8':m.r(m.S+0xd8),'allow_observers':bool(m.rb(m.S+0xdc))}}
    assert out['mode']==(3 if countdown else mode) and out['game_type']==game_type and out['state']==m.read_state(args)
    rows.append({'image':image,'precision':p,'kind':kind,'input_mode':mode,'input_game_type':game_type,'input_multiplayer':multi,'countdown':countdown,'after':out});visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'ActualHost75B20E..75B274 and771240/7745C0/4145C0; explicit supplied UI values, successful inherited string/UI seams, no original UI defaults/map launch claim',
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-host-fields.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'Host field rows')
if __name__=='__main__':main()
