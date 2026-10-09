# Scanner measurements

2026-10-07, this development machine, Windows x86-64, optimized build. Synthetic
fixture: 50 projects, 20,050 regular files, 10,240,100 logical bytes. These are
development measurements, not representative speed guarantees.

| Run | Scan | 100 page queries | Cancellation | Sampled process working set peak |
| --- | --- | --- | --- | --- |
| 0.1.0 | 5,570 ms | 42 ms | 8 ms | 7,892,992 bytes |

Cancellation was requested 100 ms after starting a second scan and the scanner
returned `cancelled`. Windows/network reads can still block longer. Memory is
sampled every 200 ms for the whole benchmark process including fixture creation;
it is a lower bound on the instantaneous peak. Page queries included JSON decoding.

Run `scripts/benchmark.ps1` to reproduce. OS filesystem caches were not flushed.
Other development builds and a repository scan ran during this measurement;
filesystem and CPU contention make it unsuitable for comparison with earlier runs.
Changing classification or fixtures requires a new measurement before comparing.
