# QSL-196 performance baseline

This file records the benchmark set, its noise floor, and the rule that QSL-202
to QSL-206 use to claim an improvement. Every figure below comes from a run of
the benchmarks in this crate. Nothing is estimated or carried over from the
architecture evaluation.

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
`cargo bench --locked -p qsl-bench --bench <axis>`, and criterion options go
through `BENCH_ARGS`.

Every input is built through the layers' public API. So a rewrite behind that
API needs no bench change. A change to the API itself does, the way QSL-201's
new `intake::admit` return type did. All model-intake calls go through
`src/model.rs`, so such a change is one edit there. Each generator has a
smallest-size `#[test]`, so a generator that stops building its input fails in
`make ci` rather than at bench time.

## How to claim an improvement

**The absolute tables below are informational.** On this shared machine the
same code measured in two different sessions differs by up to half. For
example, `model/object_universe_of/4000` on unchanged code measured 454 ms in
the first session and 311 ms in the second (a 31% "gain"). A comparison
against these tables alone therefore proves nothing.

A candidate revision claims an improvement on a benchmark only by winning one
**interleaved same-session A/B run**:

1. **Build both sides.** Build the baseline revision (A) and the candidate
   revision (B) in two worktrees, each with its own target directory.
2. **Alternate the rounds.** Run A, B, A, B … for five rounds each, one
   `make bench-<axis>` per round, on one machine and in one sitting. Record
   `/proc/loadavg` before and after every round.
3. **Take each side's median.** Per benchmark, each side's figure is the median
   of its five criterion point estimates.
4. **Take the session's noise.** The session's noise is the larger of the two
   sides' MAD/median, where MAD is the median absolute deviation of the five
   point estimates.
5. **Apply the claim.** B improves on the benchmark only when B's median is
   below A's median by more than **max(the stated variance in the tables below,
   the session's noise)**. Anything smaller is recorded as no change.
6. **Record every round** of the session, both sides. Do not rerun a session
   until it passes.

Criterion's own `--save-baseline` / `--baseline` compares one run against one
run, not five interleaved rounds. It does not meet this rule.

## Where and when this was measured

| Item | Value |
| --- | --- |
| Machine | Intel Core i9-10900K @ 3.70 GHz, 20 logical CPUs (`nproc`), 62 GiB RAM, WSL2 Linux 6.18.33.2-microsoft-standard-WSL2 |
| Toolchain | rustc 1.98.1 (48a229cea 2026-09-01), `bench` profile (release) |
| Harness | criterion 0.8.2, default features off. Defaults apply unless a bench overrides them: 3 s warm-up, 100 samples, 5 s measurement. |
| Checker and evaluator axes | 5 runs, 2026-09-23 17:22 to 18:23 (UTC-7), at `4dd02fb1`. That revision's checker, evaluator, package, forms and kernel sources are identical to this branch's base, `25ee38ff`. Load average ranged from 3.3 to 26.6, with two other QSL builds running. |
| Parser, CST and model axes | 5 runs, 2026-09-23 19:39 to 20:40 (UTC-7), at `46113bb5`. This is rebased on `25ee38ff` (QSL-201) and includes the qsl-cst identity-byte counter the CST axis reads. Load average ranged from 4.2 to 11.6. |

These three axes were re-measured because each changed what it measures:

- **Parser:** benchmark IDs now carry the parse outcome.
- **CST:** qsl-cst's parse path now counts the bytes it hashes.
- **Model:** QSL-201 changed intake, and new benchmarks were added.

## How the figures and the variance are computed

- **Figure.** The median of five point estimates, one per complete run. Each
  point estimate is criterion's estimate of wall time per iteration.
- **Range.** The fastest and slowest of the five point estimates.
- **Stated variance.** The larger of two measures:
  - the MAD of the five point estimates, divided by their median;
  - the widest criterion 95% confidence-interval half-width in any run, divided
    by that run's estimate.

  This is the table variance that the claim rule above uses.

## Baseline tables

Each row gives the unit (wall time per call), the input size in the unit that
layer counts, and the comparison point. The comparison point is the row the
figure is read against.

### Parser: one `qsl_cst::parse` call (`benches/parser.rs`)

Inputs, both parsed at default `Limits`:

