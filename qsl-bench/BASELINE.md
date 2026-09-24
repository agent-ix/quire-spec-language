# QSL-196 performance baseline

This is the baseline that QSL-202 to QSL-206 compare against. Every figure
below comes from a run of the benchmarks in this crate. Nothing is estimated
or carried over from the architecture evaluation.

## Running the benchmarks

| Axis | Command | Bench file | Input generator |
| --- | --- | --- | --- |
| Parser: nesting depth and volume | `make bench-parser` | `benches/parser.rs` | `src/parse.rs` |
| Checker: call chain and independent functions | `make bench-checker` | `benches/checker.rs` | `src/check.rs` |
| CST identity hashing against source bytes | `make bench-cst` | `benches/cst.rs` | `src/parse.rs` |
| Model layer on an N-type `DomainPackage` built through FR-154 intake | `make bench-model` | `benches/model.rs` | `src/model.rs` |
| Evaluator cost per call frame | `make bench-evaluator` | `benches/evaluator.rs` | `src/check.rs` |
| Counts, refusal boundaries, one-shot large inputs and peak RSS | `make bench-probe` | `src/bin/qsl-bench-probe.rs` | all of the above |

`make bench` runs all five criterion suites. Each target runs
`cargo bench --locked -p qsl-bench --bench <axis>`. Criterion options go through
`BENCH_ARGS`. For example, `make bench-model BENCH_ARGS='--save-baseline before'`
saves a baseline, and `BENCH_ARGS='--baseline before'` compares against it.

Every input is built through the layers' public API: `qsl_cst::parse`,
`PackageDeclarations::check`, `CheckedPackage::call`, and
`qsl_semantics::model::{intake, normalize, population}`. So the benchmarks keep
compiling when a layer is rewritten behind that API. QSL-197 rewrites the S1
parser behind `qsl_cst::parse`.

## Where and when this was measured

| Item | Value |
| --- | --- |
| Source revision | `4dd02fb154397a76ddc8cf306cffee298b28758a`. Product code is identical to `89326999`, since this branch adds only `qsl-bench/` and inventory docs. |
| Toolchain | rustc 1.98.1 (48a229cea 2026-09-01), `bench` profile (release) |
| Machine | Intel Core i9-10900K @ 3.70 GHz, 20 logical CPUs (`nproc`), 62 GiB RAM, WSL2 Linux 6.18.33.2-microsoft-standard-WSL2 |
| Harness | criterion 0.8.2, default features off. Defaults apply unless a bench overrides them: 3 s warm-up, 100 samples, 5 s measurement. |
| Runs | 5 complete runs of all five suites, one after another, 2026-09-23 17:22 to 18:23 (UTC-7) |
| Machine load | 1-minute load average ranged from 3.3 to 26.6 across the runs. Two other QSL builds were running on the same machine throughout. |

**The load was high.** Several runs started at load 18 to 27 on 20 logical
CPUs. The spread between runs is correspondingly wide: the slowest of five runs
is often 1.5 to 2 times the fastest. The medians and the stated variance below
are robust to one or two slow runs, but they are still figures for a shared,
loaded machine.

## How the figures and the variance are computed

- **Figure.** For each benchmark, the figure is the median of five point
  estimates. Each point estimate is criterion's estimate of wall time per
  iteration, taken from one complete run.
- **Range.** The fastest and slowest of the five point estimates.
- **Stated variance.** The larger of two measures:
  - the median absolute deviation of the five point estimates, divided by
    their median;
  - the widest criterion 95% confidence-interval half-width in any run,
    divided by that run's estimate.
- **What the variance rules out (QSL-196 acceptance criterion 3).** A later
  change claims an improvement on a benchmark only when both hold:
  - its own median of five runs, on this machine, is below this median by more
    than the stated variance;
  - it clears that bar in an A/B pair: the baseline revision and the candidate
    revision run back to back, under the same load.

  A smaller change is recorded as no change. Because the machine is shared, the
  A/B pair is what counts. A candidate compared against this table alone is
  exposed to the load drift the range column shows.

