"""Original prepared actor query and553BA0 exact first-hit iterator.
Allocation/free, source/custom exclusion and definition-view virtual results are explicit fixture leaves.
Original descriptor setup, snapping, world bounds, tile/actor scans, all shape
intersections and intrusive marks/cleanup execute unchanged x86.
"""
from pathlib import Path
import json,struct
from original_index import Index,shape,bits,W,H,original,modified
from unicorn.x86_const import *
HERE=Path(__file__).parent
class Query(Index):
    QUERY=0x200e000;OUT=0x200e100;CUSTOM=0x200e700
    def __init__(self,row,cw):
        super().__init__(row['grid'],row['members'],row['flags'],cw);self.row=row;self.calls=[];self.u.mem_map(0,0x1000)
        self.u.mem_write(self.STATE+12,W(*row['extent_bits']))
        self.u.mem_write(0x200e710,b'\x8b\xc1\xc3');self.u.mem_write(0x200e720,b'\x31\xc0\xc3')
        self.reset(0);self.u.reg_write(UC_X86_REG_ECX,self.STATE+0x30);self.w(self.STACK+4,row['grid']['cell_size_bits'])
        self.u.emu_start(0x46eb60,self.STOP,count=10000)
        self.geometry={'extent_bits':row['extent_bits'],'cell_shape':self.shape(self.STATE+0x30)}
        for m in row['members']:
            d=self.r(self.pointers[m['key']]+4);definition=row['definitions'][str(m['actor']['definition'])]
            self.w(d,d+0x380);self.w(d+0x3b0,0x200e710 if definition['object_data']else 0x200e720)
            self.w(d+0x1c,definition['flags']);self.u.mem_write(d+0x1a8,bytes([definition['flag_1a8']]))
        for m in row['members']:self.apply({'op':'insert','actor':m['key']})
        for actor in row['retired']:self.apply({'op':'retire_reference','actor':actor})
        for actor in row['pre_marked']:self.w(self.pointers[actor]+0x10,self.pointers[actor])
        self.reset(0);self.u.reg_write(UC_X86_REG_ECX,self.QUERY);self.u.emu_start(0x552dd0,self.STOP,count=10000)
        self.reset(0);self.u.reg_write(UC_X86_REG_ECX,self.QUERY);self.w(self.STACK+4,self.pointers[1]);self.w(self.STACK+8,int(row['strict']))
        self.u.emu_start(0x5531f0,self.STOP,count=10000)
        self.w(self.QUERY+0x38,self.r(self.QUERY+0x38)|row['extra_flags']);self.w(self.QUERY+0x3c,self.r(self.QUERY+0x3c)|row['extra_flags'])
        initial=self.prepared()
        self.w(self.QUERY+0x20,self.pointers[row['ignore_actor']]if row['ignore_actor']is not None else 0)
        self.w(self.QUERY+0x24,self.r(self.pointers[row['ignore_definition_actor']]+4)if row['ignore_definition_actor']is not None else 0)
        self.u.mem_write(self.QUERY+0x30,bytes([row['ignore_all']]))
        self.w(self.QUERY+0x34,self.CUSTOM if row['custom_filter']else 0)
        self.prepared_initial=initial
    def shape(self,ptr):
        w=list(struct.unpack('<6I',self.u.mem_read(ptr,24)))
        return {'kind':w[0],'radius_bits':w[1],'bounds':w[2:]}
    def ret(self,n=0,value=0):
        sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value)
        self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
    def hook(self,u,pc,size,user):
        if pc in [0x4d5600,self.CUSTOM]:
            sp=u.reg_read(UC_X86_REG_ESP)
            if pc==0x4d5600:
                actor=self.inverse[self.r(sp+4)];source=self.inverse[u.reg_read(UC_X86_REG_ECX)];descriptor=u.reg_read(UC_X86_REG_ESI)
                self.calls.append({'op':'source_excludes','source':source,'actor':actor,'mask':self.r(descriptor+0x3c)})
                self.ret(4,int(source==actor or actor in self.row['excluded']));return
            descriptor=self.r(sp+4);actor=self.inverse[self.r(sp+8)]
            self.calls.append({'op':'custom_filter','actor':actor,'mask':self.r(descriptor+0x3c)})
            self.ret(value=int(actor==3));return
        super().hook(u,pc,size,user)
    def prepared(self):
        return {'placement':{'shape':self.shape(self.QUERY),'facing':[self.r(self.QUERY+0x44),self.r(self.QUERY+0x48)],'position':[self.r(self.QUERY+0x4c),self.r(self.QUERY+0x50)]},
            'required_actor_flags':self.r(self.QUERY+0x38),'mask':self.r(self.QUERY+0x3c)}
    def run(self,xy):
        self.reset(0);self.calls=[];self.u.reg_write(UC_X86_REG_ECX,self.QUERY);self.u.mem_write(self.STACK+4,W(*xy))
        self.u.emu_start(0x552250,self.STOP,count=10000)
        prepared=self.prepared()
        self.reset(0);self.u.reg_write(UC_X86_REG_ECX,self.STATE);self.w(self.STACK+4,self.OUT);self.w(self.STACK+8,self.QUERY)
        self.u.emu_start(0x553ba0,self.STOP,count=1000000)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
        words=list(struct.unpack('<4I',self.u.mem_read(self.OUT,16)))
        links={str(key):self.inverse.get(self.r(ptr+0x10))for key,ptr in self.pointers.items()}
        return {'prepared':prepared,'hit':{'kind':words[0],'reference':words[1],'cell':[struct.unpack('<i',W(x))[0]for x in words[2:]]},'calls':self.calls,'links':links}
