"""Bounded real Windows/v14 Ni shared-default constructor observation.
Fork of the proved grid watch helper. Only a new isolated QA debuggee is
instrumented; stop at8C3F20 after actual12 shared-default constructors.
All four calibration MOVs run after the sealed observation cutoff, and
are transport checks rather than game producers. No native FP state rewritten.
"""
from pathlib import Path
import ctypes as C
from ctypes import wintypes as W
import argparse
import hashlib
import json
import os
import shutil
import struct
import time
import traceback

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
GAME = ROOT.parent
NATIVE = GAME / 'reconstruction/native-game/runtime-expanded-v14'
EXPECTED = '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'


def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


class STARTUP(C.Structure):
    _fields_ = [('cb', W.DWORD), ('reserved', W.LPWSTR), ('desktop', W.LPWSTR), ('title', W.LPWSTR),
        *[(n, W.DWORD) for n in ['x','y','xs','ys','xc','yc','fill','flags']],
        ('show', W.WORD), ('reserved2n', W.WORD), ('reserved2', C.c_void_p),
        ('stdin', W.HANDLE), ('stdout', W.HANDLE), ('stderr', W.HANDLE)]


class PROCESS(C.Structure):
    _fields_ = [('process', W.HANDLE), ('thread', W.HANDLE), ('pid', W.DWORD), ('tid', W.DWORD)]


class FLOAT(C.Structure):
    _fields_ = [(n, W.DWORD) for n in ['ControlWord','StatusWord','TagWord','ErrorOffset','ErrorSelector','DataOffset','DataSelector']] + [('registers', C.c_ubyte*80),('Cr0NpxState',W.DWORD)]


class CONTEXT(C.Structure):
    _fields_ = [('flags',W.DWORD)] + [(n,W.DWORD) for n in ['Dr0','Dr1','Dr2','Dr3','Dr6','Dr7']] + [('floating',FLOAT)] + [(n,W.DWORD) for n in ['SegGs','SegFs','SegEs','SegDs','Edi','Esi','Ebx','Edx','Ecx','Eax','Ebp','Eip','SegCs','EFlags','Esp','SegSs']] + [('extended',C.c_ubyte*512)]


class EVENT(C.Structure):
    _fields_ = [('code',W.DWORD),('pid',W.DWORD),('tid',W.DWORD),('padding',W.DWORD),('payload',C.c_ubyte*160)]


