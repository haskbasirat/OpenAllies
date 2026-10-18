"""Native5788B0/57DC40 over captured actual saved-restore scalar/owner facts.

Unmodified native arithmetic executes. Missing completed1E4 is an explicitly
injected required-source interruption at57891A before its FLD; this is not a
native throw claim. Original/v14 and the captured incoming precision execute.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,pin,sha
from unicorn import UC_HOOK_CODE
class RequiredSource(Exception):pass
class Saved(Machine):
    def __init__(self,cw,c,image):
        super().__init__(cw);self.c=c;self.cursor=0x2001000
        if image=='v14':
            old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
            if new>old:self.uc.mem_map(ORIGINAL.base+old,new-old)
            self.uc.mem_write(ACCEPTED.base,ACCEPTED.data[:ACCEPTED.header_size])
            for rva,size,offset,_ in ACCEPTED.sections:
                if size:self.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
        self.org=self.fixture(0x100);members=self.fixture(max(4,len(c['members'])*4))
        data=self.fixture(0x40);groups=self.fixture(max(4,len(c['groups'])*4));params=self.fixture(max(4,len(c['parameters'])*4))
        self.put(self.org+0x28,members);self.put(self.org+0x2c,len(c['members']));self.put(self.org+0x18,data);self.put(data+0x28,groups);self.put(self.org+0x1c,params)
        self.put(self.org+0x68,c['current']);self.put(self.org+0x6c,c['maximum'])
        for i,g in enumerate(c['groups']):self.put(groups+i*4,g)
        self.unbound=set()
        for i,p in enumerate(c['parameters']):
            if p is not None:
                d=self.fixture(0x200);self.put(params+i*4,d);self.put(d+0x174,p['flags'])
                if p['value'] is None:self.unbound.add(d)
                else:self.put(d+0x1e4,p['value'])
        for i,m in enumerate(c['members']):
            if m is None:continue
            actor=self.fixture(0x200);self.put(members+i*4,actor)
            if m['performer'] is not None:
                p=m['performer'];performer=self.fixture(0x70);self.put(actor+0x74,performer);self.put(performer+4,actor);self.put(performer+0x60,p['value'])
                assert p['owner']==m['id'],'actual canonical Performer owner'
                if p['body'] is not None:
                    body=self.fixture(0x20);self.put(actor+0x60,body);self.put(body+0x10,p['body'][0]);self.put(body+0x14,p['body'][1])
        self.uc.hook_add(UC_HOOK_CODE,self.code)
    def fixture(self,n):
        p=self.cursor;self.cursor+=(n+15)&~15;assert self.cursor<0x2400000
        return p
    def code(self,u,address,size,user):
        if address==0x57891a:
            from unicorn.x86_const import UC_X86_REG_EAX
            if u.reg_read(UC_X86_REG_EAX) in self.unbound:raise RequiredSource()
    def run(self):
        outcome='ok'
        try:self.call(0x5788b0,[],self.org)
        except RequiredSource:outcome='required_source_failure'
        return {'outcome':outcome,'current':self.read(self.org+0x68),'maximum':self.read(self.org+0x6c)}
def main():
    captured=json.loads((HERE/'actual-saved-inputs.json').read_text());modes=dict(MODES);rows=[]
    for row in captured:
        for image in ('original','v14'):
            m=Saved(modes[row['precision']],row['input'],image)
            rows.append({'image':image,'precision':row['precision'],'input':row['input'],'output':m.run()})
    doc={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),'generator_sha256':sha(Path(__file__).read_bytes()),'captured_inputs_sha256':sha((HERE/'actual-saved-inputs.json').read_bytes()),'windows':pin({'organization':(0x5788b0,0x578931),'weighted_performer':(0x57dc40,0x57dc5e)}),'cases':rows}
    path=HERE/'native-saved-org-combat.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
