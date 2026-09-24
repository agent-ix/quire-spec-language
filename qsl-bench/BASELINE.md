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

Run benches with `-p qsl-bench`, never `--workspace`. A workspace build turns
on `quire-exact/test-support` through other crates' dev-dependencies, and the
meter then logs every charge (QSL-206).

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

Refusal boundary at 89326999, from `qsl-bench-probe parse`, which tries depths
0 to 64: depth 4 is the deepest nesting that parses, and every depth from 5 to
64 is refused. 4,000 functions is refused with "CST leaf budget exhausted".

Since QSL-197, nesting counts bracket pairs, so every depth from 0 to 63 in the
function body parses (the body's `{` is the 64th pair) and the depth rows
become `parser/depth/admitted/<d>`. `volume/3000` is still refused, now by the
syntax-node ceiling (54,003 nodes against 50,000): "syntax node ceiling of
50000 nodes exhausted". 4,000 functions is refused with "token ceiling of
100000 tokens exhausted".

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

**QSL-200 has landed.** A parse no longer hashes per-node data. A node's
identity is the revision digest, its span and its arena index; the structural
path is derived from parent links on request, and reuse is a one-to-one span
mapping over typed productions (`LosslessCst::reuse_map`). A parse computes
one identity digest, over the document-revision labels, whatever its node
count; qsl-cst's own unit test counts the digest calls.

- `cst/parse/<input>` times the whole parse. Throughput is source bytes.
- The `cst/sha256/<input>` rows timed the per-node digests QSL-200 removed and
  are gone with them. `qsl-bench-probe cst` now prints node counts only.

The tables below were measured at 89326999, before QSL-200, when every node
carried a SHA-256 `StableNodeId` over its whole source slice and every
ancestor's production name. They are kept as the record of what QSL-200
removed; the PR #380 A/B run is the comparison against them.

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
- **Fixed by QSL-204 (PR #387).** Admission now reads the package and its
  object universe from the effective view the caller normalized under its own
  limits, and does no normalization. The `model/object_universe_of/<n>` bench
  is gone with the function it timed. The rows above are kept as history.
  Back-to-back A/B in one session, base `f157f346` against PR head, criterion
  point estimates:

  | Bench | Base | QSL-204 | Change |
  |---|---|---|---|
  | `model/admit_binding/250` | 7.82 ms | 0.322 ms | −95.8% |
  | `model/admit_invocation/250` | 16.04 ms | 0.666 ms | −95.7% |
  | `model/admit_binding/1000` | 42.64 ms | 1.29 ms | −97.4% |
  | `model/admit_invocation/1000` | 87.65 ms | 1.65 ms | −98.1% |
  | `model/admit_binding/4000` | 318.3 ms | 8.31 ms | −97.4% |
  | `model/admit_invocation/4000` | 608.3 ms | 14.94 ms | −97.5% |

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

## QSL-203: termination by Tarjan's SCC. Improvement on every chain size.

QSL-203 replaced the termination check's per-function reachability sets and
all-pairs component filter with an iterative Tarjan SCC
(`qsl-semantics/src/check/termination.rs`). It was measured by the claim rule
above, in one session on 2026-09-23, 21:12 to 21:27 (UTC-7):

- A is `67f44989` (origin/main) and B is `59da8964` (the fix). Each side has
  its own worktree and target directory.
- The rounds ran A1, B1, A2, B2 … A5, B5. Each round ran one
  `make bench-checker`, then `qsl-bench-probe check chain <n>` for 250, 1,000,
  2,000, 4,000 and 8,000, one process per size.
- Load average ranged from 4.2 to 16.5. A QSL-197 parser A/B session and other
  QSL builds ran on the machine during the session.
- The apparatus is unchanged: `benches/checker.rs`, `src/check.rs` and
  `Cargo.toml` are byte-identical on both sides.

What each figure counts:

- **Wall time** is one `PackageDeclarations::check` over the whole package.
  Criterion rows are criterion point estimates. Probe rows are one `check` per
  process.
- **Peak RSS** is `VmHWM` of the probe process, which builds and checks only
  that chain.
- **Margin** is max(the stated variance, the session's MAD/median).

| Benchmark | A, 5 rounds | B, 5 rounds | A median | B median | Gain | Margin | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `checker/chain/250` | 5.56, 7.24, 5.71, 5.42, 4.75 ms | 1.32, 1.58, 1.34, 1.29, 1.40 ms | 5.56 ms | 1.34 ms | 76% | 12% | improvement |
| `checker/chain/1000` | 99.7, 136, 99.2, 96.0, 94.6 ms | 15.8, 15.7, 13.9, 14.0, 19.7 ms | 99.2 ms | 15.7 ms | 84% | 13% | improvement |
| `checker/chain/2000` | 458, 559, 541, 455, 544 ms | 45.4, 55.1, 45.4, 47.2, 49.5 ms | 541 ms | 47.2 ms | 91% | 12% | improvement |
| `checker/independent/1000` | 9.32, 10.5, 12.0, 9.19, 9.55 ms | 9.31, 10.1, 8.71, 8.60, 8.75 ms | 9.55 ms | 8.75 ms | 8% | 15% | no change |
| `checker/independent/5000` | 211, 218, 230, 204, 185 ms | 168, 171, 204, 167, 167 ms | 211 ms | 168 ms | 20% | 21% | no change |
| probe chain of 250 | 7.2, 6.0, 8.8, 4.8, 4.9 ms | 1.6, 1.5, 1.7, 1.5, 1.7 ms | 6.0 ms | 1.6 ms | 73% | 20% | improvement |
| probe chain of 1,000 | 136, 110, 183, 106, 90 ms | 18.0, 15.6, 16.0, 14.9, 14.6 ms | 110 ms | 15.6 ms | 86% | 19% | improvement |
| probe chain of 2,000 | 606, 600, 820, 457, 469 ms | 41.3, 49.3, 48.0, 44.5, 47.1 ms | 600 ms | 47.1 ms | 92% | 22% | improvement |
| probe chain of 4,000 | 6.65, 4.63, 6.24, 2.12, 2.13 s | 223, 225, 217, 195, 201 ms | 4.63 s | 217 ms | 95% | 44% | improvement |
| probe chain of 8,000 | 23.4, 25.9, 29.9, 12.7, 9.70 s | 867, 1,084, 833, 869, 894 ms | 23.4 s | 869 ms | 96% | 28% | improvement |

A's 4,000 and 8,000 rounds fell by more than half in rounds 4 and 5, as the
load average dropped from about 9 to 5. That is why A's session noise is 44%
at 4,000. B beats A by far more than that margin.

Peak RSS, 5 rounds each. The RSS variance is below 2% on both sides.

| Input | A median | B median | B / A |
| --- | --- | --- | --- |
| chain of 250 | 5,200 KiB | 4,540 KiB | 0.87 |
| chain of 1,000 | 16,928 KiB | 7,108 KiB | 0.42 |
| chain of 2,000 | 50,652 KiB | 10,452 KiB | 0.21 |
| chain of 4,000 | 178,860 KiB | 17,016 KiB | 0.10 |
| chain of 8,000 | 681,836 KiB | 30,892 KiB | 0.05 |

After the fix, B's peak RSS grows linearly: 1.8 times for 2 times the size from
4,000 to 8,000. **Check time is still superlinear.** B's probe medians grow 3.0,
4.6 and 4.0 times for each doubling from 1,000 to 8,000. So the chain check is
still quadratic, but the quadratic term is no longer termination. A
`perf record --call-graph dwarf` profile of B's `qsl-bench-probe check chain
8000` (1.19 s) attributes the samples, inclusive, as follows:

- 45%: the duplicate-name check (`check/mod.rs:440-447`), with its `memcmp`;
- 30%: `OccurrenceMap::record` (`check/family.rs:1164`);
- 19%: `check_application`'s linear signature search;
- 0.16%: the termination check.

These are F7's lookup sites, which belong to QSL-205. The inclusive shares
overlap, because `memcmp` samples unwind into more than one caller.

The session is recorded as four collections under
`spec/evidence/measurements/`, each holding every round and the load averages:

- **Criterion rows (MP-002):** `qsl203-ab-a-checker-v2.json` and
  `qsl203-ab-b-checker-v2.json`.
- **Probe rows (MP-006):** `qsl203-ab-a-probe-v1.json` and
  `qsl203-ab-b-probe-v1.json`. Wall time and peak RSS are separate
  `quantity` dimensions.


`quoin report --since 67f44989` compares A with B for both plans.

### Per-component cost of the termination pass

A review of the first fix found that `check` still did O(V) work per
recursive component. It allocated a V-bit membership set per component and a
V-length parent vector per refusal. So the pass was O(V × components) when
every function is its own recursive component. The fix records one component
id per member and shares one parent buffer.

Informal timing only: `check` in `termination.rs` alone, on N self-recursive
members with no measure, so N refused components. The command is
`cargo test --release -p qsl-semantics termination_scaling -- --ignored
--nocapture`.

| N | Before (1 run) | After (3 runs) |
| --- | --- | --- |
| 5,000 | 10.1 ms | 2.2, 2.5, 2.3 ms |
| 10,000 | 34.2 ms | 6.7, 4.4, 4.2 ms |
| 20,000 | 138 ms | 10.5, 9.2, 8.5 ms |
| 40,000 | 518 ms | 28.1, 18.6, 17.9 ms |

Before, the time grew 3.4 to 4.1 times per doubling, which is quadratic. After,
it grows about 2 times, which is linear. `qsl-bench-probe check
self-recursive <n>` times the same shape through the whole check. There, the
F7 lookup sites dominate, as they do on the chain.

## QSL-206: one meter, a charge count and no per-task location clone. Improvement on every chain size.

QSL-206 changed three things on the evaluator's per-frame path:

- `quire_exact::Meter` keeps a count of admitted charges, not a `Vec` of them.
- The evaluator's task loop borrows each task's node and clones a `Location`
  only when a halt reports one. Before, it cloned one on every task.
- `CheckedPackage::call` charges the top-level `function.call` to the
  caller's meter. So B does one more charge per call than A.

It was measured by the claim rule above, in one session on 2026-09-23, 23:49
to 23:56 (UTC-7):

- A is `55db8a4c` (origin/main) and B is `396f9503` (the fix). Each side has
  its own worktree and target directory.
- The rounds ran A1, B1, A2, B2 … A5, B5, one `make bench-evaluator` each.
- Load average ranged from 4.3 to 10.1.
- The apparatus is unchanged: `benches/evaluator.rs`, `src/check.rs` and
  `Cargo.toml` are byte-identical on both sides.

Each figure is a criterion point estimate of one `CheckedPackage::call` of
`f0(5)` on an *n*-function chain. **Margin** is max(the stated variance in the
evaluator table above, the session's MAD/median).

| Benchmark | A, 5 rounds | B, 5 rounds | A median | B median | Gain | Margin | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `evaluator/call_chain/1` | 333, 360, 340, 369, 320 ns | 278, 255, 264, 258, 264 ns | 340 ns | 264 ns | 22% | 16% | improvement |
| `evaluator/call_chain/10` | 2.52, 2.46, 2.61, 2.43, 2.36 µs | 2.02, 1.52, 1.39, 1.66, 1.43 µs | 2.46 µs | 1.52 µs | 38% | 11% | improvement |
| `evaluator/call_chain/100` | 24.1, 24.0, 23.9, 23.9, 24.5 µs | 14.2, 13.6, 13.4, 13.7, 13.4 µs | 24.0 µs | 13.6 µs | 43% | 16% | improvement |
| `evaluator/call_chain/1000` | 300, 295, 262, 264, 324 µs | 155, 159, 187, 160, 163 µs | 295 µs | 160 µs | 46% | 33% | improvement |

Cost per call frame is the slope between two sizes, computed per round. The
variance is the MAD/median of the five per-round slopes.

| Slope between | A median (MAD/median) | B median (MAD/median) |
| --- | --- | --- |
| 1 and 10 frames | 234 ns (3.1%) | 140 ns (11%) |
| 10 and 100 frames | 239 ns (0.3%) | 134 ns (0.2%) |
| 100 and 1,000 frames | 301 ns (11%) | 162 ns (2.1%) |
| 1 and 1,000 frames | 295 ns (10%) | 160 ns (1.8%) |

## QSL-205: checker lookups by map. Improvement on every checker benchmark; chain check near-linear.

QSL-205 replaced the checker's linear lookups with maps built once:

- `OccurrenceMap` keys spans by (node id, role); the next ordinal is that
  key's span count.
- `Scope` indexes type names, enum members (by enum name and case, parsed from
  `E::m`) and model operations. `Signatures` indexes function names. No
  formatted string is compared on a lookup path.
- The duplicate-name check groups declarations by name in one pass.
- `CheckedGraph` keeps its `Signatures` and an identity index. The evaluator
  reads each callee from the graph by index, so no callable list is built per
  evaluation.
- Lowering's `settle` resolves only the keys written since the previous settle.
  Before, it re-resolved every function and composite key per function group.
  This was the next hot spot after the listed sites: 19% of self time at
  8,000.

It was measured by the claim rule above, in one session on 2026-09-24, 01:37 to
01:53 (UTC-7):

- A is `ddc0083e` (origin/main, after QSL-156 A4b) and B is `61dd8188` (the
  fix). Each side has its own worktree and target directory.
- The rounds ran A1, B1, A2, B2 … A5, B5. Each round ran one
  `make bench-checker`, then `qsl-bench-probe check chain <n>` for 1,000,
  2,000, 4,000 and 8,000 and `qsl-bench-probe check independent <n>` for 1,000
  and 5,000, one process per input.
- Load average ranged from 5.6 to 13.1. Other agents' QSL builds and benches
  ran on the machine during the session.
- The apparatus is unchanged: `benches/checker.rs`, `src/check.rs` and
  `Cargo.toml` are byte-identical on both sides.

A is much slower than QSL-203's B on the same inputs (chain of 1,000: 93 ms
against 15.6 ms). A4b's lowering keys every node, and its cost is in both
sides.

What each figure counts:

- **Wall time** is one `PackageDeclarations::check` over the whole package.
  Criterion rows are criterion point estimates. Probe rows are one `check` per
  process.
- **Calls per declaration.** A chain of *n* has *n* − 1 calls, one per
  declaration but the last. An independent package has none.
- **Margin** is max(the stated variance, the session's MAD/median). Probe rows
  have no stated variance, so their margin is the session's.

| Benchmark | A, 5 rounds | B, 5 rounds | A median | B median | Gain | Margin | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `checker/chain/250` | 21.2, 20.4, 19.8, 15.9, 22.7 ms | 18.1, 17.9, 16.8, 14.1, 14.1 ms | 20.4 ms | 16.8 ms | 18% | 12% | improvement |
| `checker/chain/1000` | 95.6, 109, 112, 93.4, 107 ms | 77.9, 69.7, 87.0, 58.3, 65.2 ms | 107 ms | 69.7 ms | 35% | 13% | improvement |
| `checker/chain/2000` | 312, 513, 411, 297, 299 ms | 164, 140, 177, 123, 157 ms | 312 ms | 157 ms | 50% | 12% | improvement |
| `checker/independent/1000` | 155, 150, 126, 108, 114 ms | 75.0, 96.1, 75.6, 66.9, 67.6 ms | 126 ms | 75.0 ms | 40% | 15% | improvement |
| `checker/independent/5000` | 3.68, 4.50, 4.93, 2.34, 3.66 s | 394, 518, 361, 359, 353 ms | 3.68 s | 361 ms | 90% | 22% | improvement |
| probe chain of 1,000 | 90.6, 121, 106, 93.3, 92.6 ms | 84.0, 99.0, 80.2, 78.9, 61.1 ms | 93.3 ms | 80.2 ms | 14% | 5% | improvement |
| probe chain of 2,000 | 310, 459, 358, 299, 338 ms | 176, 166, 138, 135, 125 ms | 338 ms | 138 ms | 59% | 9% | improvement |
| probe chain of 4,000 | 1.11, 1.67, 1.50, 1.09, 1.23 s | 408, 439, 288, 269, 259 ms | 1.23 s | 288 ms | 77% | 11% | improvement |
| probe chain of 8,000 | 9.40, 11.3, 12.7, 4.36, 6.75 s | 899, 871, 624, 568, 578 ms | 9.40 s | 624 ms | 93% | 28% | improvement |
| probe independent 1,000 | 170, 120, 167, 106, 114 ms | 123, 94.8, 70.2, 67.4, 73.4 ms | 120 ms | 73.4 ms | 39% | 11% | improvement |
| probe independent 5,000 | 7.12, 6.30, 4.74, 2.10, 2.62 s | 733, 472, 379, 381, 368 ms | 4.74 s | 381 ms | 92% | 45% | improvement |

**Growth per doubling on the chain.** B's probe medians grow 1.72, 2.08 and
2.17 times from 1,000 to 8,000 (A: 3.62, 3.64 and 7.63), so QSL-205's
near-linear criterion (at most about 2.5 times per doubling) passes on the
medians. Taken round by round, B's ratios range over 1.68–2.09, 1.99–2.64 and
1.98–2.23. One round exceeds 2.5: round 2's 2,000 to 4,000 (439 ms / 166 ms =
2.64), under a load average of 13.1. Round 1's same step was 2.32, at 10.9.

Peak RSS, 5 rounds each. The RSS variance is below 0.5% on both sides.

| Input | A median | B median | B / A |
| --- | --- | --- | --- |
| chain of 1,000 | 15,556 KiB | 16,236 KiB | 1.04 |
| chain of 2,000 | 26,816 KiB | 27,800 KiB | 1.04 |
| chain of 4,000 | 48,712 KiB | 51,004 KiB | 1.05 |
| chain of 8,000 | 92,904 KiB | 96,992 KiB | 1.04 |
| independent 1,000 | 13,128 KiB | 13,772 KiB | 1.05 |
| independent 5,000 | 47,608 KiB | 50,208 KiB | 1.05 |

The indexes cost about 4% more peak memory, growing linearly.

A `perf record --call-graph dwarf` profile of B's `qsl-bench-probe check chain
8000` puts 69% of samples, inclusive, in node keying (`node_key::node_key`: the
canonical JSON preimage and its SHA-256), a fixed cost per node. The lookup
sites are gone from the profile.

### Enum member count (review follow-up)

Review found one more per-function cost the benches above cannot see,
because they declare no enums. Each `Typer` (one per function body, measure
and standalone expression) and each evaluator `Machine` rebuilt the package's
enum-member index, and each checked enum equality copied its enum's member
table. With M enum members, that is O(M) per function. The fix builds the
index and each enum's table once, in `Scope`, and shares them.

`checker/enum_members/<m>` (new, `benches/checker.rs`) checks 1,000 functions
over one ordered enum of *m* members. Each function is `fI(x) = E::c{I mod m}
== E::c0`, so it resolves an enum member by name and checks an enum
equality. It was measured in one interleaved A/B session on 2026-09-24, 02:57
to 03:06 (UTC-7), 5 rounds each, with only this group per round:

- A is `5c93a3a1` (origin/main) with the new bench and its generator applied
  and not committed. B is `149a4055`. The apparatus is byte-identical on both
  sides.
- Load average ranged from 5.2 to 12.9.
- Each figure is a criterion point estimate of one whole-package check. Per
  function is that figure divided by 1,000.

| Benchmark | A, 5 rounds | B, 5 rounds | A median | B median | Gain | Session MAD/median | Per function, A / B |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `checker/enum_members/10` | 103, 107, 99.1, 100, 96.8 ms | 73.9, 86.4, 71.8, 73.9, 72.7 ms | 100 ms | 73.9 ms | 26% | 3% | 100 / 74 µs |
| `checker/enum_members/100` | 137, 138, 133, 139, 138 ms | 91.3, 83.8, 76.0, 76.5, 95.7 ms | 138 ms | 83.8 ms | 39% | 9% | 138 / 84 µs |
| `checker/enum_members/1000` | 639, 632, 741, 603, 638 ms | 93.3, 101, 99.0, 116, 108 ms | 638 ms | 101 ms | 84% | 7% | 638 / 101 µs |

From 10 to 1,000 members, A's cost per function grows 6.4 times and B's
1.37 times. What is left is inherent to the type: `ValueType::Enum` owns its
`EnumShape`, a vector of every variant, so each resolved member reference
builds an O(M) value type.

The session is recorded as four collections under
`spec/evidence/measurements/`, each holding every round and the load averages:

- **Criterion rows (MP-002):** `qsl205-ab-a-checker-v2.json` and
  `qsl205-ab-b-checker-v2.json`.
- **Probe rows (MP-006):** `qsl205-ab-a-probe-v1.json` and
  `qsl205-ab-b-probe-v1.json`. Wall time and peak RSS are separate
  `quantity` dimensions.

## QSL-202: one `ModelIndex` per package. Improvement on conformance, admission and `allInstances`; normalize unchanged.

QSL-202 replaced the model layer's per-call maps with one `ModelIndex`
(`qsl-semantics/src/model/index.rs`), built once per package:

- Normalization builds it and keeps it in the `EffectiveView`. Dispatch
  linking and population admission read it from the view, and every
  `PopulationBinding` shares it and the view's type catalog by `Arc`.
- Conformance is read from each type's ancestry, computed once per type and
  kept. Before, every conformance decision walked the ancestors again.
- The reverse type catalog is built once per normalization. Before,
  `value::model_query` rebuilt it on every query, and admission copied the
  forward catalog into every binding.
- The conformance checks take the index. Before, each check built its own.

It was measured by the claim rule above, in one session on 2026-09-24, 02:47
to 03:47 (UTC-7):

- A is `5c93a3a1` (origin/main) and B is `9d5ccdfd` (the fix). Each side has
  its own worktree and target directory.
- The rounds ran A1, B1, A2, B2 … A5, B5, one `make bench-model` each.
- Load average (1-minute) ranged from 4.9 to 19.0. Other agents' builds and
  benchmarks ran on the machine during the session.
- **The apparatus changed for one benchmark.** The conformance checks now
  take a `ModelIndex`, so `resolve_root_field_redefinition` in `src/model.rs`
  takes the view's index, and `benches/model.rs` passes it. On A,
  `model/conformance/resolve_redefinition_target/<n>` times an index build
  over the whole package plus one check. On B it times one check against the
  shared index. The row measures the ticket's claim, that the build runs once
  per package and not once per check. It is not a like-for-like speedup of
  one check. B's index build is inside `model/normalize/<n>`. Every other
  benchmark calls the same `src/model.rs` functions on both sides.

Each figure is a criterion point estimate of one call. **Margin** is max(the
stated variance in the model table above, the session's MAD/median).

| Benchmark | A, 5 rounds | B, 5 rounds | A median | B median | Gain | Margin | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `model/intake/admit/1000` | 30.1 ms, 27.2 ms, 26.9 ms, 33.8 ms, 31.3 ms | 38.8 ms, 24.8 ms, 28.9 ms, 24.8 ms, 27.2 ms | 30.1 ms | 27.2 ms | 10% | 9% | improvement |
| `model/intake/admit/250` | 5.82 ms, 6.91 ms, 5.03 ms, 5.24 ms, 7.6 ms | 4.89 ms, 5.11 ms, 5.9 ms, 6.54 ms, 8.04 ms | 5.82 ms | 5.9 ms | -1% | 14% | no change |
| `model/intake/admit/4000` | 117 ms, 93.4 ms, 111 ms, 110 ms, 113 ms | 116 ms, 121 ms, 125 ms, 121 ms, 103 ms | 111 ms | 121 ms | -9% | 6% | regression |
| `model/intake/parse/1000` | 30.9 ms, 32.2 ms, 31.4 ms, 29.2 ms, 23.7 ms | 21.1 ms, 25.8 ms, 29 ms, 30.3 ms, 26.2 ms | 30.9 ms | 26.2 ms | 15% | 12% | improvement |
| `model/intake/parse/250` | 5.57 ms, 4.47 ms, 4.66 ms, 6.29 ms, 7.29 ms | 6.63 ms, 7.45 ms, 10.3 ms, 10.4 ms, 8.06 ms | 5.57 ms | 8.06 ms | -45% | 18% | regression |
| `model/intake/parse/4000` | 120 ms, 94.7 ms, 102 ms, 141 ms, 118 ms | 142 ms, 97.9 ms, 108 ms, 121 ms, 98.3 ms | 118 ms | 108 ms | 9% | 13% | no change |
| `model/intake/read_records/1000` | 79.7 ms, 60.4 ms, 70.9 ms, 83.6 ms, 87.9 ms | 89.1 ms, 59.1 ms, 63.2 ms, 78.8 ms, 61.8 ms | 79.7 ms | 63.2 ms | 21% | 10% | improvement |
| `model/intake/read_records/250` | 7.8 ms, 7.22 ms, 7.29 ms, 8.69 ms, 8.61 ms | 6.84 ms, 7.55 ms, 9.92 ms, 11 ms, 11.6 ms | 7.8 ms | 9.92 ms | -27% | 17% | regression |
| `model/intake/read_records/4000` | 3.3 s, 1.04 s, 4.98 s, 2.31 s, 2.94 s | 1.98 s, 1.44 s, 3.77 s, 3.73 s, 1.88 s | 2.94 s | 1.98 s | 33% | 51% | no change |
| `model/admit_binding/1000` | 1.34 ms, 890 µs, 2.36 ms, 1.8 ms, 1.45 ms | 180 µs, 105 µs, 104 µs, 104 µs, 191 µs | 1.45 ms | 105 µs | 93% | 25% | improvement |
| `model/admit_binding/250` | 374 µs, 398 µs, 478 µs, 521 µs, 393 µs | 93.1 µs, 93.1 µs, 148 µs, 107 µs, 125 µs | 398 µs | 107 µs | 73% | 13% | improvement |
| `model/admit_binding/4000` | 14.6 ms, 10.6 ms, 18.9 ms, 9.31 ms, 11.7 ms | 477 µs, 192 µs, 182 µs, 278 µs, 271 µs | 11.7 ms | 271 µs | 98% | 29% | improvement |
| `model/admit_invocation/1000` | 2.92 ms, 2 ms, 5.21 ms, 3.87 ms, 4.33 ms | 415 µs, 271 µs, 255 µs, 341 µs, 309 µs | 3.87 ms | 309 µs | 92% | 25% | improvement |
| `model/admit_invocation/250` | 790 µs, 704 µs, 1.17 ms, 1.2 ms, 834 µs | 220 µs, 223 µs, 292 µs, 222 µs, 278 µs | 834 µs | 223 µs | 73% | 16% | improvement |
| `model/admit_invocation/4000` | 32.2 ms, 20.3 ms, 28.6 ms, 44.5 ms, 37.8 ms | 484 µs, 400 µs, 424 µs, 418 µs, 562 µs | 32.2 ms | 424 µs | 99% | 17% | improvement |
| `model/all_instances/members/1000` | 2.25 ms, 2.06 ms, 2.93 ms, 2.97 ms, 2.99 ms | 450 µs, 406 µs, 657 µs, 771 µs, 591 µs | 2.93 ms | 591 µs | 80% | 24% | improvement |
| `model/all_instances/members/250` | 568 µs, 543 µs, 903 µs, 644 µs, 587 µs | 374 µs, 115 µs, 135 µs, 123 µs, 87 µs | 587 µs | 123 µs | 79% | 10% | improvement |
| `model/all_instances/members/4000` | 12 ms, 9.31 ms, 18.2 ms, 12 ms, 10.6 ms | 2.69 ms, 2.03 ms, 6.85 ms, 2.7 ms, 3.47 ms | 12 ms | 2.7 ms | 77% | 25% | improvement |
| `model/all_instances/own/1` | 340 µs, 311 µs, 358 µs, 462 µs, 466 µs | 385 µs, 385 µs, 364 µs, 311 µs, 440 µs | 358 µs | 385 µs | -8% | 16% | no change |
| `model/all_instances/own/120` | 570 µs, 324 µs, 462 µs, 419 µs, 1.02 ms | 330 µs, 343 µs, 457 µs, 757 µs, 433 µs | 462 µs | 433 µs | 6% | 23% | no change |
| `model/all_instances/own/32` | 447 µs, 325 µs, 458 µs, 434 µs, 429 µs | 324 µs, 331 µs, 363 µs, 661 µs, 1 ms | 434 µs | 363 µs | 16% | 11% | improvement |
| `model/all_instances/own/8` | 337 µs, 332 µs, 406 µs, 372 µs, 316 µs | 348 µs, 365 µs, 1.02 ms, 314 µs, 415 µs | 337 µs | 365 µs | -8% | 14% | no change |
| `model/all_instances/root/1` | 520 µs, 546 µs, 806 µs, 797 µs, 572 µs | 503 µs, 525 µs, 485 µs, 570 µs, 566 µs | 572 µs | 525 µs | 8% | 9% | no change |
| `model/all_instances/root/120` | 46.6 ms, 33.3 ms, 39.9 ms, 34.8 ms, 37.2 ms | 599 µs, 402 µs, 491 µs, 1.23 ms, 598 µs | 37.2 ms | 598 µs | 98% | 18% | improvement |
| `model/all_instances/root/32` | 11.3 ms, 8.15 ms, 17.3 ms, 11.3 ms, 10.3 ms | 412 µs, 415 µs, 637 µs, 446 µs, 425 µs | 11.3 ms | 425 µs | 96% | 9% | improvement |
| `model/all_instances/root/8` | 2.42 ms, 2.14 ms, 4.53 ms, 2.94 ms, 3.02 ms | 557 µs, 419 µs, 622 µs, 421 µs, 516 µs | 2.94 ms | 516 µs | 82% | 18% | improvement |
| `model/conformance/resolve_redefinition_target/1000` | 3.9 ms, 1.47 ms, 2.6 ms, 2.19 ms, 3.76 ms | 495 ns, 316 ns, 342 ns, 299 ns, 363 ns | 2.6 ms | 342 ns | 100% | 44% | improvement |
| `model/conformance/resolve_redefinition_target/250` | 313 µs, 291 µs, 363 µs, 558 µs, 360 µs | 360 ns, 295 ns, 373 ns, 318 ns, 414 ns | 360 µs | 360 ns | 100% | 14% | improvement |
| `model/conformance/resolve_redefinition_target/4000` | 26.5 ms, 27.8 ms, 27.1 ms, 28.6 ms, 31.9 ms | 306 ns, 358 ns, 355 ns, 306 ns, 381 ns | 27.8 ms | 355 ns | 100% | 14% | improvement |
| `model/normalize/1000` | 79.8 ms, 70.7 ms, 98.3 ms, 127 ms, 94.1 ms | 122 ms, 88.4 ms, 93.6 ms, 77.6 ms, 124 ms | 94.1 ms | 93.6 ms | 1% | 17% | no change |
| `model/normalize/250` | 18.3 ms, 14.5 ms, 16.8 ms, 18.9 ms, 17.7 ms | 14.4 ms, 14.4 ms, 22.7 ms, 17.2 ms, 19.4 ms | 17.7 ms | 17.2 ms | 3% | 25% | no change |
| `model/normalize/4000` | 553 ms, 458 ms, 756 ms, 601 ms, 568 ms | 729 ms, 383 ms, 639 ms, 365 ms, 534 ms | 568 ms | 534 ms | 6% | 28% | no change |
| `model/query/evaluate_all_instances/1000` | 2.49 ms, 2.67 ms, 4.54 ms, 3.16 ms, 3.3 ms | 505 µs, 457 µs, 818 µs, 1.17 ms, 879 µs | 3.16 ms | 818 µs | 74% | 38% | improvement |
| `model/query/evaluate_all_instances/250` | 788 µs, 629 µs, 1.01 ms, 917 µs, 691 µs | 193 µs, 143 µs, 145 µs, 110 µs, 123 µs | 788 µs | 143 µs | 82% | 16% | improvement |
| `model/query/evaluate_all_instances/4000` | 9.32 ms, 9.84 ms, 15.9 ms, 14.7 ms, 9.48 ms | 2.01 ms, 2.22 ms, 4.84 ms, 3.46 ms, 4.61 ms | 9.84 ms | 3.46 ms | 65% | 36% | improvement |

Reading the table:

- **Conformance.** One check costs about 350 ns at every package size, where
  it cost 360 µs to 27.8 ms with the per-check index build.
- **`allInstances` over the ancestor axis.** At 120 ancestors,
  `allInstances<C0>` over 1,000 members drops from 37.2 ms to 598 µs. The
  root query now costs about what the `own` query costs at every depth, since
  each member type's ancestry is computed once. The `own` rows, which walk
  nothing on either side, show no change except `own/32`, whose margin is
  small.
- **Admission.** `admit_binding` and `admit_invocation` no longer copy the
  view's type catalog or rebuild the supertype map per binding, so they no
  longer scale with the package: 271 µs and 424 µs at 4,000 types.
- **Normalize.** No change at any size. The index build replaces the
  normalizer's own index, and phase 4's per-type record scan is cheap on this
  input, which declares no redefinition.
- **Intake.** Three intake rows read as regressions and three as
  improvements under the rule.
  `PackageDocument::parse` is byte-identical source on both sides, and
  `read_records` differs only in no longer keeping the unread
  `has_own_precondition` flag. Those rows are session noise at this load, not
  a QSL-202 effect.

Output is unchanged: a dump of the effective-view identity, each universe's
identity, every declaration identity, the full normalization charge sequence
and counters, binding and invocation population identities, members,
`allInstances` and `lookup` at every chain level and the model-query bridge
result, over 20 generated shapes under 8 `ancestor_steps` limits (38,124
lines, including 55 refused or incomplete normalizations), is byte-identical
between A and B.

The two sides are stored as quoin measurement collections under
`spec/evidence/measurements/qsl202-ab-a-model-v2.json` and
`qsl202-ab-b-model-v2.json`.

## Engineering-assurance record

The same benchmark set is recorded as engineering-assurance MeasurementPlans,
one per axis, at definition version `-v2`:

- `spec/assurance/MP-001-parser-wall-time.md`
- `spec/assurance/MP-002-checker-wall-time.md`
- `spec/assurance/MP-003-cst-wall-time.md`
- `spec/assurance/MP-004-model-wall-time.md`
- `spec/assurance/MP-005-evaluator-wall-time.md`

QSL-203 added `spec/assurance/MP-006-probe-check-wall-time-and-rss.md`, at
`-v1`. It covers the probe's one-shot `check` runs, with wall time and peak
RSS per input.

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
- **The query bridge's catalog lookup in the type count.** It is measured at
  1,000 types only. Since QSL-202 it is a lookup in a catalog built once per
  normalization, not a per-query rebuild.
