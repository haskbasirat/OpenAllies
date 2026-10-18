"""Actual478D20 map_strings destruction/replacement and4BA710 loading,
while another registered TextData survives. Native parser/registration/Find
execute; only inherited OS/CString allocator/logging transports are seams.
The explicit source-read cutoff after Reset proves removal before a later
unavailable source, not a fabricated successful parse or fallback.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));from native_mapstrings import MapStrings,L
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PCS={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
def raw(entries):
 return ('[Text language=Default]\n{\nids=explicit\nname="table"\n'+''.join(k+'="'+v+'"\n'for k,v in entries)+'}\n').encode()
def node(m,data):
 buf=m.alloc(len(data)+16);m.u.mem_write(buf,data);name=m.alloc(4);m.w(name,m.cstring('MAPSTRINGS'));r=m.call(0x473050,0,[name]);m.w(r+0x30,buf);m.w(r+0x34,len(data));m.u.mem_write(r+0x44,b'\0');m.call(m.r(m.r(r)+0x4c),r);return m.call(0x5b4410,m.r(r+0x3c),[m.cstring('Text'),0])
def create(m,name,entries):
 p=m.alloc(0x24);arg=m.alloc(4);m.w(arg,m.cstring(name));m.call(0x475180,p,[arg]);m.call(m.r(m.r(p)),p,[node(m,raw(entries))]);return p
def find(m,mgr,key):
 arg=m.alloc(4);m.w(arg,m.cstring(key));p=m.call(0x4b9d40,mgr+0x4c,[arg]);return 0 if not p else m.r(p)
def run(image,pc,stop,index):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=MapStrings();m.u.reg_write(UC_X86_REG_FPCW,PCS[pc]);mgr=m.manager()
 create(m,'global_fixture',[('global_key','Kept'),('global_other','Other')]);global_entry=find(m,mgr,'global_key');assert global_entry
 old=create(m,'map_strings',[('map_key','Old'+str(index)),('old_key','OldOnly')]);gamedata=m.r(0xb36530);m.w(gamedata+0x5b4,old)
 assert m.lookup(mgr,'map_key')=='Old'+str(index)
 m.call(0x478d20,gamedata);new=m.r(gamedata+0x5b4);assert new and new!=old
 cleared=dict(global_same=find(m,mgr,'global_key')==global_entry,global_text=m.lookup(mgr,'global_key'),old_key_absent=find(m,mgr,'old_key')==0,map_key_absent=find(m,mgr,'map_key')==0)
 assert all(cleared[k]for k in ['global_same','old_key_absent','map_key_absent'])and cleared['global_text']=='Kept'
 if not stop:m.call(m.r(m.r(new)),new,[node(m,raw([('map_key','New'+str(index)),('new_key','NewOnly')]))])
 out=dict(cleared=cleared,error='explicit source-read cutoff after478D20'if stop else None,
  global_same=find(m,mgr,'global_key')==global_entry,map_text=m.lookup(mgr,'map_key'),old_key_absent=find(m,mgr,'old_key')==0,new_text=m.lookup(mgr,'new_key'))
 if not stop:assert out['map_text']=='New'+str(index)and out['new_text']=='NewOnly'
 assert out['global_same']and out['old_key_absent']
 return out,m.windows()
def main():
 rows=[];blocks={}
 for pc,stop,index in itertools.product(PCS,[False,True],range(2)):
  a,pa=run('original',pc,stop,index);b,pb=run('v14',pc,stop,index);assert a==b
  blocks.update({r['address']:r for r in pa+pb});rows.append(dict(precision=pc,source_stop=stop,index=index,output=a))
 data=(json.dumps(dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,blocks=[blocks[k]for k in sorted(blocks)]),indent=1)+'\n').encode();path=HERE/'native-map-strings-replacement.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,blocks=len(blocks),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