- `parser/depth/<outcome>/d`: one function whose body nests *d* parenthesis
  pairs, such as `((x + x) + x)` for *d* = 2.
- `parser/volume/<outcome>/n`: *n* one-line functions,
  `function fI using Complete (x: Integer): Integer pure { x }`.

`<outcome>` (`admitted`, `recovered` or `refused`) is the input's parse outcome,
decided once at setup. When QSL-197 turns a refusal into a parse, the benchmark
gets a new ID (`parser/depth/admitted/5`). It is not compared against the time
it took to refuse.

| Benchmark | Input size | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- | --- |
| `parser/depth/admitted/0` | 243 B, 52 tokens, 21 nodes | 270 µs | 256 µs – 292 µs | 3% | Floor for the depth rows |
| `parser/depth/admitted/1` | 249 B | 388 µs | 369 µs – 398 µs | 3% | depth 0 |
| `parser/depth/admitted/2` | 255 B | 547 µs | 533 µs – 602 µs | 3% | depth 0 |
| `parser/depth/admitted/3` | 261 B | 636 µs | 621 µs – 841 µs | 3% | depth 0 |
| `parser/depth/admitted/4` | 267 B, 76 tokens, 81 nodes (deepest that parses) | 832 µs | 781 µs – 1.06 ms | 6% | depth 0: 3.1 times the time for 10% more bytes |
| `parser/depth/refused/5` | 273 B; `resource_exhausted` / `insufficient-next-charge` | 310 µs | 287 µs – 352 µs | 8% | depth 4. Time to reach the refusal. |
| `parser/depth/refused/8` | 291 B; same cause | 312 µs | 285 µs – 372 µs | 9% | refused/5 |
| `parser/volume/admitted/100` | 100 functions, 6,269 B | 8.44 ms | 8.31 ms – 11.9 ms | 3% | volume/1000 |
| `parser/volume/admitted/500` | 500 functions, 31,069 B | 44.1 ms | 42.9 ms – 65.4 ms | 6% | volume/100: 5.2 times, linear |
| `parser/volume/admitted/1000` | 1,000 functions, 62,069 B, 26,026 tokens, 18,003 nodes | 107 ms | 87.1 ms – 156 ms | 14% | volume/500: 2.4 times for 2 times. About 107 µs per function, or 0.58 MB/s. |
| `parser/volume/admitted/2000` | 2,000 functions, 125,069 B, 36,003 nodes (largest that parses) | 194 ms | 182 ms – 261 ms | 9% | volume/1000: 1.8 times, linear |
| `parser/volume/refused/3000` | 3,000 functions, 188,069 B; `resource_exhausted`, "complete grammar work or nesting budget exhausted" | 134 ms | 117 ms – 149 ms | 11% | volume/2000. Time to reach the refusal. QSL-197 AC 5 requires this input to parse. |

Refusal boundary, from `qsl-bench-probe parse`, which tries depths 0 to 64:
depth 4 is the deepest nesting that parses, and every depth from 5 to 64 is
refused. 4,000 functions is refused with "CST leaf budget exhausted".

### Checker: one `PackageDeclarations::check` over the whole package (`benches/checker.rs`)

Inputs:

- `checker/chain/n`: an *n*-function call chain, `f0(x) = f1(x) … f{n-1}(x) = x`.
- `checker/independent/n`: *n* functions `fI(x) = x + 1` with no calls between
  them.

Each input is checked once at setup and must succeed. Building the declarations
is excluded from the timing.

| Benchmark | Input size | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- | --- |
| `checker/chain/250` | 250 declarations, 249 calls | 5.97 ms | 4.45 ms – 7.39 ms | 12% | independent/1000 |
| `checker/chain/1000` | 1,000 declarations | 104 ms | 90.0 ms – 118 ms | 13% | chain/250: 17 times the time for 4 times the size |
| `checker/chain/2000` | 2,000 declarations | 564 ms | 453 ms – 592 ms | 12% | chain/1000: 5.4 times the time for 2 times the size |
| `checker/independent/1000` | 1,000 declarations, no calls | 12.0 ms | 8.87 ms – 13.1 ms | 15% | chain/1000: 8.7 times faster at the same size |
| `checker/independent/5000` | 5,000 declarations, no calls | 227 ms | 175 ms – 274 ms | 21% | independent/1000: 19 times the time for 5 times the size, so quadratic |

