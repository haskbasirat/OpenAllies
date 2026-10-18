"""Original TGI parser and48C050 placeholder prefix, then475280 deferred fixup.
Stops before unrelated depot/music/texture/faction loading is completed.
Thing/placement-group lookup are explicit canonical fixture seams.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-layouts'))
from layout_machine import Machine,DATA,P,V,H,native_windows
from unicorn.x86_const import *
class NationMachine(Machine):
    def __init__(self):
        self.mapping={};self.prefix=False;super().__init__()
        for name in ['placeholder_corps','placeholder_bunker','gbr_corps','gbr_bunker','other']:
            self.mapping[name]=self.alloc(0x20)
    def hook(self,u,pc,size,user):
        if pc==0x5b54d0 and self.raw_wstr(self.arg())=='depot_name':
            self.prefix=True;u.emu_stop();return
        if pc==0x48d500:
            self.recorded.append({'placement_group':self.raw_wstr(self.arg())});self.ret(0x123456,pop=4);return
        if pc==0x4bdbe0:
            name=self.raw_wstr(self.arg());value=self.mapping.get(name.lower(),0)
            self.recorded.append({'thing_lookup':name,'kind':self.arg(1),'resolved':bool(value)})
            self.ret(value,pop=8);return
        if pc==0x405d60:
            self.w(self.arg(),self.cstring('<diagnostic>'));self.ret();return
        super().hook(u,pc,size,user)
def run(pairs):
    m=NationMachine();m.parse_file('Templates/template_nation.tgi')
    source='[Nation template=Nation]\n{ IDS = fixture\nname = "Fixture"\nplacement_group_IDS = pg\nresource_prefix = Fixture\n'+''.join('[Placeholder]\n{ placeholder_IDS = '+a+'\nthing_IDS = '+b+'\n}\n' for a,b in pairs)+'}\n'
    parsed=m.parse_file('nation-fixture.tgi',source.encode());node=m.children(parsed['root'])[0]
    dst=m.alloc(0x80);m.u.mem_write(dst,b'\0'*0x80);m.w(dst+8,m.cstring('fixture'));m.w(dst+0x10,m.cstring(''))
    try:m.thiscall(0x48c050,dst,[node])
    except AssertionError:assert m.prefix or m.last_throw
    if m.prefix:
        fix=m.r(m.gd+0x824)
        while fix:
            result=m.thiscall(0x475280,fix,[])
            if 'thrown'in result:break
            fix=m.r(fix+8)
    rows=[];p=m.r(dst+0x1c);rev={v:k for k,v in m.mapping.items()}
    while p:
        value=m.r(p+4)
        rows.append({'placeholder':rev.get(m.r(p),f'unknown:{m.r(p):x}'),'replacement':None if value==0 else rev.get(value,f'unset:{value:x}')});p=m.r(p+8)
    return m,{'source':source,'pairs':pairs,'native_rows':rows,'prefix_complete':m.prefix,'thrown':m.last_throw,'seams':m.recorded}
def main():
    rows=[];visited={}
    for pairs in [[],[('placeholder_corps','gbr_corps')],[('placeholder_corps','gbr_corps'),('placeholder_bunker','gbr_bunker'),('placeholder_corps','other')],[('placeholder_corps','null')],[('unknown','gbr_corps')],[('placeholder_corps','missing')]]:
        m,row=run(pairs);rows.append(row);visited.update(m.visited);print(pairs,row['native_rows'],bool(row['thrown']),flush=True)
    out={'schema':1,'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'template':(DATA/'Templates/template_nation.tgi').read_text(errors='replace'),'cases':rows,'native_windows':native_windows(visited)}
    (HERE/'native-nation-pairs.json').write_text(json.dumps(out,indent=2)+'\n')
if __name__=='__main__':main()