## Baseline tables

Each row gives the unit (wall time per call), the input size in the unit that
layer counts, and the comparison point. The comparison point is the other row
the figure is meant to be read against.

### Parser: one `qsl_cst::parse` call (`benches/parser.rs`)

Input: one function whose body nests *d* parenthesis pairs, such as
`((x + x) + x)` for *d* = 2 (`parser/depth/d`). Or *n* one-line functions
`function fI using Complete (x: Integer): Integer pure { x }`
(`parser/volume/n`). Both use default `Limits`.

| Benchmark | Input size | Median | Range (5 runs) | Stated variance | Outcome at `89326999` | Comparison point |
| --- | --- | --- | --- | --- | --- | --- |
| `parser/depth/0` | depth 0, 243 B, 52 tokens, 21 nodes | 278 µs | 261 µs – 579 µs | 13% | admitted | Floor for the depth rows |
| `parser/depth/1` | depth 1, 249 B | 395 µs | 364 µs – 891 µs | 8% | admitted | depth 0 |
| `parser/depth/2` | depth 2, 255 B | 668 µs | 480 µs – 746 µs | 12% | admitted | depth 0 |
| `parser/depth/3` | depth 3, 261 B | 740 µs | 600 µs – 870 µs | 17% | admitted | depth 0 |
| `parser/depth/4` | depth 4, 267 B, 76 tokens, 81 nodes | 979 µs | 820 µs – 1.04 ms | 6% | admitted (deepest that parses) | depth 0: 3.5 times the time for 10% more bytes |
| `parser/depth/5` | depth 5, 273 B | 339 µs | 283 µs – 536 µs | 14% | **refused**: `resource_exhausted` / `insufficient-next-charge` | depth 4. Time to reach the refusal. |
| `parser/depth/8` | depth 8, 291 B | 324 µs | 293 µs – 548 µs | 10% | **refused**, same cause | depth 5 |
| `parser/volume/100` | 100 functions, 6,269 B | 9.70 ms | 8.07 ms – 20.0 ms | 16% | admitted | volume/1000 (linearity) |
| `parser/volume/500` | 500 functions, 31,069 B | 61.7 ms | 42.8 ms – 84.4 ms | 21% | admitted | volume/1000 |
| `parser/volume/1000` | 1,000 functions, 62,069 B, 26,026 tokens, 18,003 nodes | 105 ms | 95.9 ms – 165 ms | 9% | admitted | About 105 µs per one-line function, or 0.6 MB/s |
| `parser/volume/2000` | 2,000 functions, 125,069 B, 36,003 nodes | 208 ms | 183 ms – 311 ms | 12% | admitted (largest that parses) | volume/1000: 2.0 times, linear |
| `parser/volume/3000` | 3,000 functions, 188,069 B | 138 ms | 115 ms – 189 ms | 16% | **refused**: `resource_exhausted`, "complete grammar work or nesting budget exhausted" | Time to reach the refusal. QSL-197 AC 5 requires this input to parse. |

Refusal boundary, from `qsl-bench-probe parse`, which tries depths 0 to 64:
depth 4 is the deepest nesting that parses, and every depth from 5 to 64 is
refused. 4,000 functions is refused with "CST leaf budget exhausted". After
QSL-197, the `depth/5`, `depth/8` and `volume/3000` rows stop measuring a
refusal. Re-baseline them rather than reading them as a speed-up.

### Checker: one `PackageDeclarations::check` over the whole package (`benches/checker.rs`)

Input: `checker/chain/n` is an *n*-function call chain,
`f0(x) = f1(x) … f{n-1}(x) = x`. `checker/independent/n` is *n* functions
`fI(x) = x + 1` with no calls between them. Building the declarations is
excluded from the timing.

