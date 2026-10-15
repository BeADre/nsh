# Performance

Performance is the project's first priority; the rules are in `AGENTS.md`. Changes to hot paths come with before /
after `nsh trace` timings and item counts. Current baselines (Apple Silicon):

- counter, headless (debug): first frame ~0.5–0.65ms (58 ops / 49 mount items), one tap ~0.15ms (12 ops / 8 items).
- counter, Catalyst (release): first frame ~1.05–1.15ms (of which ~0.7ms is cold CoreText layout of 10 texts), one tap ~0.12ms.
- Hot reload: ~120ms from saving a file to the device running the new version (file event latency and ~60ms of development
  bundling included); applying a hot update takes 0.2–0.5ms on the engine thread.
- gallery Photos list (`examples/gallery/Bench.tsx`, debug, headless): first frame 653 ops / 224 nodes, ~2.7ms (list bind
  rounds included).
- Photos scrolling (release): a frame that rebinds one row is 4 ops / 4 mount items; frames that reveal no new row call no JS and produce
  no batch. `list_bench` runs at ~20–30µs per frame back to back (JS ~20µs) and ~40µs with caches flushed between frames, but ~150µs after the
  thread sleeps 10ms. The difference is the CPU clocking down while the thread sleeps (Apple CLPC), not the code path; neither QoS (the engine
  thread is USER_INTERACTIVE) nor the Mach time-constraint policy changes it. Judge optimizations by the back-to-back and flushed-cache numbers:
  single-frame `nsh trace` timings are dominated by the clock state.
- On Catalyst (release), rebinding one row while scrolling slowly takes ~0.4ms (clocked down) and rebinding a whole screen of ~32 rows on a
  fast fling 0.9–1.6ms.
- Host startup (`Host::new`, release): ~155µs. The web APIs add nothing until first used (they used to cost ~0.8ms as source,
  ~0.33ms as bytecode).
- Android emulator (arm64, release): rebinding one gallery row while scrolling ~0.3–0.4ms (JS ~60µs); text costs per new
  text: `PrecomputedText` ~120µs, measure ~50µs, final layout reused. First gallery frame 20–50ms (noisy on the emulator).

- Rust components vs the same component in TSX (`component_bench`, `examples/bench`, release, minimum of the runs;
  the TSX Button is the one `nsh/ui/mobile` had before components moved into Rust):

  | | Rust Button: JS / core | TSX Button: JS / core |
  |---|---|---|
  | first frame, 200 Buttons (ops) | 1.29ms / 0.77ms (1015) | 2.34ms / 0.71ms (2415) |
  | one update of all 200 titles (ops) | 0.43ms / 0.40ms (200) | 0.76ms / 0.35ms (600) |
  | scrolling a 1000-row list, per rebinding frame | 8.7µs / 6.2µs | 10.7µs / 5.8µs |

  JS does half the work or less; the core spends ~10% more expanding renders than decoding the equivalent ops, a net
  win of ~30% on the first frame and ~25% on updates. The glue must stay one effect per instance reading the props
  directly: the first version (`splitProps` / `mergeProps` / `spread`) cost 4.0ms of JS for the same first frame,
  more than the TSX Button.

The dev profile builds `rquickjs-sys` and `taffy` with `opt-level = 3`, so debug timings are meaningful too; release uses fat LTO.
