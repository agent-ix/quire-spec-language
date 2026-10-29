# Benchmarking

This guide defines how to run benchmarks and compare a candidate with current
main. Decisions use only measurements collected live for the current A/B run;
no stored measurements or past results are comparison inputs.

## Running the benchmarks

| Axis | Command | Bench file | Input generator |
| --- | --- | --- | --- |
| Parser: nesting depth and volume | `make bench-parser` | `benches/parser.rs` | `src/parse.rs` |
| Checker: call chain and independent functions | `make bench-checker` | `benches/checker.rs` | `src/check.rs` |
| CST identity hashing against source bytes | `make bench-cst` | `benches/cst.rs` | `src/parse.rs` |
| Model layer on an N-type `DomainPackage` built through FR-154 intake | `make bench-model` | `benches/model.rs` | `src/model.rs` |
| Evaluator cost per call frame | `make bench-evaluator` | `benches/evaluator.rs` | `src/check.rs` |
| Checker: Text-reachable recursive cluster | `make bench-text_cluster` | `benches/text_cluster.rs` | `src/text_cluster.rs` |
| Counts, refusal boundaries, one-shot large inputs and peak RSS | `make bench-probe` | `src/bin/qsl-bench-probe.rs` | all of the above |

`make bench` runs all six criterion suites. Each target runs
`cargo bench --locked -p qsl-bench --bench <axis>`, and criterion options go
through `BENCH_ARGS`. Run benches with `-p qsl-bench`, never `--workspace`;
other crates' dev-dependencies can enable `quire-exact/test-support` and make
the meter log every charge.

Each input is built through the layers' public API. A change behind that API
needs no benchmark change; an API change requires updating its generator. Each
generator has a smallest-size test so invalid inputs fail in `make ci` before
benchmark time.

## Comparing a candidate with current main

1. Build current `origin/main` (A) and the candidate (B) in separate worktrees
   with separate target directories.
2. In one session, alternate A and B for five rounds each on the same machine;
   record load average before and after every round.
3. For each benchmark, calculate each side's median of its five point estimates.
4. Calculate the session noise from those ten live observations: the largest
   of A's and B's MAD/median and, for criterion runs, the widest 95%
   confidence-interval half-width divided by its estimate. Probe runs use the
   two sides' MAD/median because they do not report criterion intervals.
5. B is faster only if its median is below A's median by more than that session
   noise. A smaller difference is no change.
6. Record every round for both sides. Do not select favorable rounds or reuse
   measurements from an earlier session as a threshold.

The A/B runs are the comparison. No fixed table variance or earlier
measurement collection supplies a decision threshold.
