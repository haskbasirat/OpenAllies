"""Execute real401BD0 through AVar48 Point3 registration (C0).
The real constructors and CString implementation execute; heap/registry
publication retain the same prior explicit source-variable transport scope.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_camera_variables as V
class Machine(V.Machine):
 def register(self):
  index=self.arg(0);p=self.arg(1);assert index==len(self.rows)
  row=dict(index=index,name=self.text(self.r(p+0x1c)),kind=self.r(p+0x24),flags=self.r(p+0x28),word=self.r(p),point3=[self.r(p+o)for o in(4,8,12)])
  self.rows.append(row);var=self.alloc(0x2c);self.u.mem_write(var,bytes(self.u.mem_read(p,0x2c)));self.w(self.fast+4*index,var)
  if index==48:raise V.Done()
  self.ret(8,var)
 def run(self):
  try:self.call(0x401bd0,self.manager)
  except V.Done:pass
  assert len(self.rows)==49;return self.rows[48]
def main():
 rows=[];visited={};seams=set()
 for image,pc in itertools.product(('original','v14'),V.PRECISIONS):
  m=Machine(image,pc);row=m.run();visited.update(m.actual);seams|=set(m.seams);rows.append(dict(image=image,precision=pc,registration=row))
 pins=[]
 for va,size in sorted(visited.items()):
  if va in seams or not(0x401000<=va<0x9ef000):continue
  pins.append(dict(va=hex(va),original=V.original.range(va,size).hex(),v14=V.modified.range(va,size).hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=pins)
 path=HERE/'native-camera-render-variables.json'
 if'--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(pins),registration=rows[0]['registration'],sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