| Benchmark | Input size | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- | --- |
| `checker/chain/250` | 250 declarations, 249 calls | 5.97 ms | 4.45 ms – 7.39 ms | 12% | — |
| `checker/chain/1000` | 1,000 declarations | 104 ms | 90.0 ms – 118 ms | 13% | chain/250: 17 times the time for 4 times the size |
| `checker/chain/2000` | 2,000 declarations | 564 ms | 453 ms – 592 ms | 12% | chain/1000: 5.4 times the time for 2 times the size |
| `checker/independent/1000` | 1,000 declarations, no calls | 12.0 ms | 8.87 ms – 13.1 ms | 15% | chain/1000 |
| `checker/independent/5000` | 5,000 declarations, no calls | 227 ms | 175 ms – 274 ms | 21% | independent/1000: 19 times the time for 5 times the size, so quadratic |

Chains too slow to sample repeatedly were measured once per process with
`qsl-bench-probe check chain <n>`. Each size ran in three separate processes.
The figures are wall time for one `check`, and the peak RSS (`VmHWM`) of a
process that built and checked only that chain.

| Input size | Wall time, 3 runs | Median | Peak RSS | Comparison point |
| --- | --- | --- | --- | --- |
| chain of 4,000 | 6,782 ms, 4,956 ms, 8,580 ms | 6.78 s | 179 MB (179,140 to 179,184 KiB) | QSL-203 reports 7.4 s and 178 MB from the evaluation, which agrees |
| chain of 8,000 | 38,165 ms, 29,812 ms, 33,564 ms | 33.6 s | 682 MB (681,716 to 681,884 KiB) | chain of 4,000: 5.0 times the time and 3.8 times the memory for 2 times the size |

For the smaller sizes, the probe measured, once each: chain of 250 at 6.5 ms and
5.4 MB; 1,000 at 134 ms and 17 MB; 2,000 at 661 ms and 51 MB; independent 1,000
at 18 ms and 7.2 MB; independent 5,000 at 266 ms and 21 MB.

### CST identity hashing (`benches/cst.rs`)

`qsl_cst::parse` gives every CST node a `StableNodeId`. The ID is a SHA-256
digest over a preimage that holds the node's whole source slice and every
ancestor's production name (`LosslessCst::new`). `cst/parse/<input>` times the
whole parse. `cst/sha256/<input>` times SHA-256 alone over a buffer the size of
that input's hashed bytes, split into one message per node.

The hashed-byte counts come from `qsl-bench-probe cst`. They are computed from
each parsed CST's own node spans and ancestor lists, using the preimage layout
at `89326999`. They are a model of that code, not an instrumented count. The
`cst/sha256` timings are the measured counterpart.

| Input | Source bytes | CST nodes | Bytes hashed | Hashed per source byte | Of which: source slices / ancestor names |
| --- | --- | --- | --- | --- | --- |
| nested-0 | 243 | 21 | 3,369 | 13.9 | 593 / 1,442 |
| nested-1 | 249 | 36 | 7,405 | 29.7 | 716 / 4,529 |
| nested-2 | 255 | 51 | 12,881 | 50.5 | 899 / 8,996 |
| nested-3 | 261 | 66 | 19,797 | 75.9 | 1,142 / 14,843 |
| nested-4 | 267 | 81 | 28,153 | 105.4 | 1,445 / 22,070 |
| volume-1000 | 62,069 | 18,003 | 2,671,362 | 43.0 | 231,026 / 1,416,026 |

Hashed bytes per source byte grow about quadratically with nesting depth:
13.9, 29.7, 50.5, 75.9 and 105.4 at depths 0 to 4. The ancestor-name paths
dominate: 78% of the hashed bytes at depth 4. The source slices, the part a
reader would expect, are at most 5.4 bytes per source byte.

