"""Original4D0EE0/4EB820/4EB500 callback orchestration with explicit world leaves.
Native membership, footprint expansion, cached cells, regiment copy, follower
offset arithmetic and history ring bodies execute. Expensive world mutations,
attachment virtuals, diagnostics and Element/target-mover services are seams.
"""
from pathlib import Path
import json,struct
from original_index import Index,shape,bits,W,H,original,modified
from unicorn.x86_const import *
HERE=Path(__file__).parent
class Position(Index):
    COMP=0x2600000
    def __init__(self,row,cw):
        super().__init__(row['grid'],row['members'],row['flags'],cw)
        self.row=row;self.calls=[];self.u.mem_map(self.COMP,0x10000)
        self.components={k:self.COMP+k*0x100 for k in [10,11,12,13,14,15]}
        self.component_ids={p:k for k,p in self.components.items()}
        self.w(0xb36550,0x200e100);self.w(0x200e1e4,row['session_mode'])
        self.w(0x200a048,row['world_word_48']);self.w(0x200a0e4,row['time_bits'])
        self.w(0xb39ab0,0x200e200);self.w(0x200e218,0x200e300);self.w(0x200e300,row['margin_bits'])
        self.w(0x200e228,0x200e304);self.w(0x200e304,row['interval'])
        self.w(0x200e308,row['ground_height_bits']);self.u.mem_write(0x200e400,b'\xd9\x05'+W(0x200e308)+b'\xc2\x08\x00')
        for m in row['members']:
            ptr=self.pointers[m['key']];self.u.mem_write(ptr+0x44,W(*m['cached_tile']))
        p=self.pointers[1];d=self.r(p+4);c=row['components']
        self.w(d+0x8c,row['padding_bits']);self.u.mem_write(d+0x270,W(*row['follower_offset'],0))
        for field,offset in [('organization',0x7c),('element',0x80),('mover',0x6c),('settlement',0x98),('history',0x144)]:
            self.w(p+offset,self.components[c[field]]if c[field]is not None else 0)
        self.w(p+0x168,c['follower_reference'])
        for k,ptr in self.components.items():self.w(ptr+4,p)
        self.u.mem_write(self.components[10]+0xbc,W(*row['organization_xy']))
        self.w(self.components[13]+0x14,self.pointers[2]if row['settlement_actor']is not None else 0)
        self.w(self.pointers[3]+0x6c,self.components[15]);self.w(self.components[15]+4,self.pointers[3])
        h=row['history'];hp=self.components[14];samples=self.COMP+0xf000
        self.w(hp,p);self.w(hp+4,samples);self.w(hp+8,len(h['samples']));self.w(hp+0x10,h['read_index']);self.w(hp+0x14,h['write_index'])
        self.w(hp+0x18,int(h['reset_latch']));self.w(hp+0x1c,h['last_frame']);self.u.mem_write(samples,b''.join(W(*v)for v in h['samples']))
    def ret(self,n=0,value=None):
        sp=self.u.reg_read(UC_X86_REG_ESP)
        if value is not None:self.u.reg_write(UC_X86_REG_EAX,value)
        self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
    def vector(self,ptr,n=2):return list(struct.unpack('<'+'I'*n,self.u.mem_read(ptr,n*4)))
    def placed(self,ptr):
        s=self.r(ptr);v=self.vector(s,6)
        return {'shape':{'kind':v[0],'radius_bits':v[1],'bounds':v[2:]},'facing':self.vector(ptr+4),'position':self.vector(ptr+12)}
    def hook(self,u,pc,size,user):
        sp=u.reg_read(UC_X86_REG_ESP);arg=lambda i=0:self.r(sp+4+i*4)
        receiver=u.reg_read(UC_X86_REG_ECX)
        if pc==0x4f0e50:
            self.calls.append({'op':'ground_height','position':self.vector(arg()),'flags':arg(1)})
            u.reg_write(UC_X86_REG_EIP,0x200e400);return
        if pc==0x551140:
            v=self.vector(arg(),6);s=arg(4)
            self.calls.append({'op':'edit_height','edit':{'placement':{'shape':{'kind':v[0],'radius_bits':v[1],'bounds':v[2:]},'facing':self.vector(arg(1)),'position':self.vector(arg(2))},
                'settings_words':self.vector(s,5),'flags':[bool(u.mem_read(s+20,1)[0]),bool(u.mem_read(s+21,1)[0])],'strength_bits':self.r(s+24),'trailing_flag':bool(u.mem_read(s+32,1)[0])}})
            assert arg(2)==arg(3);self.ret(20);return
        if pc==0x4f5560:self.calls.append({'op':'clear_doodads','placement':self.placed(arg())});self.ret(4);return
        if pc==0x4ea260:self.calls.append({'op':'ground_variation'});self.ret(value=self.row['variation']or 0);return
        if pc==0x555c10:
            v=self.vector(arg(),6)
            self.calls.append({'op':'paint_terrain','paint':{'placement':{'shape':{'kind':v[0],'radius_bits':v[1],'bounds':v[2:]},'facing':self.vector(arg(1)),'position':self.vector(arg(2))},'variation':arg(4),'deferred':bool(arg(10)&255)}})
            assert arg(2)==arg(3) and [arg(i)for i in range(5,10)]==[0,bits(1.),0,0,0];self.ret(44);return
        if pc==0x4d6350:self.calls.append({'op':'cell_changed','old':[struct.unpack('<i',W(arg(i)))[0]for i in range(2)]});self.ret(8);return
        if pc in [0x77da70,0x77d930]:
            which='Identity'if pc==0x77da70 else 'X'if arg()==0xa273b0 else 'Y'
            self.calls.append({'op':'diagnostic','value':{which:arg(1)}});self.ret(12);return
        if pc==0x5782d0 and self.row['session_mode']==3:
            self.calls.append({'op':'organization_xy','component':self.component_ids[receiver],'xy':self.vector(self.r(receiver+4)+32)})
        if pc==0x5705e0:self.calls.append({'op':'element','component':self.component_ids[receiver]});self.ret();return
        if pc==0x4ce980:self.calls.append({'op':'is_attached'});self.ret(value=int(self.row['attached']));return
        if pc==0x4ce9a0:self.calls.append({'op':'parent'});self.ret(value=self.pointers[2]);return
        if pc==0x4eeb40:self.calls.append({'op':'move_ground','actor':self.inverse[receiver],'position':[arg(),arg(1)]});self.ret(8);return
        if pc==0x577240:self.calls.append({'op':'follower_target','actor':self.inverse[self.r(receiver+4)],'position':[arg(),arg(1),arg(2)]});self.ret(12);return
        super().hook(u,pc,size,user)
    def run(self,frame):
        self.reset(0);self.calls=[];self.trace=[];self.w(0x200a0dc,frame)
        self.u.reg_write(UC_X86_REG_ECX,self.pointers[1]);self.running=True
        self.u.emu_start(0x4d0ee0,self.STOP,count=10000000);self.running=False
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
        out=self.snapshot(self.trace);out['calls']=self.calls
        out['cached_tile']=list(struct.unpack('<2i',self.u.mem_read(self.pointers[1]+0x44,8)))
        out['organization_xy']=self.vector(self.components[10]+0xbc)
        h=self.components[14];count=self.r(h+8);samples=self.r(h+4)
        out['history']={'samples':[self.vector(samples+i*12,3)for i in range(count)],'read_index':self.r(h+16),'write_index':self.r(h+20),'reset_latch':bool(self.u.mem_read(h+24,1)[0]),'last_frame':struct.unpack('<i',W(self.r(h+28)))[0]}
        return out