def main():
    cases=[];windows={};grid={'dimensions':[8,7],'coordinate_shift':1,'cell_size_bits':bits(2.)}
    for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
      for scenario in range(128):
        members=[];definitions={}
        for i in range(6):
            key=i+1;kind=(scenario+i)%4;radius=0 if kind==0 else 1.5;flags=0x20000400 if i>0 else 0x400
            if i==0:flags|=(1<<12 if scenario&1 else 0)|(1<<18 if scenario&2 else 0)|(1<<21 if scenario&4 else 0)
            if i==4:flags=0x400
            xy=[5.,5.]if i<3 else[7.,5.]if i<5 else[9.,9.]
            facing=[.6,.8]if scenario&8 and i%2 else[0.,-1.]
            members.append({'key':key,'reference':((i+2)<<16)|key,'actor':{'definition':key+10,'flags':flags,'position':list(map(bits,xy)),'facing':list(map(bits,facing))},
                'height_bits':bits(2.),'facing_z_bits':0,'cached_tile':[0,0],'blocking':shape(kind,radius,1.25,.75),'soft':shape(3,3.,2.5,2.5),'target':shape(kind,radius,1.25,.75),'actor_category':True,'wall_present':bool(scenario&16 and i==1)})
            definitions[str(key+10)]={'flags':flags|(1<<16 if i==0 and scenario&32 else 0),'object_data':not(scenario%9==0),'flag_1a8':bool(i==2 and scenario&16)}
            if i==0 and scenario&64:definitions[str(key+10)]['flags']&=~(1<<10)
        row={'grid':grid,'extent_bits':list(map(bits,[17. if scenario&16 else 16.,15. if scenario&16 else 14.])),
            'flags':[0x300 if scenario%3==0 and x in [2,3]and y==2 else 0x200 for y in range(7)for x in range(8)],
            'members':members,'definitions':definitions,'strict':bool(scenario%2),'extra_flags':0x400,
            'ignore_actor':2 if scenario%6==1 else None,'ignore_definition_actor':3 if scenario%6==2 else None,
            'ignore_all':bool(scenario%8==3),'custom_filter':bool(scenario%7==4),'excluded':[2]if scenario%5==0 else[],
            'pre_marked':[3]if scenario%4==2 else[],'retired':[2]if scenario%7==3 else[]}
        m=Query(row,cw);points=[[5.,5.],[6.25,6.75],[0.,0.],[-.25,5.],[15.9,13.9],[14.99,12.99]]
        steps=[{'point':list(map(bits,point)),'native':m.run(list(map(bits,point)))}for point in points]
        windows.update(m.visited);cases.append({'precision':precision,'input':row,'geometry':m.geometry,'initial':m.prepared_initial,'steps':steps})
    pins=[]
    for a,n in sorted(windows.items()):
        raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
    out={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'accepted_sha256':H(modified.b),'cases':cases,'executed_windows':pins}
    p=HERE/'native-query.json';p.write_text(json.dumps(out,indent=2)+'\n');print({'sequences':len(cases),'queries':sum(len(c['steps'])for c in cases),'sha256':H(p.read_bytes())})
if __name__=='__main__':main()
