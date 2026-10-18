"""Original/v14 absent Economy and null allegiance exits,4CF0F0->570020.

Runs actual GActor owner getter4D1E80. Nonnull owner stops before570051 as
an explicit unported boundary, not a native exception or a successful refresh.
No resource arithmetic is claimed. Prior full refresh oracle is pinned too.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import Machine,MODES,ORIGINAL,ACCEPTED,pin,sha
from unicorn import UC_HOOK_MEM_READ
A,E,VT=0x2010000,0x2011000,0x2012000
class Boundary(Exception):pass
class Entry(Machine):
    def __init__(self,cw,c,image):
        super().__init__(cw);self.trace=[];self.collect=False
        if image=='v14':
            old=(ORIGINAL.size+4095)&~4095;new=(ACCEPTED.size+4095)&~4095
            if new>old:self.uc.mem_map(ORIGINAL.base+old,new-old)
            self.uc.mem_write(ACCEPTED.base,ACCEPTED.data[:ACCEPTED.header_size])
            for rva,size,offset,_ in ACCEPTED.sections:
                if size:self.uc.mem_write(ACCEPTED.base+rva,ACCEPTED.data[offset:offset+size])
        self.uc.mem_write(E,bytes((i*37+19)&255 for i in range(0x58)))
        self.put(A,VT);self.put(VT+0x108,0x4d1e80);self.put(A+0x84,E if c['economy_present'] else 0)
        self.put(A+0xe8,c['kingdom']);self.put(E+4,A)
        self.before_actor=bytes(self.uc.mem_read(A,0x200));self.before_economy=bytes(self.uc.mem_read(E,0x58))
        self.seam(0x570051,self.boundary)
        self.uc.hook_add(UC_HOOK_MEM_READ,self.read_event);self.collect=True
    def boundary(self,m,esp):raise Boundary()
    def read_event(self,u,access,addr,size,value,user):
        if self.collect and addr in (A+0x84,E+4,A+0xe8):
            self.trace.append(['actor84' if addr==A+0x84 else 'component_owner' if addr==E+4 else 'actorE8',self.read(addr)])
    def run(self):
        outcome='returned'
        try:self.call(0x4cf0f0,[],A)
        except Boundary:outcome='unported_nonnull_owner'
        return {'outcome':outcome,'trace':self.trace,'actor_unchanged':self.before_actor==bytes(self.uc.mem_read(A,0x200)),
                'component_unchanged':self.before_economy==bytes(self.uc.mem_read(E,0x58))}
def main():
    rows=[]
    for image in ('original','v14'):
        for pc,cw in MODES:
            for present in (False,True):
                for owner in (0,17,0xffffffff):
                    c={'economy_present':present,'kingdom':owner}
                    rows.append({'image':image,'precision':pc,'input':c,'output':Entry(cw,c,image).run()})
    previous=ROOT/'research/phase7-economy-refresh/native-refresh-fixtures.json'
    out={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
         'prior_refresh_oracle_sha256':sha(previous.read_bytes()),'generator_sha256':sha(Path(__file__).read_bytes()),
         'function_windows':pin({'entry':(0x4cf0f0,0x4cf100),'owner':(0x4d1e80,0x4d1e87),'refresh_head':(0x570020,0x570051),'refresh_return':(0x5701f6,0x570210)}),'cases':rows}
    path=HERE/'native-entry.json';path.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