def prepare(folder):
    assert not folder.exists(), 'Use a fresh capture directory'
    assert sha(NATIVE/'AA.exe') == EXPECTED
    folder.mkdir(parents=True)
    originals = {}
    for source in NATIVE.rglob('*'):
        if not source.is_file():
            continue
        relative = source.relative_to(NATIVE)
        # Logs, user profiles and save files are intentionally not imported.
        if any(part.lower() in ['logs','userdata','saves','saved games','profiles'] for part in relative.parts):
            continue
        dest = folder/'runtime'/relative
        dest.parent.mkdir(parents=True, exist_ok=True)
        if source.suffix.lower() == '.rwd':
            originals[str(source)] = sha(source)
        shutil.copy2(source, dest)
        if source.suffix.lower() == '.rwd':
            assert not os.path.samefile(source, dest)
            dest.chmod(0o444)
    for name in ['personal','common','other']:
        (folder/name).mkdir()
    return originals


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--folder',type=Path,required=True)
    parser.add_argument('--seconds',type=int,default=90)
    parser.add_argument('--words',type=int,nargs=4,required=True)
    args=parser.parse_args()
    assert len(set(args.words))==4 and all(0<=i<5 for i in args.words)
    seed_addresses=[0xb43e68,0xb43e6c,0xb43e70,0xb45f54,0xb323c0]
    watch_addresses=[seed_addresses[i] for i in args.words]
    folder=args.folder.resolve()
    assert ROOT.resolve() in folder.parents
    original_hashes=prepare(folder)
    kernel=C.WinDLL('kernel32',use_last_error=True)
    def api(name,restype,argtypes):
        f=getattr(kernel,name);f.restype=restype;f.argtypes=argtypes;return f
    create=api('CreateProcessW',W.BOOL,[W.LPCWSTR,W.LPWSTR,C.c_void_p,C.c_void_p,W.BOOL,W.DWORD,C.c_void_p,W.LPCWSTR,C.POINTER(STARTUP),C.POINTER(PROCESS)])
    wait=api('WaitForDebugEvent',W.BOOL,[C.POINTER(EVENT),W.DWORD])
    cont=api('ContinueDebugEvent',W.BOOL,[W.DWORD,W.DWORD,W.DWORD])
    get=api('Wow64GetThreadContext',W.BOOL,[W.HANDLE,C.POINTER(CONTEXT)])
    put=api('Wow64SetThreadContext',W.BOOL,[W.HANDLE,C.POINTER(CONTEXT)])
    read=api('ReadProcessMemory',W.BOOL,[W.HANDLE,C.c_void_p,C.c_void_p,C.c_size_t,C.POINTER(C.c_size_t)])
    write=api('WriteProcessMemory',W.BOOL,[W.HANDLE,C.c_void_p,C.c_void_p,C.c_size_t,C.POINTER(C.c_size_t)])
    flush=api('FlushInstructionCache',W.BOOL,[W.HANDLE,C.c_void_p,C.c_size_t])
    suspend=api('SuspendThread',W.DWORD,[W.HANDLE])
    resume=api('ResumeThread',W.DWORD,[W.HANDLE])
    terminate=api('TerminateProcess',W.BOOL,[W.HANDLE,W.UINT])
    close=api('CloseHandle',W.BOOL,[W.HANDLE])
    join=api('WaitForSingleObject',W.DWORD,[W.HANDLE,W.DWORD])
    def check(ok):
        if not ok: raise C.WinError(C.get_last_error())
    assert C.sizeof(CONTEXT)==716 and C.sizeof(EVENT)==176
    process=PROCESS();startup=STARTUP();startup.cb=C.sizeof(startup);startup.flags=1;startup.show=0
    executable=folder/'runtime/AA.exe'
    command=C.create_unicode_buffer('"'+str(executable)+'"')
    check(create(str(executable),command,None,None,False,2,None,str(executable.parent),C.byref(startup),C.byref(process)))
    # Debug-event process/thread handles are closed by Windows on continued
    # EXIT events. Own duplicates instead, including a stable process wait handle.
    duplicate=api('DuplicateHandle',W.BOOL,[W.HANDLE,W.HANDLE,W.HANDLE,C.POINTER(W.HANDLE),W.DWORD,W.BOOL,W.DWORD])
    current=api('GetCurrentProcess',W.HANDLE,[])
    image_name=api('QueryFullProcessImageNameW',W.BOOL,[W.HANDLE,W.DWORD,W.LPWSTR,C.POINTER(W.DWORD)])
    exit_code=api('GetExitCodeProcess',W.BOOL,[W.HANDLE,C.POINTER(W.DWORD)])
    def duplicate_owned(handle):
        owned=W.HANDLE();check(duplicate(current(),handle,current(),C.byref(owned),0,False,2));return owned.value
    owned_process=duplicate_owned(process.process)
    def verify_owned_image():
        name=C.create_unicode_buffer(32768);size=W.DWORD(len(name));check(image_name(owned_process,0,name,C.byref(size)))
        assert Path(name.value).resolve()==executable.resolve(), 'Owned process image identity mismatch'
        return name.value
    verified_image=verify_owned_image()
    threads={};points={};stepping={};events=[];captures=[];manager=None;completed=False
    global_captures=[];watch_arms=[];watch_writes=[];probes=[];session_completed=False
    probe_state=None;observation_cutoff=None;grid_completed=False;grid_pointer=None
    pending=None;original_error=None;cleanup_errors=[];cleanup_events=[];manual_suspended={};process_exited=False
    get_context_flags=0x1003f
    set_context_flags=0x10001 # CONTROL only: never rewrite observed FP/SSE state.
    def memory(address,length):
        b=C.create_string_buffer(length);done=C.c_size_t();check(read(owned_process,address,b,length,C.byref(done)));assert done.value==length;return b.raw
    def word(address):return struct.unpack('<I',memory(address,4))[0]
    def change(address,bytes_):
        done=C.c_size_t();b=C.create_string_buffer(bytes_);check(write(owned_process,address,b,len(bytes_),C.byref(done)));assert done.value==len(bytes_);check(flush(owned_process,address,len(bytes_)))
    def breakpoint(address,label):
        if address not in points:
            points[address]=(memory(address,1),label);change(address,b'\xcc')
    def context(tid):
        c=CONTEXT();c.flags=get_context_flags;check(get(threads[tid],C.byref(c)));return c
    def put_control(tid,c):
        c.flags=set_context_flags;check(put(threads[tid],C.byref(c)))
    def put_debug(tid,c):
        c.flags=0x10010;check(put(threads[tid],C.byref(c)))
    def globals_snapshot(label,tid,c=None,**extra):
        pointer=word(0xb36550)
        row={'label':label,'tid':tid,'seed_words':[word(a) for a in seed_addresses],
             'session_pointer':pointer,'session_phase':word(pointer+0xf8) if pointer else None,**extra}
        if c is not None:
            row.update({'eip':c.Eip,'dr6':c.Dr6,'dr7':c.Dr7,'ecx':c.Ecx,'esi':c.Esi,'eax':c.Eax,
                        'raw_context':bytes(c).hex()})
        global_captures.append(row)
        return row
    def arm_watches(tid):
        c=context(tid)
        assert c.Dr7&0xff==0,'A new owned thread already has active data watches'
        for i,address in enumerate(watch_addresses):setattr(c,'Dr'+str(i),address)
        c.Dr6=0;c.Dr7=0x400
        for i in range(4):c.Dr7|=(1<<(2*i))|(0xd<<(16+4*i)) # local, WRITE, aligned4bytes
        put_debug(tid,c)
        observed=context(tid)
        assert [getattr(observed,'Dr'+str(i)) for i in range(4)]==watch_addresses
        assert observed.Dr7&0xffff00ff==c.Dr7&0xffff00ff
        assert observed.Dr6&0xf==0
        watch_arms.append({'tid':tid,'addresses':watch_addresses,'requested_dr7':c.Dr7,
                          'observed_dr7':observed.Dr7,'observed_dr6':observed.Dr6})
    def start_probe(tid,index):
        nonlocal probe_state
        c=context(tid);address=watch_addresses[index];value=word(address)
        # Exact original constructor MOV [ESI+F8],EBX. Only after the sealed
        # real-observation cutoff, execute one same-value instruction in the
        # owned QA process. INTEGER|CONTROL never writes FP/SSE/debug fields.
        assert memory(0x76fae1,6)==bytes.fromhex('899ef8000000')
        c.Esi=address-0xf8;c.Ebx=value;c.Eip=0x76fae1;c.EFlags|=0x100
        c.flags=0x10003;check(put(threads[tid],C.byref(c)))
        probe_state={'index':index,'tid':tid,'address':address,'value':value}
    def continue_pending(status=0x10002):
        nonlocal pending
        if pending is not None:
            e=pending;check(cont(e.pid,e.tid,status));pending=None
    def wide(address):
        raw=bytearray()
        for offset in range(0,1024,2):
            pair=memory(address+offset,2)
            if pair==b'\0\0':return raw.decode('utf-16-le')
            raw.extend(pair)
        raise ValueError('unterminated source IDS')
    def snapshot(c,label,tid):
        legacy_cw=c.floating.ControlWord&0xffff;legacy_sw=c.floating.StatusWord&0xffff
        fx_cw=struct.unpack_from('<H',bytes(c.extended),0)[0];fx_sw=struct.unpack_from('<H',bytes(c.extended),2)[0]
        row={'label':label,'thread_id':tid,'eip':c.Eip,'esp':c.Esp,
             'context_flags_returned':c.flags,'legacy_x87_control':legacy_cw,'legacy_x87_status':legacy_sw,
             'fxsave_x87_control':fx_cw,'fxsave_x87_status':fx_sw,'legacy_fxsave_match':legacy_cw==fx_cw and legacy_sw==fx_sw,
             'raw_legacy_float_save':bytes(c.floating).hex(),'raw_fxsave':bytes(c.extended).hex(),
             'raw_wow64_context':bytes(c).hex(),
             'x87_control':legacy_cw,'x87_status':legacy_sw,
             'mxcsr':struct.unpack_from('<I',bytes(c.extended),24)[0]}
        # Retain the raw evidence BEFORE any equality or pointer dereference can
        # fail. A mismatch is not silently resolved by choosing a preferred view.
        captures.append(row)
        assert row['legacy_fxsave_match'], f'Legacy/FXSAVE mismatch: CW {legacy_cw:#x}/{fx_cw:#x}, SW {legacy_sw:#x}/{fx_sw:#x}'
        row['crt_sse2_global']=word(0xb4b340)
        row['globals']={'assertion_guard':memory(0xb3b4f8,1)[0], 'csv_guard':word(0xb3b4f4),'ranged_guard':word(0xb3a66c)}
        table=word(0xb39ab0)
        row['settings_table']=table
        row['settings']={f'{off:x}':{'pointer':word(table+off),'bits':word(word(table+off))} for off in [0xa4,0x144,0x148,0x14c,0x150,0x154,0x158,0x15c]}
        if row['globals']['csv_guard']&1:
            cached=word(0xb3b4f0);row['csv_cached_pointer']=cached
            row['csv_length']=word(word(cached+0x10)-0xc)
        if row['globals']['ranged_guard']&1:
            row['ranged_cached']={f'{a:x}':{'pointer':word(a),'bits':word(word(a))} for a in [0xb3a668,0xb3a664]}
        if manager is not None:
            row['manager_means']={f'{offset:x}':word(manager+offset) for offset in range(0x74,0x94,4)}
            row['stream_object_bytes']=memory(manager+0x4c,0x30).hex()
    start=time.monotonic()
    try:
        while time.monotonic()-start<args.seconds:
            e=EVENT()
            if not wait(C.byref(e),250):
                if C.get_last_error()==121:continue
                raise C.WinError(C.get_last_error())
            pending=e
            p=bytes(e.payload);status=0x10002;stop=False
            if e.code==3:
                _,hp,ht,base=struct.unpack_from('<QQQQ',p)
                assert base==0x400000,hex(base)
                assert e.pid==process.pid
                threads[e.tid]=duplicate_owned(ht)
                globals_snapshot('create_process_before_program_execution',e.tid)
                arm_watches(e.tid)
                for address,label in [(0x85aa1b,'folder_redirect'),(0x8c3ed0,'defaults_init_entry'),(0x8c3f20,'defaults_properties_completed')]:breakpoint(address,label)
                if (handle:=struct.unpack_from('<Q',p)[0]):close(handle)
            elif e.code==2:
                threads[e.tid]=duplicate_owned(struct.unpack_from('<Q',p)[0]);arm_watches(e.tid)
                if observation_cutoff is not None:
                    check(suspend(threads[e.tid])!=0xffffffff)
                    manual_suspended[threads[e.tid]]=manual_suspended.get(threads[e.tid],0)+1
            elif e.code==4:
                handle=threads.pop(e.tid,None)
                if handle:close(handle)
            elif e.code==6:
                if (handle:=struct.unpack_from('<Q',p)[0]):close(handle)
            elif e.code==1:
                code=struct.unpack_from('<I',p)[0];address=struct.unpack_from('<Q',p,16)[0]
                if code in [0x80000003,0x4000001f] and address in points:
                    c=context(e.tid);original,label=points[address];change(address,original);c.Eip=address
                    if label=='folder_redirect':
                        assert c.Eax < 0x80000000, 'SHGetFolderPathW failed; isolation not established'
                        csidl=word(c.Esp+0x218)&0x7fff
                        dest=folder/('personal' if csidl==5 else 'common' if csidl==35 else 'other')
                        raw=(str(dest)+'\0').encode('utf-16-le');assert len(raw)<=520
                        change(c.Esp+8,raw)
                        assert memory(c.Esp+8,len(raw))==raw
                        events.append({'folder_redirection_csidl':csidl,'destination':str(dest),'tid':e.tid,
                            'hresult':c.Eax,'buffer_address':c.Esp+8,'buffer_bytes':len(raw),'verified':True})
                    elif label in ['defaults_init_entry','defaults_properties_completed']:
                        row=globals_snapshot(label,e.tid,c)
                        row['defaults_guard_byte']=memory(0xb45f54,1)[0]
                        addresses=[0xb43f3c,0xb442f8,0xb44304,0xb43f48,0xb44334,0xb44310,0xb4431c,0xb442ec,0xb43dfc,0xb43f1c,0xb43e08,0xb44328]
                        sizes=[0x24,0x24,0x34,0x64,0x24,0x24,0x24,0x40,0x48,0x2c,0x24,0x28]
                        row['properties']=[{'slot':i,'global':address,'pointer':word(address),'bytes':memory(word(address),size).hex() if word(address) else None}for i,(address,size)in enumerate(zip(addresses,sizes))]
                        if label=='defaults_properties_completed':
                            assert row['defaults_guard_byte']==1
                            assert all(p['pointer'] for p in row['properties'])
                            completed=True;session_completed=True;grid_completed=True
                    elif label in ['session_ctor_entry','session_ctor_phase_stored','session_ctor_return','change_state_entry','change_state_phase_stored','grid_ctor_entry','grid_ctor_return']:
                        globals_snapshot(label,e.tid,c)
                        if label=='grid_ctor_entry':
                            grid_pointer=c.Ecx;breakpoint(word(c.Esp),'grid_ctor_return')
                            global_captures[-1]['grid_pointer']=grid_pointer
                            global_captures[-1]['view_variable_tables']={}
                            for base,slots in [(0xb39990,[0,0x18]),(0xb39940,[0])]:
                                table=word(base);assert table
                                global_captures[-1]['view_variable_tables'][hex(base)]={'table':table,'slots':{hex(off):{'pointer':word(table+off),'bytes':memory(word(table+off),0x40).hex()}for off in slots}}
                        if label=='grid_ctor_return':
                            assert c.Eax==grid_pointer
                            grid_completed=True
                            global_captures[-1]['grid_bytes']=memory(grid_pointer,0x110).hex()
                        if label=='session_ctor_entry':breakpoint(word(c.Esp),'session_ctor_return')
                        if label=='session_ctor_phase_stored':
                            assert c.Esi==word(0xb36550) and word(c.Esi+0xf8)==0
                        if label=='session_ctor_return':
                            assert c.Eax==word(0xb36550)
                            session_completed=True
                    else:
                        if label=='source_entry':
                            assert any(event.get('folder_redirection_csidl')==5 for event in events), 'Personal folder was not redirected before source startup'
                            manager=c.Ecx;assert manager==word(0xb39f58), 'Source manager receiver differs from actual global'
                            breakpoint(word(c.Esp),'source_return')
                        snapshot(c,label,e.tid)
                        globals_snapshot(label,e.tid,c)
                        if label=='source_return':
                            rows=[];node=word(word(manager+0x68)+8);seen=set()
                            while node:
                                assert node not in seen;seen.add(node);actor=word(node)
                                rows.append({'ids':wide(word(actor+8)),'flags_174':word(actor+0x174), 'fields':{f'{off:x}':word(actor+off) for off in [0x1e4,0x30c,0x36c]}})
                                node=word(node+8)
                            captures[-1]['actors']=rows
                            assert len(rows)==1028, f'Expected1028 standard ActorData entries, observed {len(rows)}'
                            completed=True
                    if grid_completed:
                        observation_cutoff={'event_count':len(events),'global_capture_count':len(global_captures),
                            'write_count':len(watch_writes),'tid':e.tid,'eip':c.Eip,
                            'seed_words':[word(a) for a in seed_addresses]}
                        globals_snapshot('sealed_real_observation_cutoff',e.tid,c)
                        for tid,handle in threads.items():
                            if tid!=e.tid:
                                check(suspend(handle)!=0xffffffff)
                                manual_suspended[handle]=manual_suspended.get(handle,0)+1
                        start_probe(e.tid,0)
                        continue_pending(status)
                        continue
                    if stop:
                        # Keep the return instruction stopped. Cleanup requests
                        # termination before continuing this pending debug event.
                        break
                    suspended=[]
                    for tid,handle in threads.items():
                        if tid!=e.tid:
                            previous=suspend(handle)
                            check(previous!=0xffffffff);suspended.append(handle)
                            manual_suspended[handle]=manual_suspended.get(handle,0)+1
                    c.EFlags|=0x100;put_control(e.tid,c);stepping[e.tid]=(address,suspended)
                elif code in [0x80000004,0x4000001e]:
                    c=context(e.tid);hits=c.Dr6&0xf
                    if probe_state is not None:
                        assert e.tid==probe_state['tid'] and c.Eip==0x76fae7
                        assert hits==1<<probe_state['index'],(hits,probe_state)
                        assert word(probe_state['address'])==probe_state['value']
                        probes.append({**probe_state,'dr6':c.Dr6,'dr7':c.Dr7,'eip':c.Eip,'observed_bits':word(probe_state['address'])})
                        c.Dr6=0;put_debug(e.tid,c)
                        if probe_state['index']==3:
                            assert [word(a) for a in seed_addresses]==observation_cutoff['seed_words']
                            stop=True
                            break # retain pending debug stop until termination; execute no next instruction
                        else:start_probe(e.tid,probe_state['index']+1)
                    else:
                        if hits:
                            watch_writes.append(globals_snapshot('observed_native_grid_seed_write',e.tid,c,
                                hit_slots=[i for i in range(4) if hits&(1<<i)],
                                watched_addresses=[watch_addresses[i] for i in range(4) if hits&(1<<i)]))
                            c.Dr6=0;put_debug(e.tid,c)
                        if e.tid in stepping:
                            address,suspended=stepping.pop(e.tid);change(address,b'\xcc');c=context(e.tid);c.EFlags&=~0x100;put_control(e.tid,c)
                            for handle in suspended:
                                check(resume(handle)!=0xffffffff);manual_suspended[handle]-=1
                        elif not hits:
                            events.append({'unknown_single_step':hex(code),'address':hex(address),'dr6':c.Dr6});status=0x80010001
                elif code in [0x80000003,0x4000001f]:events.append({'loader_breakpoint':hex(code),'address':hex(address)})
                else:
                    events.append({'exception':hex(code),'address':hex(address),'first_chance':struct.unpack_from('<I',p,152)[0]});status=0x80010001
            elif e.code==5:
                events.append({'exit':struct.unpack_from('<I',p)[0]});stop=True;process_exited=True
            continue_pending(status)
            if stop:break
        if (not completed or not session_completed or len(probes)!=4) and not process_exited:
            raise TimeoutError('Bounded capture did not complete shared12-default init prefix+four isolated calibration probes')
    except BaseException as error:
        original_error={'type':type(error).__name__,'message':str(error),'traceback':traceback.format_exc(),
                        'pending_event':None if pending is None else {'code':pending.code,'pid':pending.pid,'tid':pending.tid},
                        'capture_count':len(captures)}
        # This survives even a later debugger cleanup failure and never masks the
        # original exception with a TerminateProcess/Wait assertion.
        with (folder/'failure-before-cleanup.json').open('x',encoding='utf-8') as stream:
            json.dump({'error':original_error,'events':events,'captures':captures},stream,indent=2)
        print(json.dumps({'original_error':original_error['message'],'type':original_error['type'],
                          'capture_count':len(captures)}),flush=True)
    finally:
        # TerminateProcess is asynchronous. A stopped debugger event, especially
        # EXIT_PROCESS, MUST be continued before the process object can signal.
        # Keep pumping termination events; never wait behind an uncontinued one.
        try:
            if join(owned_process,0)!=0:
                verify_owned_image()
                if not terminate(owned_process,0):
                    cleanup_errors.append({'operation':'TerminateProcess','winerror':C.get_last_error()})
            if pending is not None:
                cleanup_events.append({'continued_pending_event':pending.code,'tid':pending.tid})
                continue_pending()
            deadline=time.monotonic()+20
            while join(owned_process,0)!=0 and time.monotonic()<deadline:
                e=EVENT()
                if not wait(C.byref(e),200):
                    if C.get_last_error()==121:continue
                    cleanup_errors.append({'operation':'WaitForDebugEvent','winerror':C.get_last_error()});break
                pending=e;p=bytes(e.payload)
                cleanup_events.append({'code':e.code,'pid':e.pid,'tid':e.tid})
                if e.code in [3,6] and (handle:=struct.unpack_from('<Q',p)[0]):close(handle)
                if e.code==4:
                    if (handle:=threads.pop(e.tid,None)):close(handle)
                if e.code==5:process_exited=True
                continue_pending()
            wait_result=join(owned_process,1000)
            if wait_result!=0:cleanup_errors.append({'operation':'WaitForSingleObject','result':wait_result})
            value=W.DWORD()
            code_ok=bool(exit_code(owned_process,C.byref(value)))
            cleanup_result={'wait_result':wait_result,'exit_code':value.value if code_ok else None,
                            'exit_process_event_observed':process_exited,'errors':cleanup_errors,'events':cleanup_events,
                            'manual_suspend_counts_at_termination':manual_suspended}
        except BaseException as error:
            cleanup_errors.append({'operation':'cleanup_exception','message':str(error),'traceback':traceback.format_exc()})
            cleanup_result={'errors':cleanup_errors,'events':cleanup_events}
        for handle in set(threads.values()):close(handle)
        close(owned_process)
        preserved=all(sha(Path(path))==digest for path,digest in original_hashes.items())
        redirected=any(event.get('folder_redirection_csidl')==5 and event.get('verified') for event in events)
        report={'schema':2,'source_exe_sha256':sha(executable),'completed_default_init_prefix':completed,'completed_twelve_properties':grid_completed,'seed_addresses':seed_addresses,
            'scope':'Actual Windows/v14 from process creation through8C3ED0 guard and12 shareddefault constructors ending8C3F20. No later type-registration/first-map/battle coverage or perpetual global immutability claim. All probes after sealed observation cutoff.',
            'owned_process':{'pid':process.pid,'exe':verified_image},'original_error':original_error,'cleanup':cleanup_result,
            'runtime_pins':{name:sha(executable.parent/name) for name in ['AA.exe','D3D9.dll','dgVoodoo.conf']},
            'temporary_process_only_breakpoints':True,'get_context_flags':get_context_flags,'set_context_flags':set_context_flags,
            'fp_state_never_written_by_debugger':True,'folder_redirection_observed':redirected,'events':events,'captures':captures,
            'debug_register_set_context_flags':0x10010,'probe_integer_control_set_context_flags':0x10003,
            'watched_word_indices':args.words,'watched_addresses':watch_addresses,'watch_arms':watch_arms,
            'global_captures':global_captures,'observed_native_watched_writes':watch_writes,
            'completed_default_prefix':session_completed,'observation_cutoff':observation_cutoff,
            'isolated_post_cutoff_watch_calibration_probes':probes,
            'original_archives_preserved':preserved,'original_archives_sha256':original_hashes,'script_sha256':sha(Path(__file__))}
        with (folder/'capture.json').open('x',encoding='utf-8') as stream:json.dump(report,stream,indent=2);stream.write('\n')
        print(json.dumps({'completed':completed,'capture_count':len(captures),'preserved':preserved,
            'original_error':original_error,'cleanup':cleanup_result,'output':str(folder/'capture.json')},indent=2),flush=True)
    assert grid_completed and completed and session_completed and len(probes)==4 and preserved and original_error is None and not cleanup_errors, 'Capture incomplete; original error is preserved in capture.json'


if __name__=='__main__':main()
