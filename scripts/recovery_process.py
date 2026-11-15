"""Read-only macOS identity and snapshot reconciliation for Issue 44.

The ABI layout follows Apple's proc_info.h (PROC_PIDTBSDINFO=3).
Only identity-bound callers may later decide to signal their own Popen process.
"""
import ctypes
import errno
import sys
import recovery_environment as environment


class BSDInfo(ctypes.Structure):
    _fields_ = [(n, ctypes.c_uint32) for n in (
        'flags', 'status', 'xstatus', 'pid', 'ppid', 'uid', 'gid', 'ruid',
        'rgid', 'svuid', 'svgid', 'reserved')]
    _fields_ += [('comm', ctypes.c_char*16), ('name', ctypes.c_char*32)]
    _fields_ += [(n, ctypes.c_uint32) for n in ('nfiles','pgid','jobc','tdev','tpgid')]
    _fields_ += [('nice',ctypes.c_int32),('start_sec',ctypes.c_uint64),('start_usec',ctypes.c_uint64)]


def identity(pid):
    if sys.platform != 'darwin':
        raise ValueError('MACOS_IDENTITY_REQUIRED')
    lib = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
    lib.proc_pidinfo.argtypes = [ctypes.c_int,ctypes.c_int,ctypes.c_uint64,ctypes.c_void_p,ctypes.c_int]
    lib.proc_pidpath.argtypes = [ctypes.c_int,ctypes.c_void_p,ctypes.c_uint32]
    def info():
        value=BSDInfo();ctypes.set_errno(0)
        n=lib.proc_pidinfo(pid,3,0,ctypes.byref(value),ctypes.sizeof(value))
        if n==0 and ctypes.get_errno()==errno.ESRCH:return None
        if n!=ctypes.sizeof(value):raise ValueError('PROCESS_IDENTITY_UNAVAILABLE')
        if value.pid!=pid:raise ValueError('PROCESS_IDENTITY_MISMATCH')
        return (value.ppid,value.start_sec*1_000_000+value.start_usec)
    before=info()
    if before is None:return None
    path=ctypes.create_string_buffer(4096);ctypes.set_errno(0)
    n=lib.proc_pidpath(pid,path,len(path))
    after=info()
    if after is None:return None
    if after!=before:raise ValueError('PROCESS_IDENTITY_CHANGED')
    if n<=0:raise ValueError('PROCESS_PATH_UNAVAILABLE')
    return (*after,path.value.decode('utf-8'))


def capture():
    """Only enrich sensitive candidates; never persist unrelated command arguments."""
    result=[]
    for pid,ppid,exe in environment.processes():
        counts=environment.classify([(pid,ppid,exe)])
        candidate=any(counts[k] for k in ('build_or_server','other_aether'))
        ident=identity(pid) if candidate else None
        result.append((pid,ppid,exe,ident,candidate))
    return result


def reconcile(rows, host_pid, host_identity, child_executable, children, read_identity=None):
    read_identity = read_identity or identity
    owned=set();current_rows=[];audit=[]
    for pid,ppid,exe,snapshot_identity,candidate in rows:
        if not candidate:
            current_rows.append((pid,ppid,exe));continue
        current=read_identity(pid)
        is_host=pid==host_pid
        is_child=ppid==host_pid and exe==child_executable
        if snapshot_identity is None:
            # Cannot prove ownership from a PID/PPID/path-only stale row.
            raise ValueError('SNAPSHOT_IDENTITY_UNAVAILABLE')
        if (ppid,exe)!=(snapshot_identity[0],snapshot_identity[2]):
            raise ValueError('SNAPSHOT_IDENTITY_CHANGED')
        if current is not None and current!=snapshot_identity:
            raise ValueError('PID_REUSED_OR_IDENTITY_CHANGED')
        if is_host:
            if snapshot_identity!=host_identity:raise ValueError('HOST_IDENTITY_CHANGED')
            if current is None:
                audit.append(dict(kind='HOST_DISAPPEARED',pid=pid,start_us=snapshot_identity[1]))
                continue  # Normal exit still requires Popen exit 0, not this observation.
            owned.add(pid)
        elif is_child:
            known=children.get(pid)
            if known is not None and known!=snapshot_identity:raise ValueError('CHILD_IDENTITY_CHANGED')
            if current is None:
                if known!=snapshot_identity:raise ValueError('UNREGISTERED_CHILD_DISAPPEARED')
                audit.append(dict(kind='CHILD_DISAPPEARED',pid=pid,start_us=snapshot_identity[1]))
                continue  # Normal reap is a separate required ledger proof.
            host_now=read_identity(host_pid)
            if host_now!=host_identity:raise ValueError('CHILD_PARENT_UNVERIFIED')
            children[pid]=snapshot_identity;owned.add(pid)
            audit.append(dict(kind='CHILD_CONFIRMED' if known else 'CHILD_DISCOVERED',pid=pid,
                              ppid=ppid,start_us=snapshot_identity[1]))
        elif current is None:
            # A foreign build seen in this snapshot still invalidates the interval.
            audit.append(dict(kind='FOREIGN_DISAPPEARED',pid=pid))
        current_rows.append((pid,ppid,exe))
    for pid,known in children.items():
        now=read_identity(pid)
        if now is not None and now!=known:raise ValueError('CHILD_IDENTITY_CHANGED')
    counts=environment.classify(current_rows,owned)
    return {k:counts[k] for k in ('build_or_server','other_aether','postgres','docker')},audit


def inspect(host_pid,host_identity,child_executable,children,rows=None):
    return reconcile(capture() if rows is None else rows,host_pid,host_identity,child_executable,children)


def sample(pid, expected):
    from benchmark import command, cpu_seconds
    before=identity(pid)
    if before is None:return None
    if before!=expected:raise ValueError('SAMPLE_IDENTITY_CHANGED')
    result=command(['/bin/ps','-p',str(pid),'-o','time=','-o','rss='])
    after=identity(pid)
    if after is None:return None
    if after!=expected:raise ValueError('SAMPLE_IDENTITY_CHANGED')
    parts=result.stdout.split()
    if result.returncode or len(parts)!=2:raise ValueError('RESOURCE_SAMPLE_UNAVAILABLE')
    return dict(cpu_seconds=cpu_seconds(parts[0]),rss_bytes=int(parts[1])*1024)