def main():
    source=json.loads((HERE/'native-index.json').read_text());cases=[];windows={}
    for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
      for scenario in range(64):
        base=source['cases'][scenario%16];members=json.loads(json.dumps(base['members']))
        for m in members:m['actor']['position']=list(map(bits,[5.25,5.75]if m['key']==1 or scenario&2 else[9.,7.]))
        members[0]['actor']['flags']=(0x20000400 if scenario&4 else 0x200)|(8 if scenario&1 else 0)|(1<<13 if scenario&8 else 0)|(1<<30 if scenario&16 else 0)
        members[0]['blocking']=shape(scenario%4,1.25,1.25,.75)
        members[0]['actor']['facing']=list(map(bits,[.6,.8]if scenario%4 else[0.,-1.]))
        components={'organization':10 if scenario&2 else None,'element':11 if scenario&4 else None,'mover':12 if scenario&8 else None,
            'settlement':13 if scenario&16 else None,'history':14 if scenario&32 else None,'follower_reference':members[2]['reference']if scenario%3 else 0xffffffff}
        row={'grid':base['grid'],'flags':base['flags'],'members':members,'components':components,'session_mode':3 if scenario%2 else 2,
            'world_word_48':1 if scenario%3==0 else 0,'margin_bits':bits(.5),'ground_height_bits':bits(1.25),'time_bits':bits(7.),'interval':2,
            'variation':123 if scenario%3==1 else None,'padding_bits':bits(-3. if scenario%5==0 else .3),
            'organization_xy':list(map(bits,[-7.,-8.])),'follower_offset':list(map(bits,[1.3,-2.7])),
            'attached':bool(scenario&1),'settlement_actor':2 if scenario&16 else None,
            'history':{'samples':[[0,0,0]]*4,'read_index':1,'write_index':3,'reset_latch':False,'last_frame':4},'publish':bool(scenario&32)}
        if scenario%13==0:row['history']['last_frame']=0x7ffffffe
        machine=Position(row,cw)
        if row['publish']:
            for m in members:machine.apply({'op':'insert','actor':m['key']})
        frames=[-2147483648,-2147483647,-2147483646]if scenario%13==0 else[5,6,8]
        steps=[{'frame':frame,'native':machine.run(frame)}for frame in frames]
        windows.update(machine.visited);cases.append({'precision':precision,'input':row,'steps':steps})
    pins=[]
    for a,n in sorted(windows.items()):
        raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
    out={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),
        'cases':cases,'executed_windows':pins,'seams':['malloc/free','4F0E50 constant required sampler','551140 height brush','4F5560 doodad purge','4EA260 variation lookup','555C10 paint',
        '4D6350 cell-change virtual','77DA70/77D930 trace sinks','5705E0 Element service','4CE980/4CE9A0 attachment virtuals','4EEB40 linked actor move','577240 follower target transform']}
    path=HERE/'native-position.json';path.write_text(json.dumps(out,indent=2)+'\n');print({'sequences':len(cases),'steps':len(cases)*3,'sha256':H(path.read_bytes())})
if __name__=='__main__':main()
