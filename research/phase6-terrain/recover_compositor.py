"""Read-only native terrain compositor matrix/state and authored source recovery."""
from pathlib import Path
import json, re, struct, sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
DATA=ROOT.parent/'reconstruction/assets/original/Data'

def strip(raw): return re.sub(r';[^\r\n]*','',re.sub(r'/\*.*?\*/','',raw,flags=re.S))
def field(raw,name):
    m=re.search(r'^\s*'+name+r'\s*=\s*([^\r\n;]+)',raw,re.M|re.I)
    return m[1].strip() if m else None

def metadata():
    sources=[]; variants={}; biomes={}; sets={}; terrain_defaults={}; biome_terrains={}
    template=DATA/'Templates/template_terrain.tgi'
    raw_template=strip(template.read_text(encoding='cp1252'))
    templates={}
    for block in re.split(r'\[template\s+',raw_template,flags=re.I)[1:]:
        name=block.split(']',1)[0].split()[0].lower()
        templates[name]=block.split(']',1)[1]
    defaults={}
    for parent in ['terrain','mountainterrain','waterterrain']:
        defaults[parent]={}
        for flag in ['land','water']:
            match=re.search(r'^\s*(?:bool\s+)?'+flag+r'\s*=\s*(true|false)\b',templates[parent],re.M|re.I)
            assert match,(parent,flag)
            defaults[parent][flag]=match[1].lower()=='true'
    for p in sorted((DATA/'Terrain').rglob('*.tgi')):
        raw=strip(p.read_text(encoding='cp1252'))
        source={'path':p.relative_to(DATA).as_posix(),'sha256':sha(p.read_bytes())}
        if re.search(r'\[Terrain\s+template=',raw,re.I):
            base=raw.split('[Variation]',1)[0]
            parent=re.search(r'\[Terrain\s+template=(\w+)\]',raw,re.I)[1].lower()
            flags={}
            for flag in ['land','water']:
                value=field(base,flag)
                assert value is None or value.lower() in ['true','false'],(p,flag,value)
                flags[flag]=defaults[parent][flag] if value is None else value.lower()=='true'
            priority=int(field(base,'base_priority'))
            for index,section in enumerate(re.split(r'\[Variation\]',raw,flags=re.I)[1:]):
                ident=field(section,'IDS')
                assert ident and ident not in variants
                texture=field(section,'texture')
                relative=(p.parent/texture).relative_to(DATA).with_suffix('.png').as_posix()
                variants[ident]={'canonical':ident,'priority':priority+index,
                    'terrain_id':field(base,'IDS'),'terrain_template':parent,**flags,
                    'texture':'/assets/converted/textures/'+relative,
                    'width':float(field(section,'texture_width') or 8),
                    'height':float(field(section,'texture_height') or 8),**source}
                if index==0:terrain_defaults[field(base,'IDS')]=ident
            sources.append(source)
        if re.search(r'\[Biome\s+template=',raw,re.I):
            ident=field(raw,'IDS')
            biomes[ident]=re.findall(r'^\s*set_IDS\s*=\s*(\S+)',raw,re.M|re.I)
            biome_terrains[ident]=re.findall(r'^\s*terrain_IDS\s*=\s*(\S+)',raw,re.M|re.I)
            sources.append(source)
        if re.search(r'\[TransitionSet\s+template=',raw,re.I):
            sets[field(raw,'IDS')]='Terrain/Transitions/'+field(raw,'path_prefix')
            sources.append(source)
    translation_path=DATA/'translate.tgi'
    translations={'variation':{},'biome':{}}
    for block in re.split(r'\[Translate\s+template=Translate\]',strip(translation_path.read_text(encoding='cp1252')),flags=re.I)[1:]:
        node=(field(block,'node') or '').lower()
        if node in translations:
            alias,target=field(block,'ids_old'),field(block,'ids_new')
            assert alias and target
            assert alias not in translations[node] or translations[node][alias]==target,(node,alias)
            translations[node][alias]=target
    resolved_aliases={};unresolved_aliases={}
    biome_defaults={}
    for ident,terrains in biome_terrains.items():
        assert terrains and terrains[0] in terrain_defaults,(ident,terrains)
        biome_defaults[ident]={'terrain_id':terrains[0],'variation_id':terrain_defaults[terrains[0]]}
    for node,objects in [('variation',variants),('biome',biomes)]:
        resolved_aliases[node]={};unresolved_aliases[node]={}
        authored=set(objects)
        for alias in translations[node]:
            target=alias;chain=[]
            while target not in authored and target in translations[node]:
                assert target not in chain,(node,alias,chain)
                chain.append(target);target=translations[node][target]
            chain.append(target)
            if target in authored:
                objects[alias]=dict(objects[target]) if node=='variation' else list(objects[target])
                resolved_aliases[node][alias]=chain
                if node=='biome':biome_defaults[alias]=dict(biome_defaults[target])
            else:
                unresolved_aliases[node][alias]=chain
    report={'schema':1,'variants':variants,'biomes':biomes,'transition_sets':sets,'sources':sources,
            'biome_defaults':biome_defaults,
            'default_evidence':'Biome Terrain child append order4B8F28..4B9015; first pointer4B97B9..4B97C1 to+78. Terrain Variation append order4B8D00..4B8DCF; first pointer4B8E12..4B8E21 to+30.',
            'translation_source':{'path':translation_path.relative_to(DATA).as_posix(),'sha256':sha(translation_path.read_bytes())},
            'resolved_aliases':resolved_aliases,'unresolved_aliases':unresolved_aliases,
            'terrain_template':{'path':template.relative_to(DATA).as_posix(),'sha256':sha(template.read_bytes())},
            'passability_defaults':defaults,
            'texels_per_meter':16,'texels_evidence':'0x41FA74 pushdefault16 /0x41FA92 slot1 TerrainTextureTexelsPerMeter',
            'priority_evidence':'0x4B8D22 adds terrain basepriority to appendordinal; template requiresuniquepriorities.'}
    (HERE/'terrain-render-metadata.json').write_text(json.dumps(report,indent=2))
    return report

