---
id: TC-868
title: "A seeded profile-layering regression fails the gate naming the case and its edge"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: verifies
---
# TC-868: A seeded profile-layering regression fails the gate naming the case and its edge

## Description

Verify `xtask refinement layering` end to end over layering corpora with
entries on all six QSpec AD-003 `requires` edges E1 to E6 (QSpec FR-453).
Each `case` and `distinguishing` entry is two units, byte-equal except for
the identity string of one header `profile` declaration, and both are
compiled by the running build.

Scope: FR-345-AC-4 to FR-345-AC-11 and FR-345-AC-13.

## Test Procedure

Compile outcomes are injected only through FR-345's compile seam, and witness
dispositions only through its test negotiation function; both live in the
test module. The test passes the gate's comparison core a wrapper around the
FR-278 composition that returns a chosen result for a named unit.
Steps 1 to 6 pass a test registry holding one backend that advertises
`operation-contract` and `value-validity` in `bounded` mode, and a test
negotiation function, defined in the test module, that settles every
request `supported`, unless a step names another. No step calls
quire-contract-codegen's `negotiate_*`.

1. Build a test-only corpus of `entry.json` entries. For each of E1 to E6
   it holds:
   - a `case` both layers refuse with a typed refusal (a bounded-integer
     range proof that fails);
   - a `case` using the edge's distinguishing form, which the parent
     prohibits and the child admits;
   - a `distinguishing` entry for the edge;
   and for each of the six layers a `witness` entry: a bounded-scalar
   monitor invariant (state core), a named-predicate query over a bounded
   log (state queries), a reachability invariant over a finite universe
   (state graph), a record validation function (complete value), a
   dispatched operation contract (complete model), and a timed state model
   whose only profile is `quire.temporal.complete/v1`, with a clock field, a
   reset, a guard, a time invariant and one `invariant` clause (complete
   temporal; model selection follows through `requires`).
2. Run `xtask refinement layering` over it with the test negotiation function
   and then without a negotiation function.
3. Run the gate core over step 1's corpus with a wrapper that, for each
   edge's both-refuse case, returns for the child unit the `Ok` compile of
   the same unit with the failing range proof removed.
4. Run the gate over a corpus holding one `case` whose units differ in one
   byte of a declaration body, then over one naming state core as parent
   and complete model as child.
5. Run the gate core with a wrapper that returns a refusal for state core's
   witness, then with one that returns `Ok` for E1's distinguishing unit on
   the parent side. Then run step 1's corpus with a test registry whose one
   backend advertises only `value-validity`. Then run it with a test
   registry of two backends advertising both kinds in `bounded` mode, one
   whose negotiation arm settles every witness item `supported` and one that
   settles every item `unsupported`; then with both settling `unsupported`.
   Then run step 1's corpus with the `value-validity`-only test registry
   and no negotiation function. Finally run it with the original registry
   and no negotiation function, leaving all witness candidates present.
6. Run the gate over an empty corpus; over step 1's corpus without the state
   graph `witness` and the E5 `distinguishing` entry; over step 1's corpus
   with one `entry.json` holding an extra member `expected`.
7. Run step 1's corpus without the E6 `distinguishing` entry and without the temporal `witness` entry.
8. Run `make refinement-layering`, which generates the real corpus into a
   temporary directory and runs `refinement layering-check` over it.
   Capture the child CLI exit code, JSON report and Make target exit. Then run
   `refinement layering-check` over a copy of the generated corpus with one
   `witness` entry removed. Call `layering_check::check` with a stub
   executable that prints the expected report and exits 0, 1 and 10, and
   check the expected report with one layer result removed. The check
   receives the stub executable directly as a test argument; it reads no
   environment override and adds no production provider.
9. From a root-package test, call `compare` over the real corpus that
   `layering_corpus::generate` writes, with the
   FR-278 composition, QSL's registry snapshot and a test negotiation
   function that settles every request `supported`; then call it without a
   negotiation function. Capture both reports and process streams, and read
   `cargo tree -p xtask -e normal,build,dev`.

Each expected result is a literal in the test. Tag the test
`#[trace("TC-868", "FR-345-AC-n")]` over the criteria each step backs.

## Expected Results

- Step 2, negotiated run: verdict success, exit 0. On each edge the both-refuse case
  `holds` reporting both codes and the distinguishing-form case is
  `not applicable`; each layer result `holds`, every witness item settling
  `supported`; each edge result `holds`. The layer, case and edge results
  are otherwise equal to the no-negotiation run.
- Step 2, no-negotiation run: each of the six layers reports
  `unresolved (unsupported)`, reason `not-negotiated`, names its candidate
  backends and submits no request; verdict unsupported, exit 21. No layer
  reports `holds` for an unrequested witness.
- Step 3: verdict violation, exit 10; exactly six `regression`s, one per
  edge, each naming its case's two `RawSourceRef`s, its edge's two layer
  identities and the parent's codes; every other result as in step 2.
- Step 4: each run gives `tool failure` naming the entry; verdict tool
  failure, exit 30.
- Step 5: the first run gives a `regression` naming `quire.state.core/v1`;
  the second gives a `regression` naming E1; the third gives a
  `regression` for each of the five layers whose `witness_request` kind is
  `operation-contract` (state core, state queries, state graph, model and
  temporal), naming disposition `unsupported` and the empty
  candidate set, while complete value holds; the fourth gives verdict
  success, every layer holding, with each candidate requested by name and
  no request settling `ambiguous-backend`; the fifth gives a `regression`
  for every layer, each naming both backend identities with disposition
  `unsupported`; the sixth, with no negotiation function, gives a
  `regression` naming the empty candidate set for each of the five
  `operation-contract` layers, while complete value is `unresolved
  (unsupported)`, reason `not-negotiated`, and submits no request; verdict
  unsupported, exit 21. The final run, with candidates present but no
  negotiation function, gives all six layers `unresolved (unsupported)`,
  reason `not-negotiated`, makes no requests and has verdict unsupported,
  exit 21. Each run but the fourth and the two no-negotiation runs has verdict
  violation, exit 10.
- Step 6: exactly one tool-failure result naming the corpus and no other
  result; exactly two naming
  state graph / `witness` and E5 / `distinguishing`; one naming the
  `entry.json` path and the member `expected`, with every other entry still
  compiled. Each verdict tool failure, exit 30.
- Step 7: exactly two tool-failure results, one naming E6 / `distinguishing` and one naming `quire.temporal.complete/v1` / `witness`; verdict tool failure, exit 30 (FR-345-AC-11).
- Step 8, with all six witness units admitted and each layer's candidate set
  non-empty: the CLI exits 21 and reports `unsupported`; each of the six layer
  results is `unresolved (unsupported)` with reason `not-negotiated` and its
  candidate backend identities, and the Make target exits 0. With one
  required witness absent, the CLI exits 30 with a tool-failure result and
  the Make target fails. The stub CLI exits 0, 1 and 10 each make the test
  seam fail; a stub that exits 21 but omits a layer result also makes the seam
  fail.
- Step 9: the negotiated call gives verdict success, matches the case, layer
  and edge results of the no-negotiation CLI report, and requests each layer's
  witness once per candidate. The call without a negotiation function makes
  no request, returns a byte-equal report to the no-negotiation CLI report and
  reports all six witness results as `unresolved (unsupported)`, reason
  `not-negotiated`. Neither call writes to stdout or stderr, and the dependency tree contains no
  quire-contract-codegen crate.
