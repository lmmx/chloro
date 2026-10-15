#!/usr/bin/env python3
"""EXP-023 diagnostic: run a traced chloro binary N times on a workload and derive the
schedule's utilisation, tail and phase times (the wall view of the CHLORO_TRACE timeline).

usage: [E23_MODE=check-nocache|check-coldcache|write-raw] diag.py BINARY WORKLOAD_DIR RUNS THREADS OUT_PREFIX [taskset-cpus]
Writes OUT_PREFIX.trace<k>.tsv and OUT_PREFIX.runs.tsv (one line per run) and prints the
medians as one JSON line. Work intervals: each file on its worker, less a split file's units
phase (its worker waits in join) when the build records it (trace_patch.py), plus each item
worker; idle core time is 4 minus the number of such intervals open, over the format window.
"""
import os, sys, time, statistics, json

MODE = os.environ.get('E23_MODE', 'check-nocache')
def run_once(binary, tree, threads, trace_path, cpus=None):
    if MODE == 'check-nocache':
        argv = [binary, '--check', '--no-cache', '--threads', str(threads), tree]
    elif MODE == 'check-coldcache':
        import shutil, tempfile
        cdir = tempfile.mkdtemp(prefix='e23cache')
        argv = [binary, '--check', '--cache-dir', cdir, '--threads', str(threads), tree]
    elif MODE == 'write-raw':
        import shutil, tempfile
        work = tempfile.mkdtemp(prefix='e23write')
        shutil.copytree(tree.replace('/formatted/', '/raw/'), work + '/tree')
        argv = [binary, '--write', '--no-cache', '--threads', str(threads), work + '/tree']
    if cpus:
        argv = ['taskset', '-c', cpus] + argv
    env = dict(os.environ, CHLORO_TRACE=trace_path)
    devnull = os.open(os.devnull, os.O_RDWR)
    t0 = time.perf_counter_ns()
    pid = os.posix_spawnp(argv[0], argv, env, file_actions=[
        (os.POSIX_SPAWN_DUP2, devnull, 0), (os.POSIX_SPAWN_DUP2, devnull, 1), (os.POSIX_SPAWN_DUP2, devnull, 2)])
    _, status, ru = os.wait4(pid, 0)
    t1 = time.perf_counter_ns()
    os.close(devnull)
    if MODE != 'check-nocache':
        import shutil
        shutil.rmtree(cdir if MODE == 'check-coldcache' else work)
    return (t1 - t0) / 1e6, (ru.ru_utime + ru.ru_stime) * 1e3, os.waitstatus_to_exitcode(status)

def parse(path):
    marks, workers, files, items, cpu = {}, [], [], [], {}
    for line in open(path):
        f = line.rstrip('\n').split('\t')
        if f[0] == 'mark':
            marks[f[1]] = int(f[2])
        elif f[0] == 'worker':
            workers.append(dict(id=int(f[1]), start=int(f[2]), end=int(f[3]), cpu=int(f[4]) if len(f) > 4 else 0))
        elif f[0] == 'file':
            files.append(dict(worker=int(f[1]), bytes=int(f[2]), split=f[3] == '1', start=int(f[4]),
                              read=int(f[5]), hashed=int(f[6]), formatted=int(f[7]), end=int(f[8]), cpu=0))
        elif f[0] == 'cpu':
            cpu[int(f[1])] = int(f[2])
        elif f[0] == 'item':
            items.append(dict(kind=int(f[1]), start=int(f[2]), end=int(f[3]), cpu=int(f[4]), n=int(f[5])))
    for f in files:
        f['cpu'] = cpu.get(f['start'], 0)
    return marks, workers, files, items