| Benchmark | Median | Range (5 runs) | Stated variance | Comparison point: SHA-256 share of the parse |
| --- | --- | --- | --- | --- |
| `cst/parse/nested-0` | 375 µs | 260 µs – 659 µs | 29% | — |
| `cst/sha256/nested-0` | 18.3 µs | 14.6 µs – 27.1 µs | 19% | 4.9% of `cst/parse/nested-0` |
| `cst/parse/nested-1` | 491 µs | 363 µs – 1.14 ms | 20% | — |
| `cst/sha256/nested-1` | 38.0 µs | 34.2 µs – 106 µs | 9% | 7.7% |
| `cst/parse/nested-2` | 640 µs | 484 µs – 2.00 ms | 23% | — |
| `cst/sha256/nested-2` | 80.8 µs | 62.4 µs – 82.9 µs | 3% | 12.6% |
| `cst/parse/nested-3` | 719 µs | 625 µs – 1.06 ms | 13% | — |
| `cst/sha256/nested-3` | 93.6 µs | 79.8 µs – 110 µs | 12% | 13.0% |
| `cst/parse/nested-4` | 915 µs | 751 µs – 1.30 ms | 18% | — |
| `cst/sha256/nested-4` | 127 µs | 116 µs – 205 µs | 8% | 13.9% |
| `cst/parse/volume-1000` | 166 ms | 97.7 ms – 233 ms | 9% | — |
| `cst/sha256/volume-1000` | 15.8 ms | 12.8 ms – 22.3 ms | 9% | 9.5% |

SHA-256 over the identity preimages accounts for 5% to 14% of parse time on
these inputs. Most of the cost of a parse lies elsewhere. At depth 4 the parse
costs 2.4 times its depth-0 time, while the SHA-256 share adds only 109 µs of
the 540 µs difference.

### Model layer (`benches/model.rs`)

Input: a Semantic IR 2.0.0 document carried through FR-154 intake
(`intake::admit`, then `intake::read_records`, then `DomainPackage::new`). The
document holds *n* object types, each with one native `Boolean` field. It has a
supertype chain `C0 <- C1 <- … <- C{depth}`, filler types whose supertype is
`C0` (one object universe), and one closed population `Pop` whose member type
is `C0`. The size sweep uses depth 4. Record counts, from
`qsl-bench-probe model`:

- *n* = 250: 186,790 B, 501 records.
- *n* = 1,000: 2,001 records.
- *n* = 4,000: 3,001,505 B, 8,001 records, 12,005 effective declarations.

Stage inputs are prepared outside the timing. Admission uses a 100-member
population whose objects are all of type `C4`.