Chains too slow to sample repeatedly were measured with
`qsl-bench-probe check chain <n>`. Each size ran three times, in three separate
processes, at `4dd02fb1` (same checker code as above).

- **Wall time** is one `check`.
- **Peak RSS** is `VmHWM` of a process that built and checked only that chain.
- **Stated variance** is MAD/median over the three runs.

| Input size | Wall time, 3 runs | Median | Stated variance | Peak RSS, 3 runs | Median RSS | RSS variance | Comparison point |
| --- | --- | --- | --- | --- | --- | --- | --- |
| chain of 4,000 | 6,782 ms, 4,956 ms, 8,580 ms | 6.78 s | 27% | 179,140, 179,128, 179,184 KiB | 179 MB | 0.01% | QSL-203 reports 7.4 s and 178 MB from the evaluation, within this variance |
| chain of 8,000 | 38,165 ms, 29,812 ms, 33,564 ms | 33.6 s | 11% | 681,784, 681,716, 681,884 KiB | 682 MB | 0.01% | chain of 4,000: 5.0 times the time and 3.8 times the memory for 2 times the size |

The smaller sizes were measured once each by the probe:

| Input | Wall time | Peak RSS |
| --- | --- | --- |
| chain of 250 | 6.5 ms | 5.4 MB |
| chain of 1,000 | 134 ms | 17 MB |
| chain of 2,000 | 661 ms | 51 MB |
| independent 1,000 | 18 ms | 7.2 MB |
| independent 5,000 | 266 ms | 21 MB |

### CST identity hashing (`benches/cst.rs`)

`qsl_cst::parse` gives every CST node a `StableNodeId`. The ID is a SHA-256
digest over a preimage that holds the node's whole source slice and every
ancestor's production name (`LosslessCst::new`).

- `cst/parse/<input>` times the whole parse.
- `cst/sha256/<input>` times SHA-256 alone over a buffer the size of that
  input's hashed bytes, split into one message per node.

The hashed-byte counts come from `LosslessCst::identity_preimage_bytes`, which
qsl-cst's own preimage builder counts as it hashes. They were printed by
`qsl-bench-probe cst`.

**This axis is temporary.** QSL-200, folded into QSL-197, removes the eager
per-node digest. After it lands, parsing hashes no per-node data, and
`identity_preimage_bytes` reports the document-revision preimage alone, or
nothing. The `cst/sha256` rows then time a buffer of that size. The drop to
near zero is the expected effect of QSL-200, not a regression or a measurement
fault. Whichever of QSL-197 and this PR lands second updates the bench.

| Input | Source bytes | CST nodes | Bytes hashed | Hashed per source byte | Of which: source slices / ancestor names |
| --- | --- | --- | --- | --- | --- |
| nested-0 | 243 | 21 | 3,369 | 13.9 | 593 / 1,442 |
| nested-1 | 249 | 36 | 7,405 | 29.7 | 716 / 4,529 |
| nested-2 | 255 | 51 | 12,881 | 50.5 | 899 / 8,996 |
| nested-3 | 261 | 66 | 19,797 | 75.9 | 1,142 / 14,843 |
| nested-4 | 267 | 81 | 28,153 | 105.4 | 1,445 / 22,070 |
| volume-1000 | 62,069 | 18,003 | 2,671,362 | 43.0 | 231,026 / 1,416,026 |

Hashed bytes per source byte grow about quadratically with nesting depth. The
ancestor-name paths dominate: 78% of the hashed bytes at depth 4.