def analyse(marks, workers, files, items, ncores=4):
    ms = 1e-6
    T = marks['finish']
    spawn, joined = marks['spawn'], marks['joined']
    # Work intervals: each file on its worker, minus the units phase of a split file (the
    # worker blocks in join), plus each item worker.
    phases = [i for i in items if i['kind'] == 2]
    ivs = []
    for f in files:
        if f['split']:
            inner = [p for p in phases if p['start'] >= f['start'] and p['end'] <= f['end']]
            cur = f['start']
            for p in sorted(inner, key=lambda p: p['start']):
                ivs.append((cur, p['start']))
                cur = p['end']
            ivs.append((cur, f['end']))
        else:
            ivs.append((f['start'], f['end']))
    ivs += [(i['start'], i['end']) for i in items if i['kind'] == 0]
    # Sweep: integral of min(ncores, active) over [spawn, joined].
    ev = []
    for a, b in ivs:
        if b > a:
            ev.append((a, 1)); ev.append((b, -1))
    ev.sort()
    busy = 0.0; over = 0.0; active = 0; last = spawn
    first_idle_core = None  # first time after ramp-up that active < ncores
    reached = False
    for t, d in ev:
        t = max(t, spawn)
        if t > last:
            busy += min(ncores, active) * (t - last)
            over += max(0, active - ncores) * (t - last)
            last = t
        active += d
        if active >= ncores:
            reached = True
        elif reached and first_idle_core is None:
            first_idle_core = t
    if joined > last:
        last = joined
    F = joined - spawn
    first_exit = min(w['end'] for w in workers)
    last_start = max(w['start'] for w in workers)
    tail = joined - first_exit
    # Files running at the first worker exit: who owns the tail.
    tail_files = sorted([f for f in files if f['end'] > first_exit], key=lambda f: -f['end'])
    owner = tail_files[0] if tail_files else None
    # Lost core-time over the in-process span, in core-ms.
    cap = ncores * T
    pre = spawn; post = T - joined
    serial_lost = (ncores - 1) * (pre + post)
    # idle inside the format window, split into ramp-up, tail and middle
    def idle_between(a, b):
        # core-time idle in [a, b]
        tot = 0.0; act = 0; lastt = a
        for t, d in ev:
            if t > lastt and t <= b:
                tot += (ncores - min(ncores, act)) * (t - max(lastt, a)) if t > a else 0
            if t > a:
                lastt = max(lastt, min(t, b))
            act += d
            if t >= b:
                break
        if b > lastt:
            tot += (ncores - min(ncores, act)) * (b - lastt)
        return tot
    idle_total = ncores * F - busy
    idle_tail = idle_between(first_exit, joined)
    idle_ramp = idle_between(spawn, last_start)
    idle_mid = idle_total - idle_tail - idle_ramp
    wk = {w['id']: w for w in workers}
    util = []
    for wid, w in sorted(wk.items()):
        s = sum(f['end'] - f['start'] for f in files if f['worker'] == wid)
        util.append(s / F if F else 0)
    oncpu_files = sum(f['cpu'] for f in files)
    span_files = sum(f['end'] - f['start'] for f in files)
    item_workers = [i for i in items if i['kind'] == 0]
    oncpu_items = sum(i['cpu'] for i in item_workers)
    span_items = sum(i['end'] - i['start'] for i in item_workers)
    split_files = [f for f in files if f['split']]
    parse_split = sum(i['end'] - i['start'] for i in items if i['kind'] == 1)
    stitch_split = sum(i['end'] - i['start'] for i in items if i['kind'] == 3)
    read = sum(f['read'] - f['start'] for f in files)
    hashed = sum(f['hashed'] - f['read'] for f in files if f['hashed'])
    fmt = sum(f['formatted'] - (f['hashed'] or f['read']) for f in files if f['formatted'])
    out = sum(f['end'] - f['formatted'] for f in files if f['formatted'])
    return dict(
        T=T*ms, pre=pre*ms, args=marks['args']*ms, walk=(marks['walk']-marks['cache'])*ms,
        sort=(marks['sort']-marks['walk'])*ms, spawn_gap=(spawn-marks['sort'])*ms,
        F=F*ms, post=post*ms, busy=busy*ms, over=over*ms,
        first_exit=(first_exit-spawn)*ms, tail=tail*ms, tail_share=tail/T,
        owner_bytes=owner['bytes'] if owner else 0, owner_split=owner['split'] if owner else False,
        owner_span=(owner['end']-owner['start'])*ms if owner else 0,
        ramp=(last_start-spawn)*ms,
        lost_serial=serial_lost*ms, lost_ramp=idle_ramp*ms, lost_tail=idle_tail*ms, lost_mid=idle_mid*ms,
        cap=cap*ms, util=util, util_mean=statistics.mean(util) if util else 0,
        oncpu_frac=(oncpu_files + oncpu_items) / max(1, span_files - sum(
            (p['end']-p['start']) for p in phases) + span_items),
        nsplit=len(split_files), parse_split=parse_split*ms, stitch_split=stitch_split*ms,
        read=read*ms, hash=hashed*ms, fmt=fmt*ms, out=out*ms, nfiles=len(files),
        cache=(marks['cache']-marks['args'])*ms, save=(marks['saved']-marks['formatted'])*ms,
    )

def main():
    binary, tree, runs, threads, prefix = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
    cpus = sys.argv[6] if len(sys.argv) > 6 else None
    rows = []
    for r in range(runs):
        tp = f'{prefix}.trace{r}.tsv'
        wall, cpu, code = run_once(binary, tree, threads, tp, cpus)
        a = analyse(*parse(tp), ncores=threads)
        a.update(wall=wall, cpu=cpu, code=code, load=os.getloadavg()[0])
        rows.append(a)
    keys = [k for k in rows[0] if k != 'util']
    with open(prefix + '.runs.tsv', 'w') as fh:
        fh.write('\t'.join(keys + ['util']) + '\n')
        for a in rows:
            fh.write('\t'.join(f'{a[k]:.3f}' if isinstance(a[k], float) else str(a[k]) for k in keys)
                     + '\t' + ','.join(f'{u:.3f}' for u in a['util']) + '\n')
    med = {k: statistics.median([a[k] for a in rows]) for k in keys if not isinstance(rows[0][k], bool)}
    print(json.dumps({k: round(v, 3) for k, v in med.items()}))

if __name__ == '__main__':
    main()