| Benchmark | Input size | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- | --- |
| `model/intake/admit/250` | 250 types | 3.12 ms | 2.81 ms – 5.69 ms | 12% | 1 `serde_json` parse plus JCS digest |
| `model/intake/read_records/250` | 250 types | 15.8 ms | 11.8 ms – 18.4 ms | 16% | 2 parses (`semantic_ir` plus `serde_json`) = 3.5 ms |
| `model/parse/serde_json/250` | 186,790 B | 1.78 ms | 1.68 ms – 3.01 ms | 8% | One bare parse (F13) |
| `model/parse/semantic_ir/250` | 186,790 B | 1.73 ms | 1.59 ms – 4.10 ms | 12% | One bare parse (F13) |
| `model/intake/admit/1000` | 1,000 types | 18.2 ms | 14.6 ms – 22.4 ms | 20% | — |
| `model/intake/read_records/1000` | 1,000 types | 117 ms | 80.7 ms – 224 ms | 26% | read_records/250: 7.4 times the time for 4 times the size |
| `model/parse/serde_json/1000` | 1,000 types | 10.1 ms | 8.26 ms – 12.3 ms | 10% | — |
| `model/parse/semantic_ir/1000` | 1,000 types | 7.63 ms | 6.97 ms – 14.9 ms | 16% | — |
| `model/intake/admit/4000` | 4,000 types | 65.1 ms | 54.2 ms – 68.7 ms | 12% | — |
| `model/intake/read_records/4000` | 4,000 types | 2.38 s | 1.83 s – 5.95 s | 23% | read_records/1000: 20 times the time for 4 times the size, so quadratic |
| `model/parse/serde_json/4000` | 3,001,505 B | 42.7 ms | 31.6 ms – 48.8 ms | 9% | — |
| `model/parse/semantic_ir/4000` | 3,001,505 B | 31.2 ms | 28.7 ms – 43.9 ms | 28% | — |
| `model/normalize/250` | 501 records | 23.9 ms | 17.5 ms – 30.6 ms | 14% | — |
| `model/object_universe_of/250` | 501 records | 12.6 ms | 9.56 ms – 16.0 ms | 8% | The unmetered rebuild admission runs (F5) |
| `model/admit_binding/250` | 501 records, 100 members | 12.6 ms | 9.16 ms – 13.8 ms | 9% | object_universe_of/250 |
| `model/admit_invocation/250` | Same, admitted as pre and post | 22.8 ms | 16.7 ms – 65.5 ms | 27% | 2 × admit_binding/250 = 25.2 ms |
| `model/normalize/1000` | 2,001 records | 114 ms | 78.1 ms – 141 ms | 24% | — |
| `model/object_universe_of/1000` | 2,001 records | 53.8 ms | 43.0 ms – 86.6 ms | 20% | — |
| `model/admit_binding/1000` | 2,001 records, 100 members | 65.6 ms | 42.5 ms – 104 ms | 35% | object_universe_of/1000 |
| `model/admit_invocation/1000` | Same, pre and post | 168 ms | 82.7 ms – 317 ms | 51% | 2 × admit_binding/1000 = 131 ms |
| `model/normalize/4000` | 8,001 records | 689 ms | 457 ms – 708 ms | 8% | — |
| `model/object_universe_of/4000` | 8,001 records | 454 ms | 281 ms – 536 ms | 9% | — |
| `model/admit_binding/4000` | 8,001 records, 100 members | 511 ms | 284 ms – 529 ms | 6% | object_universe_of/4000: the rebuild is 89% of admission |
| `model/admit_invocation/4000` | Same, pre and post | 959 ms | 648 ms – 1.38 s | 11% | 2 × admit_binding/4000 = 1.02 s |

`model/all_instances/<target>/<depth>` runs `allInstances` over 1,000 members.
Each member's most-specific type is `C{depth}`, and the package has 1,000 types
(2,001 records). The `root` target queries `C0`, which walks all *depth*
generalization steps per member. The `own` target queries `C{depth}` itself,
which needs no walk.

| Benchmark | Ancestors walked per member | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- | --- |
| `model/all_instances/root/1` | 1 | 751 µs | 542 µs – 1.01 ms | 13% | own/1 |
| `model/all_instances/own/1` | 0 | 475 µs | 317 µs – 1.11 ms | 28% | — |
| `model/all_instances/root/8` | 8 | 3.34 ms | 2.61 ms – 4.61 ms | 9% | own/8 |
| `model/all_instances/own/8` | 0 | 594 µs | 327 µs – 949 µs | 37% | — |
| `model/all_instances/root/32` | 32 | 11.3 ms | 8.92 ms – 17.1 ms | 21% | own/32 |
| `model/all_instances/own/32` | 0 | 532 µs | 361 µs – 674 µs | 17% | — |
| `model/all_instances/root/120` | 120 | 54.5 ms | 43.4 ms – 65.8 ms | 17% | own/120: 77 times slower |
| `model/all_instances/own/120` | 0 | 710 µs | 386 µs – 1.40 ms | 46% | — |

### Evaluator: one `CheckedPackage::call` of `f0(5)` on a checked, linked *n*-function chain (`benches/evaluator.rs`)

A call on a chain of *n* functions pushes *n* call frames. Checking and linking
happen outside the timing.

| Benchmark | Call frames | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- | --- |
| `evaluator/call_chain/1` | 1 | 500 ns | 318 ns – 900 ns | 16% | Fixed cost of one call |
| `evaluator/call_chain/10` | 10 | 3.58 µs | 2.52 µs – 5.82 µs | 11% | call_chain/1 |
| `evaluator/call_chain/100` | 100 | 35.3 µs | 24.2 µs – 42.1 µs | 16% | call_chain/10 |
| `evaluator/call_chain/1000` | 1,000 | 439 µs | 246 µs – 1.03 ms | 33% | call_chain/100 |