def matrix_cases():
    table=json.loads((ROOT/'research/phase5-terrain/mask-lookup.json').read_text())['table']
    transforms={m['transform']:m for m in table if m}
    cases=[]
    for transform in transforms:
      for pixels in [2,16,32,64,128]:
        m=Machine(); u=m.u; m.reset(0)
        manager,avars,entry,layer,target=0x2002000,0x2003000,0x2004000,0x2005000,0x2006000
        m.w(0xb36518,manager);m.w(manager+0xb0,2)
        m.w(0xb39940,avars);m.w(avars+4,entry);m.w(entry,pixels//2)
        m.w(layer+0x1c,transform);m.w(layer+0x18,target)
        u.reg_write(UC_X86_REG_EBX,layer);u.reg_write(UC_X86_REG_EDI,0)
        u.emu_start(0x72a3b4,0x72a473,count=10000)
        assert u.reg_read(UC_X86_REG_EIP)==0x72a473
        matrix=list(struct.unpack('<16f',u.mem_read(target+0x20,64)))
        raw=transforms[transform]['matrix'];s=(pixels-1)/pixels;h=.5/pixels
        expected=[0.0]*16
        for i in [0,1,2,4,5,6]:expected[i]=raw[i]*s
        for j in range(3):expected[8+j]=(raw[j]+raw[4+j])*h+raw[8+j]
        expected[15]=1.0
        assert matrix==expected,(transform,pixels,matrix,expected)
        cases.append({'transform':transform,'pixels':pixels,'matrix':matrix})
    return cases

class Selector(Machine):
    def select(self,x,y,width,count):
        self.reset(0)
        manager,biome,sets,block,grid=0x2002000,0x2003000,0x2004000,0x2005000,0x2006000
        self.w(0xb36518,manager);self.w(manager+0xb0,2);self.w(manager+0x74,width*2)
        self.w(manager+0x70,biome);self.w(biome+0x7c,sets);self.w(biome+0x80,count)
        self.w(block+0xd0,8);self.w(block+0xac,x//8);self.w(block+0xb0,y//8)
        self.w(grid,x%8+1);self.w(grid+4,y%8+1)
        for i in range(count):
            s=0x2007000+i*0x100
            self.w(sets+i*4,s)
        self.w(self.STACK+4,0);self.w(self.STACK+8,grid);self.w(self.STACK+12,block)
        self.u.emu_start(0x7296b0,0x72971a,count=1000)
        return (self.u.reg_read(UC_X86_REG_ECX)-0x2007000)//0x100

def state_cases():
    cases=[]
    for layer in [0,1,4,8]:
      for flags in [0xec,0xed,0,2,0x32,0x132]:
        m=Machine();m.reset(0);u=m.u;states=[]
        props,alpha,ref,material,state=0x2002000,0x2003000,0x2004000,0x2005000,0x2006000
        m.w(m.STACK+0x18,layer);m.w(m.STACK+0x84,props)
        m.w(props+8,alpha);m.w(alpha+0x20,flags);m.w(ref,material);m.w(material+0x38,state)
        u.reg_write(UC_X86_REG_EBP,ref)
        def hook(u,address,size,data):
            if address==0x8a20c0:
                esp=u.reg_read(UC_X86_REG_ESP)
                states.append([m.r(esp+4),m.r(esp+8)])
                u.reg_write(UC_X86_REG_EIP,m.r(esp));u.reg_write(UC_X86_REG_ESP,esp+16)
        u.hook_add(UC_HOOK_CODE,hook)
        u.emu_start(0x72a53a,0x72a61c,count=1000)
        cases.append({'layer':layer,'alpha_flags':flags,'states':states})
    stages=[]
    for start,end in [(0x72a74b,0x72a7c7),(0x72a822,0x72a8a4)]:
        m=Machine();m.reset(0);u=m.u;calls=[]
        u.reg_write(UC_X86_REG_ESI,0x2002000);u.reg_write(UC_X86_REG_EBX,0)
        def hook(u,address,size,data):
            pops={0x89ff80:12,0x8a07e0:4,0x8a0780:8,0x8a0830:4}
            if address in pops:
                esp=u.reg_read(UC_X86_REG_ESP);n=pops[address]
                calls.append({'function':hex(address),'args':[m.r(esp+i+4)for i in range(0,n,4)]})
                u.reg_write(UC_X86_REG_EIP,m.r(esp));u.reg_write(UC_X86_REG_ESP,esp+4+n)
        u.hook_add(UC_HOOK_CODE,hook);u.emu_start(start,end,count=1000)
        stages.append({'start':hex(start),'calls':calls})
    return cases,stages

def main():
    metadata()
    ranges={'matrix':(0x72a3b4,0x72a473),'selector':(0x7296b0,0x729759),
            'blend_state':(0x72a53a,0x72a61c),'stage_setup':(0x72a650,0x72a949),
            'density_default':(0x41fa74,0x41faa6)}
    for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a)
    selectors=[];m=Selector()
    for width in [32,128,256,512]:
      for x,y in [(0,0),(7,7),(8,8),(17,21),(width-1,width-1)]:
       for count in [1,2,3,7]:
        actual=m.select(x,y,width,count)
        assert actual==((y*width+x)*4723+6323)%6997%count
        selectors.append({'cell':[x,y],'width':width,'count':count,'index':actual})
    states,stages=state_cases()
    report={'schema':1,'passed':True,'original_sha256':sha(original.b),'accepted_sha256':sha(modified.b),
            'producer_sha256':sha(Path(__file__).read_bytes()),'matrix_cases':matrix_cases(),'selector_cases':selectors,
            'render_state_cases':states,'texture_stages':stages,
            'ranges':{n:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for n,(a,b) in ranges.items()}}
    (HERE/'compositor-fixtures.json').write_text(json.dumps(report,indent=2))
    print(json.dumps({'matrices':len(report['matrix_cases']),'selectors':len(selectors)}))
if __name__=='__main__':main()
