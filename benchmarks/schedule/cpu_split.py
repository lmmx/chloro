#!/usr/bin/env python3
"""EXP-023: CPU user/sys of one binary in three configurations, interleaved:
1c1t = taskset -c 3 --threads 1; 1c4t = taskset -c 3 --threads 4; 4c4t = --threads 4 unpinned.
usage: cpu_split.py BINARY ROUNDS crate..."""
import os, sys, time, statistics
binary, rounds, crates = sys.argv[1], int(sys.argv[2]), sys.argv[3:]
cfgs = {'1c1t': (['taskset', '-c', '3'], 1), '1c4t': (['taskset', '-c', '3'], 4), '4c4t': ([], 4)}
def run(argv):
    dn = os.open(os.devnull, os.O_RDWR)
    t0 = time.perf_counter_ns()
    pid = os.posix_spawnp(argv[0], argv, os.environ, file_actions=[(os.POSIX_SPAWN_DUP2, dn, 0), (os.POSIX_SPAWN_DUP2, dn, 1), (os.POSIX_SPAWN_DUP2, dn, 2)])
    _, st, ru = os.wait4(pid, 0)
    t1 = time.perf_counter_ns(); os.close(dn)
    return (t1 - t0) / 1e6, ru.ru_utime * 1e3, ru.ru_stime * 1e3, ru.ru_maxrss / 1024, ru.ru_nvcsw, ru.ru_nivcsw, ru.ru_minflt
print('crate | cfg | wall med | user med | sys med | user+sys med | rss MiB | vol csw | invol csw | minflt | load')
for c in crates:
    tree = f'/tmp/chloro-bench/formatted/{c}/tree'
    res = {k: [] for k in cfgs}
    for r in range(rounds):
        for k, (pre, th) in (cfgs.items() if r % 2 == 0 else reversed(list(cfgs.items()))):
            res[k].append(run(pre + [binary, '--check', '--no-cache', '--threads', str(th), tree]))
    for k, v in res.items():
        med = [statistics.median(x[i] for x in v) for i in range(7)]
        print(f'{c} | {k} | {med[0]:.1f} | {med[1]:.1f} | {med[2]:.1f} | {med[1]+med[2]:.1f} | {med[3]:.1f} | {med[4]:.0f} | {med[5]:.0f} | {med[6]:.0f} | {os.getloadavg()[0]:.1f}')
