"""Execute original complete GTeam::Load for every authored TEAMS record.

Reuses header oracle allocation/CString/localization/I-O seams. Original 4EF160,
4E6970, 49AD70, 4CD0F0 and version helper8589B0 execute; record boundaries are
the resulting stream position, never a scan for names. Localization lookup is
a host seam in key-only RMP mode, not a live translated string reproduction.
"""
from pathlib import Path
import sys,struct,json,hashlib
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from native_header import Machine as Base,P,V,VISITED,read_chunks
from unicorn.x86_const import *

class Machine(Base):
 def __init__(self,data):
  super().__init__(data)
  self.world=self.alloc(0x400);self.write(self.world+0x3c,self.read(self.stream));self.write(self.read(self.stream)+12,0x2e00040)
  self.object=self.alloc(0x400);definition=self.alloc(0x400);self.write(self.object+4,definition)
  self.write(self.object+8,0x80000000);self.write(definition+0x1c,0x20042800)
 def hook(self,u,pc,size,arg):
  if pc==0x8589b0:return # execute original version reader, not Base's value seam
  if pc==0x2e00040:self.ret(value=0);return
  super().hook(u,pc,size,arg)
 def team(self):
  start=self.r.at;definition=self.r.string();category=self.r.u32();identity=self.r.u32();assert category==5
  self.write(self.stack,self.stop);self.write(self.stack+4,self.world);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.object)
  self.u.emu_start(0x4e6970,self.stop,count=1000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  def text(off):
   p=self.read(self.object+off);return bytes(self.u.mem_read(p,self.read(p-12)*2)).decode('utf16')
  def vector(off):return list(struct.unpack('<'+'I'*self.read(self.object+off+4),self.u.mem_read(self.read(self.object+off),4*self.read(self.object+off+4))))
  return {'offset':start,'end':self.r.at,'definition':definition,'serialized_id':identity,'slot_index':self.read(self.object+0x38),'name_key':text(0x3c),'words_4c':vector(0x4c),'words_58':vector(0x58),'string_64':text(0x64)}

def main():
 rows=[]
 for p in sorted((ROOT.parent/'reconstruction/assets/original/Data/Maps').rglob('*.rmp')):
  data=read_chunks(p)['TEAMS'][0];m=Machine(data);count=m.r.u32();entries=[]
  for slot in range(count):
   present=m.r.take(1)[0]
   if present:
    row=m.team();assert row['slot_index']==slot;entries.append({'slot':slot,'present_byte':present,**row})
   else:entries.append({'slot':slot,'present_byte':0})
  assert m.r.at==len(data),(p,m.r.at,len(data))
  rows.append({'map':str(p),'sha256':hashlib.sha256(data).hexdigest(),'decoded_bytes':m.r.at,'entries':entries})
 report={'schema':1,'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'maps':rows,'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())]}
 out=Path(__file__).with_name('native-teams.json');out.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'maps':len(rows),'team_records':sum(sum(bool(e['present_byte']) for e in r['entries']) for r in rows),'sha256':hashlib.sha256(out.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
