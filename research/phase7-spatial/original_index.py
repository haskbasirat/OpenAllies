"""Run original tile list insertion/removal and world membership update bodies."""
from pathlib import Path
import json,struct,sys,hashlib,random
HERE=Path(__file__).parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from original_geometry import Geometry,original,modified,W,bits,H
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
class Index(Geometry):
    ACTORS=0x2100000;DEFS=0x2200000;TILES=0x2300000;IDS=0x2400000;HEAP=0x2500000
    def __init__(self,grid,members,flags,cw):
        super().__init__(cw);self.u.mem_map(self.ACTORS,0x100000);self.u.mem_map(self.DEFS,0x100000)
        self.u.mem_map(self.TILES,0x100000);self.u.mem_map(self.IDS,0x100000);self.u.mem_map(self.HEAP,0x100000)
        self.members=members;self.grid=grid;self.pointers={m['key']:self.ACTORS+i*0x400 for i,m in enumerate(members)}
        self.inverse={v:k for k,v in self.pointers.items()};self.heap=self.HEAP;self.trace=[];self.running=False
        self.u.hook_add(UC_HOOK_MEM_WRITE,self.written)
        self.w(0xb36534,0x200a000);self.w(0x200a030,self.STATE);self.w(0xb364c0,self.IDS)
        self.w(0xb39990,0x200c000);self.w(0x200c064,0x200c100)
        self.w(0x200c104,0);self.w(0x200c108,bits(-1.))
        self.w(self.STATE+4,grid['coordinate_shift']);self.w(self.STATE+8,grid['cell_size_bits'])
        self.w(self.STATE+28,grid['dimensions'][0]);self.w(self.STATE+32,grid['dimensions'][1]);self.w(self.STATE+40,self.TILES)
        for i,flag in enumerate(flags):self.w(self.TILES+i*48+32,flag)
        for i,m in enumerate(members):
            ptr=self.pointers[m['key']];definition=self.DEFS+i*0x400;vtable=definition+0x200
            self.u.mem_write(vtable,original.range(0xa273f0,0x180));self.w(vtable+0xb4,0x6a71d0 if m['actor_category']else 0x6a71f0)
            self.w(ptr,vtable);self.w(ptr+4,definition);self.w(ptr+8,m['actor']['flags']);self.w(ptr+20,m['reference'])
            self.u.mem_write(ptr+32,W(*m['actor']['position'],m['height_bits'],*m['actor']['facing'],0))
            self.w(ptr+0x90,0x1234 if m['wall_present']else 0)
            for offset,key in [(0x38,'blocking'),(0x50,'soft')]:
                s=m[key];self.u.mem_write(definition+offset,W(s['kind'],s['radius_bits'],*s['bounds']))
            ref=m['reference'];self.u.mem_write(self.IDS+4+(ref&65535)*2,struct.pack('<H',ref>>16))
            self.w(self.IDS+0x20004+(ref&65535)*4,ptr)
    def cell(self,ptr):
        i=(ptr-self.TILES)//48;return [i%self.grid['dimensions'][0],i//self.grid['dimensions'][0]]
    def written(self,u,access,address,size,value,_):
        if not self.running or size!=4:return
        if address-8 in self.inverse:
            self.trace.append({'action':'member_flags','actor':self.inverse[address-8],'before':self.r(address),'after':value&0xffffffff})
        elif self.TILES<=address<self.TILES+self.grid['dimensions'][0]*self.grid['dimensions'][1]*48:
            offset=(address-self.TILES)%48
            if offset in [28,32]:self.trace.append({'action':'primary'if offset==28 else'flags','cell':self.cell(address-offset),'before':self.r(address),'after':value&0xffffffff})
    def hook(self,u,pc,size,user):
        if pc in [0x6f7810,0x6f7430]:
            sp=u.reg_read(UC_X86_REG_ESP)
            if pc==0x6f7810:
                n=self.r(sp+4);ptr=self.heap;self.heap+=(n+15)&~15;assert self.heap<self.HEAP+0x100000
                u.reg_write(UC_X86_REG_EAX,ptr)
            u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4);return
        if getattr(self,'running',False) and pc in [0x54f6f0,0x54f780]:
            self.trace.append({'action':'insert'if pc==0x54f6f0 else'remove',
                'cell':self.cell(u.reg_read(UC_X86_REG_ECX)),'actor':self.inverse[self.r(u.reg_read(UC_X86_REG_ESP)+4)]})
        super().hook(u,pc,size,user)
    def placement(self,address,shape,xy,facing):
        self.u.mem_write(address,W(shape,*facing,*xy,0));return address
    def apply(self,action):
        self.reset(0);key=action['actor'];ptr=self.pointers[key];definition=self.r(ptr+4)
        xy=list(struct.unpack('<2I',self.u.mem_read(ptr+32,8)));facing=list(struct.unpack('<2I',self.u.mem_read(ptr+44,8)))
        old_b=self.placement(0x200b000,definition+0x50,xy,facing);old_a=self.placement(0x200b100,definition+0x38,xy,facing)
        if action['op']=='retire_reference':
            ref=self.r(ptr+20);self.w(self.IDS+0x20004+(ref&65535)*4,0);return self.snapshot([])
        if action['op']=='insert':address=0x5536b0;args=[ptr,old_b,old_a]
        elif action['op']=='remove':address=0x553830;args=[ptr,old_b,old_a]
        else:
            newxy=action['position'];newfacing=action.get('facing',facing)
            new_b=self.placement(0x200b200,definition+0x50,newxy,newfacing);new_a=self.placement(0x200b300,definition+0x38,newxy,newfacing)
            address=0x5539b0;args=[ptr,old_b,new_b,old_a,new_a]
        for i,arg in enumerate(args):self.w(self.STACK+4+i*4,arg)
        self.trace=[];self.running=True;self.u.emu_start(address,self.STOP,count=10000000);self.running=False
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4+4*len(args)
        #Prepared index-update fixture: caller position commit is outside these
        #native bodies, and is explicit input state for the following operation.
        if action['op']=='update':
            self.u.mem_write(ptr+32,W(*action['position']));self.u.mem_write(ptr+44,W(*action.get('facing',facing)))
        return self.snapshot(self.trace)
    def snapshot(self,trace):
        tiles=[]
        for i in range(self.grid['dimensions'][0]*self.grid['dimensions'][1]):
            ptr=self.TILES+i*48;node=self.r(ptr+8);actors=[];last=0
            while node:actors.append(self.inverse[self.r(node)]);last=node;node=self.r(node+4)
            assert last==self.r(ptr+12)
            tiles.append({'actors':actors,'flags20':self.r(ptr+32),'primary_reference':self.r(ptr+28)})
        return {'tiles':tiles,'member_flags':{str(k):self.r(p+8)for k,p in self.pointers.items()},'trace':trace}