| Benchmark | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- |
| `cst/parse/nested-0` | 269 µs | 260 µs – 289 µs | 3% | `cst/sha256/nested-0` |
| `cst/sha256/nested-0` | 15.3 µs | 14.9 µs – 18.9 µs | 3% | 5.7% of `cst/parse/nested-0` |
| `cst/parse/nested-1` | 377 µs | 371 µs – 414 µs | 4% | `cst/sha256/nested-1` |
| `cst/sha256/nested-1` | 34.4 µs | 34.3 µs – 41.1 µs | 3% | 9.1% of the parse |
| `cst/parse/nested-2` | 496 µs | 486 µs – 559 µs | 3% | `cst/sha256/nested-2` |
| `cst/sha256/nested-2` | 62.0 µs | 61.8 µs – 73.7 µs | 2% | 12.5% of the parse |
| `cst/parse/nested-3` | 622 µs | 600 µs – 712 µs | 4% | `cst/sha256/nested-3` |
| `cst/sha256/nested-3` | 80.3 µs | 80.0 µs – 85.6 µs | 2% | 12.9% of the parse |
| `cst/parse/nested-4` | 802 µs | 739 µs – 924 µs | 3% | `cst/sha256/nested-4`; nested-0: 3.0 times |
| `cst/sha256/nested-4` | 120 µs | 116 µs – 121 µs | 3% | 15.0% of the parse |
| `cst/parse/volume-1000` | 112 ms | 95.3 ms – 146 ms | 14% | `cst/sha256/volume-1000` |
| `cst/sha256/volume-1000` | 12.9 ms | 12.6 ms – 13.3 ms | 2% | 11.5% of the parse |

SHA-256 over the identity preimages accounts for 6% to 15% of parse time on
these inputs. At depth 4 the parse costs 533 µs more than at depth 0. The
SHA-256 share of that increase is 105 µs.

### Model layer (`benches/model.rs`)

Input: a Semantic IR 2.0.0 document carried through FR-154 intake
(`intake::admit`, then `intake::read_records`, then `DomainPackage::new`).

- **Types.** *n* object types, each with one native `Boolean` field.
- **Supertype chain.** `C0 <- C1 <- … <- C{depth}`.
- **Filler types.** Each has supertype `C0`, so the package is one object
  universe.
- **Population.** One closed population `Pop`, whose member type is `C0`.

The size sweep uses depth 4. Record counts, from `qsl-bench-probe model`:

| *n* | Document bytes | Records | Effective declarations |
| --- | --- | --- | --- |
| 250 | 186,790 | 501 | 755 |
| 1,000 | 745,540 | 2,001 | 3,005 |
| 4,000 | 3,001,505 | 8,001 | 12,005 |

Stage inputs are prepared outside the timing. Admission uses a 100-member
population of type `C4`.

Intake and admission, by package size:

| Benchmark | Input size | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- | --- |
| `model/intake/parse/250` | 186,790 B | 2.59 ms | 2.28 ms – 3.38 ms | 12% | intake/admit/250: the parse is 62% of admission |
| `model/intake/admit/250` | 250 types | 3.83 ms | 3.31 ms – 6.34 ms | 13% | intake/parse/250 |
| `model/intake/read_records/250` | 250 types, parsed | 8.01 ms | 7.27 ms – 15.0 ms | 9% | intake/parse/250: 3.1 times |
| `model/intake/parse/1000` | 745,540 B | 15.6 ms | 13.6 ms – 19.9 ms | 12% | intake/parse/250: 6.0 times for 4 times |
| `model/intake/admit/1000` | 1,000 types | 19.6 ms | 19.5 ms – 28.3 ms | 7% | intake/parse/1000 |
| `model/intake/read_records/1000` | 1,000 types | 66.4 ms | 59.7 ms – 70.3 ms | 6% | read_records/250: 8.3 times for 4 times |
| `model/intake/parse/4000` | 3,001,505 B | 65.1 ms | 58.8 ms – 75.9 ms | 10% | intake/admit/4000 |
| `model/intake/admit/4000` | 4,000 types | 90.6 ms | 75.8 ms – 111 ms | 6% | intake/parse/4000 |
| `model/intake/read_records/4000` | 4,000 types | 2.48 s | 1.05 s – 4.18 s | 51% | read_records/1000: 37 times for 4 times, so quadratic |
| `model/normalize/250` | 501 records | 18.9 ms | 14.3 ms – 30.3 ms | 25% | normalize/1000 |
| `model/object_universe_of/250` | 501 records | 8.20 ms | 7.58 ms – 14.3 ms | 19% | admit_binding/250 (F5) |
| `model/admit_binding/250` | 501 records, 100 members | 8.34 ms | 7.97 ms – 9.89 ms | 6% | object_universe_of/250 |
| `model/admit_invocation/250` | Same, pre and post | 18.6 ms | 16.0 ms – 19.6 ms | 8% | admit_binding/250: 2.2 times |
| `model/conformance/resolve_redefinition_target/250` | 501 records | 321 µs | 317 µs – 335 µs | 14% | resolve_redefinition_target/1000 |
| `model/normalize/1000` | 2,001 records | 81.8 ms | 74.5 ms – 99.5 ms | 12% | normalize/250: 4.3 times for 4 times |
| `model/object_universe_of/1000` | 2,001 records | 51.8 ms | 46.6 ms – 89.0 ms | 13% | admit_binding/1000 |
| `model/admit_binding/1000` | 2,001 records, 100 members | 48.4 ms | 46.0 ms – 72.8 ms | 18% | object_universe_of/1000 |
| `model/admit_invocation/1000` | Same, pre and post | 90.2 ms | 83.4 ms – 120 ms | 8% | admit_binding/1000: 1.9 times |
| `model/conformance/resolve_redefinition_target/1000` | 2,001 records | 1.77 ms | 1.33 ms – 4.02 ms | 28% | resolve_redefinition_target/250: 5.5 times for 4 times |
| `model/normalize/4000` | 8,001 records | 490 ms | 483 ms – 820 ms | 12% | normalize/1000: 6.0 times for 4 times |
| `model/object_universe_of/4000` | 8,001 records | 311 ms | 297 ms – 464 ms | 5% | normalize/4000: 63% |
| `model/admit_binding/4000` | 8,001 records, 100 members | 343 ms | 316 ms – 498 ms | 5% | object_universe_of/4000 |
| `model/admit_invocation/4000` | Same, pre and post | 604 ms | 575 ms – 859 ms | 16% | admit_binding/4000: 1.8 times |
| `model/conformance/resolve_redefinition_target/4000` | 8,001 records | 16.4 ms | 15.8 ms – 24.1 ms | 14% | resolve_redefinition_target/1000: 9.3 times for 4 times |

`allInstances` walks the supertype chain once per member. Two sweeps measure it,
over a package of 1,000 types (2,001 records):

- **Ancestor sweep** (`model/all_instances/<target>/<depth>`): 1,000 members,
  each of type `C{depth}`. The `root` target queries `C0`, which walks *depth*
  generalization steps per member. The `own` target queries `C{depth}` itself,
  which needs no walk.
- **Member sweep** (`model/all_instances/members/<m>`): *m* members of an
  8-ancestor type, querying `C0`. The same query also runs through the
  evaluator's bridge (`model/query/evaluate_all_instances/<m>`), which rebuilds
  a reverse catalog of the binding's 1,000 types on every query (QSL-202).

| Benchmark | Input size | Median | Range (5 runs) | Stated variance | Comparison point |
| --- | --- | --- | --- | --- | --- |
| `model/all_instances/root/1` | 1,000 members, 1 ancestor | 549 µs | 535 µs – 654 µs | 6% | own/1 |
| `model/all_instances/own/1` | 1,000 members, 0 walked | 391 µs | 327 µs – 515 µs | 16% | root/1 |
| `model/all_instances/root/8` | 8 ancestors | 2.11 ms | 2.03 ms – 2.41 ms | 4% | own/8 |
| `model/all_instances/own/8` | 0 walked | 326 µs | 318 µs – 351 µs | 3% | root/8 |
| `model/all_instances/root/32` | 32 ancestors | 9.00 ms | 8.53 ms – 9.95 ms | 5% | own/32 |
| `model/all_instances/own/32` | 0 walked | 377 µs | 320 µs – 543 µs | 11% | root/32 |
| `model/all_instances/root/120` | 120 ancestors | 32.6 ms | 31.9 ms – 42.9 ms | 4% | own/120: 87 times |
| `model/all_instances/own/120` | 0 walked | 376 µs | 332 µs – 459 µs | 8% | root/120 |
| `model/all_instances/members/250` | 250 members, 8 ancestors | 512 µs | 488 µs – 670 µs | 3% | members/1000 |
| `model/query/evaluate_all_instances/250` | Same, via `value::model_query` | 678 µs | 653 µs – 1.05 ms | 5% | members/250: +166 µs |
| `model/all_instances/members/1000` | 1,000 members | 2.48 ms | 2.00 ms – 3.17 ms | 18% | members/250: 4.8 times for 4 times |
| `model/query/evaluate_all_instances/1000` | Same, via `value::model_query` | 2.93 ms | 2.35 ms – 3.55 ms | 14% | members/1000: +450 µs |
| `model/all_instances/members/4000` | 4,000 members | 9.62 ms | 9.59 ms – 15.8 ms | 4% | members/1000: 3.9 times for 4 times |
| `model/query/evaluate_all_instances/4000` | Same, via `value::model_query` | 10.1 ms | 8.87 ms – 11.5 ms | 4% | members/4000: difference within variance |

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