Cost per call frame is the slope between two sizes, which removes the fixed
cost of the call:

| Slope between | Cost per call frame |
| --- | --- |
| 1 and 10 frames | 342 ns |
| 10 and 100 frames | 352 ns |
| 100 and 1,000 frames | 449 ns |

The cost per frame rises with depth. The one-shot probe (`qsl-bench-probe eval`)
measured one call on 4,000 frames at 2.52 ms, or 630 ns per frame. The 179 MB
peak RSS of that process comes from checking the 4,000-function chain first,
not from the call.

## F5, F6, F7 and F13: the four findings the evaluation did not time

### F5: population admission builds twice with unlimited limits. Confirmed.

- **Measured.** `admit_binding` over 4,000 types costs 511 ms. The unmetered
  `object_universe_of` alone costs 454 ms, so the rebuild is 89% of admission.
- **Compared with normalization.** The caller has already paid for the metered
  normalization that admission receives: `normalize` costs 689 ms.
- **Invocation.** `admit_invocation` costs 959 ms, or 1.9 times `admit_binding`,
  because it runs the rebuild once for the pre binding and once for the post
  binding (`qsl-semantics/src/model/population.rs:1281` and `:1285`).
- **Net effect.** One invocation at this size spends about 2 × 454 ms on
  normalization work the caller has already paid for.
- **Unlimited limits.** The rebuild calls `build` with
  `ModelNormalizationLimits::UNLIMITED` (`normalize.rs:2974`, `:2998`). This is
  read from code, not timed. The timing confirms the cost.
- **Magnitude by size.** The rebuild is 100% of `admit_binding` at 250 types
  (12.6 ms each) and 82% at 1,000 types (53.8 ms of 65.6 ms). The 1,000-type
  rows are noisy.

### F6: model queries do O(N×A) conformance work. Confirmed for A; N held at 1,000.

- **Measured.** `allInstances<C0>` over 1,000 members costs 751 µs at 1
  ancestor, 3.34 ms at 8, 11.3 ms at 32 and 54.5 ms at 120.
- **Comparison point.** Querying the members' own type costs 475 to 710 µs at
  every depth.
- **Slope.** 452 ns per member per ancestor, from the slope between 1 and 120
  ancestors, which is linear in A.
- **Not measured: growth in N.** Only N = 1,000 was run. The per-member walk
  (`population.rs` `all_instances`, which calls `type_conforms` once per
  member) makes linear growth in N a reading of the code, not a measurement.
- **Not measured: other claims in QSL-202.** These are the per-call rebuild of
  `ConformanceIndex` and the per-query rebuild of `reverse_catalog`
  (`value/model_query.rs`). This bench times `population::all_instances`
  directly, not through `value::model_query`.

### F7: checker lookups do about 10^9 string comparisons at 5,000 declarations. Re-scoped.

The quadratic lookups are real. They dominate the check when there are no
calls, but not on a call chain. The main site is not one QSL-205 lists, and the
count is about 10^7 to 10^8, not 10^9.

- **5,000 independent declarations (227 ms).** A `perf record --call-graph
  dwarf` profile of `qsl-bench-probe check independent 5000` attributes the
  samples, inclusive, as follows:
  - 64%: the duplicate-name check in `PackageDeclarations::check`
    (`qsl-semantics/src/check/mod.rs:440-447`). For every function it filters
    every other function by `name ==`, so it runs N² = 25 million string
    comparisons. It holds most of the `memcmp` samples. QSL-205 does not list
    this site.
  - 11%: `OccurrenceMap::record` (`check/family.rs:1172-1178`).
  - 9%: the termination check (`check/termination.rs`), whose component filter
    is O(N²) even with no call edges.
  - 6.5%: the per-declaration family check.
- **Chain of 4,000 (6.8 s).** A flat `perf` profile attributes 86% of samples
  to the termination check: 68.6% to the component filter at
  `termination.rs:108-114` and 17.2% to building the per-function reachability
  sets at `termination.rs:90-101`. `OccurrenceMap::record` is 1.2%,
  `check_application` 0.15%, and the duplicate-name check 0.4%. On this shape
  the finding is QSL-203's, not F7's.