def shape(kind,radius,hx,hy):return {'kind':kind,'radius_bits':bits(radius),'bounds':list(map(bits,[-hx,-hy,hx,hy]))}
def main():
    grid={'dimensions':[8,7],'coordinate_shift':1,'cell_size_bits':bits(2.)};flags=[0x200]*56;cases=[];windows={}
    for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
      for scenario in range(16):
        members=[]
        for i in range(3):
            large=scenario&1 or i!=0
            members.append({'key':i+1,'reference':((i+2)<<16)|(i+1),'actor':{'definition':i+10,'flags':0x20000400 if large else 0x200,'position':list(map(bits,[5.,5.])),'facing':list(map(bits,[0.,-1.]))},'height_bits':bits(2.),'facing_z_bits':0,'cached_tile':[0,0],
                'blocking':shape(1 if i==0 else 3,1.25 if large else .25,1.25,.75),
                'soft':shape(3,3.,2.5,2.5),'target':shape(3,3.,2.5,2.5),'actor_category':not(scenario&8 and i==1),'wall_present':bool(scenario&2 and i>0)})
        actions=[{'op':'insert','actor':1},{'op':'insert','actor':2},{'op':'insert','actor':3}]
        if scenario&4:actions +=[{'op':'insert','actor':2},{'op':'retire_reference','actor':2}]
        target=[5.,5.]if scenario%4==0 else[6.,6.]if scenario%4==1 else[11.,9.]if scenario%4==2 else[-2.,13.]
        actions +=[{'op':'update','actor':1,'position':list(map(bits,target))},
            {'op':'update','actor':2,'position':list(map(bits,[6.,5.])),'facing':list(map(bits,[.6,.8]))},
            {'op':'remove','actor':3},{'op':'remove','actor':2},{'op':'remove','actor':1}]
        machine=Index(grid,members,flags,cw);steps=[]
        for action in actions:steps.append({'input':action,'native':machine.apply(action)})
        windows.update(machine.visited);cases.append({'precision':pc,'grid':grid,'flags':flags,'members':members,'steps':steps})
    pins=[]
    for a,n in sorted(windows.items()):
        raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
    out={'schema':1,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),
        'scope':'Original5536B0/553830/5539B0, tile lists/primary references/blocker queries with actual ordinary shape getters and native geometry. Only malloc/free are intercepted. Update actions explicitly apply caller pose state after native membership function returns; they do not claim full actor callback execution.',
        'cases':cases,'executed_windows':pins}
    path=HERE/'native-index.json';path.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'sequences':len(cases),'steps':sum(len(c['steps'])for c in cases),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