The three slopes differ by less than the rows' 16% to 33% stated variance. This
baseline does not show the per-frame cost changing with depth.

## F5, F6, F7 and F13: the four findings the evaluation did not time

### F5: population admission builds twice with unlimited limits. Confirmed.

- **Measured at 4,000 types, by per-run pairing.** Both benches ran in the same
  run, five times. The rebuild (`object_universe_of`) is 93% of
  `admit_binding` (median of the five per-run ratios, range 87% to 94%). In the
  first session, on the same code, the paired median was 99% (range 86% to
  109%).
- **Measured at 250 and 1,000 types.** The paired medians are 99% and 102%.
- **Conclusion.** Admission's cost is the rebuild, within the noise.
- **Invocation.** `admit_invocation` is 1.7 to 2.2 times `admit_binding`
  (paired medians), because it rebuilds once for the pre binding and once for
  the post binding (`qsl-semantics/src/model/population.rs:1281`, `:1285`).
- **Compared with normalization.** The caller has already paid for the metered
  normalization: 490 ms at 4,000 types.
- **Unlimited limits.** The rebuild calls `build` with
  `ModelNormalizationLimits::UNLIMITED` (`normalize.rs:2974`, `:2998`). This is
  read from code, not timed. The timing confirms the cost.

### F6: model queries do O(N×A) conformance work. Confirmed on both axes.

- **A axis (ancestors), 1,000 members.** `allInstances<C0>` costs 549 µs, 2.11
  ms, 9.00 ms and 32.6 ms at 1, 8, 32 and 120 ancestors. That is linear, at
  about 270 ns per member per ancestor. The first session measured 452 ns on
  the same code, which is another reason the claim rule is same-session only.
- **Comparison point.** Querying the members' own type costs 0.33 to 0.39 ms at
  every depth.
- **N axis (members), 8 ancestors.** 512 µs, 2.48 ms and 9.62 ms at 250, 1,000
  and 4,000 members. That is linear, at 2.0 to 2.5 µs per member.
- **QSL-202's other claims.**
  - `model/conformance/resolve_redefinition_target/<n>` times one conformance
    call. Each call builds `ConformanceIndex` over the whole package: 321 µs,
    1.77 ms and 16.4 ms at 501, 2,001 and 8,001 records.
  - `model/query/evaluate_all_instances/<m>` adds the per-query
    `reverse_catalog` rebuild over 1,000 types. It costs 166 µs and 450 µs over
    the direct query at 250 and 1,000 members. At 4,000 members the difference
    is within the variance.

### F7: checker lookups do about 10^9 string comparisons at 5,000 declarations. Re-scoped.

The quadratic lookups are real. They dominate the check when there are no
calls, but not on a call chain. The main site is not one QSL-205 lists. The
count is about 10^7 to 10^8, not 10^9.

- **5,000 independent declarations (227 ms).** A `perf record --call-graph
  dwarf` profile of `qsl-bench-probe check independent 5000` attributes the
  samples, inclusive, as follows:
  - 64%: the duplicate-name check in `PackageDeclarations::check`
    (`qsl-semantics/src/check/mod.rs:440-447`). For every function it filters
    every other function by `name ==`, so it runs N² = 25 million string
    comparisons. It holds most of the `memcmp` samples.
  - 11%: `OccurrenceMap::record` (`check/family.rs:1172-1178`).
  - 9%: the termination check.
  - 6.5%: the per-declaration family check.