- **The comparison count.** This is derived from the code, not counted. At
  5,000 declarations:
  - 25 × 10^6 name comparisons in the duplicate-name check;
  - 12.5 × 10^6 `NodeKey` comparisons in `OccurrenceMap::record` with no calls,
    or 50 × 10^6 on a call chain with 10,000 occurrences;
  - 12.5 × 10^6 on a chain in `check_application`'s linear signature search.

  That is about 4 × 10^7 comparisons without calls and 9 × 10^7 on a chain.
  The timings agree: 227 ms, not the seconds that 10^9 comparisons would take.
- **Re-scope for QSL-205.** Add the duplicate-name check
  (`check/mod.rs:440-447`) to the sites. Measure QSL-205 on
  `checker/independent/*`, where lookups dominate, rather than on
  `checker/chain/*`, where termination hides them until QSL-203 lands.

### F13: model intake parses the same bytes three times. Confirmed, but minor. The real intake cost is elsewhere.

- **The three parses.** At `89326999` intake parses the admitted bytes three
  times:
  - `serde_json` in `admit` (`qsl-semantics/src/model/intake.rs:304`);
  - `agent_ix_semantic_ir::json::parse` in `validate_with_semantic_ir` (`:543`);
  - `serde_json` again in `read_records` (`:1519`).
- **What they cost at 4,000 types.** 42.7 + 31.2 + 42.7 ≈ 117 ms. The two
  redundant parses cost about 74 ms. That is 3% of the 2.45 s intake
  (`admit` at 65.1 ms plus `read_records` at 2.38 s).
- **What they cost at 250 types.** The redundant parses cost about 3.5 ms of
  18.9 ms, or 19%.
- **What dominates instead.** `read_records` grows quadratically: 15.8 ms,
  117 ms and 2.38 s at 250, 1,000 and 4,000 types. A `perf record --call-graph
  dwarf` profile of `qsl-bench-probe model 4000 4 100` attributes 72% of all
  samples to `agent_ix_semantic_ir::decide`, and 68% to one collect inside
  it. The collect is FCD's `constructs::frames`
  (`crates/semantic-ir/src/constructs.rs:645-646` at FCD
  `1572ba4ba8fad0e94e9a0b19a569f82a5181b488`). `frames` calls
  `document_features(document)`, which collects every field of every type, once
  per type, even for a type with no operations. That is O(T × F) in the FCD
  validator QSL calls, not in QSL.
- **Recommendation.** Fixing F13 alone saves about 3% at scale. The quadratic
  validator needs an FCD ticket: hoist `document_features` out of the per-type
  loop in `decide`.

## Engineering-assurance record

The same baseline is recorded as engineering-assurance MeasurementPlans, one
per axis:

- `spec/assurance/MP-001-parser-wall-time.md`
- `spec/assurance/MP-002-checker-wall-time.md`
- `spec/assurance/MP-003-cst-wall-time.md`
- `spec/assurance/MP-004-model-wall-time.md`
- `spec/assurance/MP-005-evaluator-wall-time.md`

Their five-run medians are stored as quoin measurement collections under
`spec/evidence/measurements/qsl196-baseline-<axis>-20260923.json`, and
`quoin report --repo .` shows them. This file remains the source of truth for
the stated variance and the findings. A collection observation carries the
median only.

## What this baseline does not cover

- **Memory.** Peak RSS is recorded only for the one-shot probe runs. The
  criterion suites time wall clock and nothing else.
- **Sizes.** The criterion suites stop at 2,000-function chains and 4,000
  model types, so that all five suites run in about ten minutes. The 4,000 and
  8,000 chains are one-shot probe runs.
- **The native S1 parser.** `src/parser.rs`, the native parser QSL-197 also
  changes, is not benchmarked. Only `qsl_cst::parse` is.