- **Chain of 4,000 (6.8 s).** 86% of samples are in the termination check
  (68.6% component filter, `termination.rs:108-114`; 17.2% reachability sets,
  `:90-101`). `OccurrenceMap::record` is 1.2%, `check_application` 0.15%, and
  the duplicate-name check 0.4%. On this shape the finding is QSL-203's.
- **The comparison count.** Derived from the code, not counted. At 5,000
  declarations:
  - 25 × 10^6 name comparisons in the duplicate-name check;
  - 12.5 × 10^6 `NodeKey` comparisons in `OccurrenceMap::record`, or 50 × 10^6
    on a chain;
  - 12.5 × 10^6 on a chain in `check_application`'s linear signature search.

  That is about 4 × 10^7 comparisons without calls and 9 × 10^7 on a chain.
  The 227 ms timing agrees.
- **Re-scope for QSL-205.** Add the duplicate-name check
  (`check/mod.rs:440-447`). Measure on `checker/independent/*`, where lookups
  dominate, not on `checker/chain/*`, where termination hides them until
  QSL-203 lands.

### F13: model intake parses the same bytes three times. Confirmed at `89326999`, fixed by QSL-201. Intake's real cost is FCD's validator.

- **At `89326999`.** The first session measured three parses of the same bytes,
  about 117 ms at 4,000 types. The two redundant parses cost about 74 ms of a
  2.45 s intake (3%).
- **After QSL-201.** Intake parses once (`PackageDocument::parse`):
  `model/intake/parse/4000` is 65.1 ms. That is 3% of admit plus read_records
  at 4,000 types, and 20% at 250 and 1,000 types.
- **What dominates.** `read_records` grows quadratically: 8.0 ms, 66 ms and
  2.48 s at 250, 1,000 and 4,000 types.
  - A dwarf `perf` profile at 4,000 types attributes 72% of all samples to
    `agent_ix_semantic_ir::decide`, and 68% to one collect inside it.
  - The collect is FCD's `constructs::frames`
    (`crates/semantic-ir/src/constructs.rs:645-646` at FCD `1572ba4b`). It
    calls `document_features(document)`, which collects every field of every
    type, once per type. That is O(T × F), in the FCD validator QSL calls.
  - This was profiled at `89326999`. QSL-201 did not change the validator
    call.
- **The fix belongs to FCD: PLAT-1051.** Hoist `document_features` out of the
  per-type loop in `decide`.

## Engineering-assurance record

The same benchmark set is recorded as engineering-assurance MeasurementPlans,
one per axis, at definition version `-v2`:

- `spec/assurance/MP-001-parser-wall-time.md`
- `spec/assurance/MP-002-checker-wall-time.md`
- `spec/assurance/MP-003-cst-wall-time.md`
- `spec/assurance/MP-004-model-wall-time.md`
- `spec/assurance/MP-005-evaluator-wall-time.md`

MP-002 and MP-004 measure QSpec requirements that live in
`agent-ix/quire-specification`, so they target
`ix://agent-ix/quire-specification/FR-146` and `.../FR-154`, the form IT-012
already uses. Plain `quire validate` passes. `quire validate --okf` reports
both as `dangling reference`, because it resolves a cross-repo target by bare
ID inside this bundle. That is a known quire bug, and the targets are correct.

Each plan's decision rule is the same-session A/B rule above. The rule's
reference, `prior-collection`, is the baseline revision's collection from the
same session.

The five-run medians are stored, to four significant figures, as quoin
measurement collections under
`spec/evidence/measurements/qsl196-baseline-<axis>-v2.json`, and
`quoin report --repo .` shows them. Parser observations carry an `outcome`
dimension. This file remains the source of truth for the stated variance and
the findings.

## What this baseline does not cover

- **Memory.** Peak RSS is recorded only for the one-shot probe runs. The
  criterion suites time wall clock and nothing else.
- **Sizes.** The criterion suites stop at 2,000-function chains and 4,000
  model types. The 4,000 and 8,000 chains are one-shot probe runs.
- **The native S1 parser.** `src/parser.rs`, which QSL-197 also changes, is not
  benchmarked. Only `qsl_cst::parse` is.
- **`reverse_catalog` in the type count.** It is measured at 1,000 types only.
  It scales with the binding's type count, which the member sweep holds fixed.
