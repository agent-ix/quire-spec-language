---
id: SR-1957024
title: PR 1946 residual LT-16 helper code-review
type: SpecReview
analysis: code-review
date: '2026-11-01'
scope: 'agent-ix/quire-spec-language; QSL-1338 PR #1946; qsl-analyze/tests/it/fill.rs
  reachable helper at 6320fe8381e6aa1db40a58d0651c20532a845888'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-223
  type: references
- target: ix://agent-ix/quire-spec-language/FR-101
  type: references
---

## Summary

Ticket: QSL-1338. Static review of the residual helper optimization.

Frozen head: 6320fe8381e6aa1db40a58d0651c20532a845888. Actual diff against origin/main is only qsl-analyze/tests/it/fill.rs (+2/-1). No source/test/spec/branch edits were made by the reviewer. No applicable AssuranceProfile was found (the sole textual search match is a prior review saying none applies).

The two key APIs call the same encoder on self.key(state), with the same identity limits and error mapping. Previously state_key cloned the encoded bytes before the known-state lookup; now the same complete bytes are copied only after lookup misses. StateKey derives Eq and Hash over Vec<u8>; HashMap still compares full bytes, so hash collisions cannot merge unequal states. Encoding remains once per admission attempt. New states retain identical Reached.key bytes. The FIFO queue, successor sorting, edge appending, initial states and complete traversal remain unchanged. No limits, caps, skips, ignores, candidate reductions, assertions, fixtures, CI or Cargo files change.

The LT-16 test still asserts 7,607 states and 19,574 edges and rejects registers [2,3,3,0] with final X=1. The fixture explicitly remaps the distinct last message value 4 to 0 in its bounded Int[0,3] domain. This residual change neither changes that mapping nor the expected outcome. LT-16 is a direct reachable-graph regression, not a newly independent RC11 comparison. The existing rc11_support checker separately enumerates read-from and modification-order candidates without importing qsl-eval; it and compare() are unchanged. Reusing the authoritative canonical encoder for enumeration identity does not replace the memory-order oracle or its assertions.

Focused clippy evidence inspected: /home/peter/dev/.agents/tmp/pr1946-checks/clippy-resume.log records cargo clippy --locked -j1 -p qsl-analyze --test it --all-features -- -D warnings, Finished and Exit status: 0 / exit=0. The dispatch reports fmt passed and its fmt.log is empty; this reviewer did not independently rerun it. Debug no-run build remains pending in the coder's existing lane; no duplicate Cargo work was started. Current-head LT-16 correctness/timing, focused helper callers, final dependency-set release timing after PR #1930, and the lead-owned final aggregate gate remain pending. No residual speedup is measured. Historical combined-branch debug/release timings cannot establish benefit from these three lines. PR body now accurately distinguishes residual helper scope and pending evidence.

## Scope

```yaml
scope:
- id: reachable::admit
  path: qsl-analyze/tests/it/fill.rs
  role: examined
  excerpt: 'let key = SubjectSystem::canonical_state_key(system, &state).expect("a
    key");

    if let Some(index) = known.get(&key) { return *index; }

    let index = reached.len();

    let bytes = key.as_bytes().to_vec();'
- id: ProtocolSystem::state_key/canonical_state_key
  path: qsl-eval/src/simulation/protocol.rs
  role: context_only
  excerpt: key::state_key(&self.key(state), identity_limits()) / key::canonical_state_key(&self.key(state),
    identity_limits())
- id: state_key/canonical_state_key
  path: qsl-eval/src/simulation/key.rs
  role: examined
  excerpt: Ok((bytes.clone(), StateKey::from_canonical_bytes(bytes))) / quire_canonical::to_vec(value,
    Limits::new(identity.input_bytes)).map(StateKey)
- id: lt16_collected_sc_predecessors_still_constrain_the_sc_load
  path: qsl-analyze/tests/it/rc11_oracle.rs
  role: examined
  excerpt: '#[trace("TC-668", "FR-223-AC-8")]

    assert_eq!(reached.len(), 7_607, "LT-16 reachable state count");

    assert_eq!(transitions, 19_574, "LT-16 successor edge count");'
- id: FR-223-AC-8
  path: spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md
  role: examined
  excerpt: "ADR-025 \xA715's LT-16 does not reach a = 2, b = 3, c = 3, d = 4 and final\
    \ `X = 1` under `ra`: `w`'s view covers `L := 2` through the view of `L := 3`\
    \ and the acquire fence, though the message `d` reads (`L := 4`) does not; when\
    \ `L := 2` has been collected, `w`'s view holds its predecessors, `w`'s `seq_cst`\
    \ load takes them, and the slot for `w`'s `X := 2` before `X := 1` in modification\
    \ order is not offered."
- id: FR-101-AC-2
  path: spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md
  role: context_only
  excerpt: The state key is the JCS encoding of FR-181's typed canonical form, and
    exploration coalesces two states exactly when their full key bytes are equal.
    The state whose `semantic` member is `float64` bits `0000000000000000` and every
    map empty has its JCS key bytes as its key; with bits `8000000000000000` (negative
    zero) it has its own, different, key bytes, and the two remain two states. Two
    NaNs with different payloads are two states. Each key is a `StateKey` under `quire.simulation.state-key/v1`.
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

CONDITIONAL: static review clean, no introduced defects. Merge readiness remains conditional on the pending correctness/timing evidence and lead-owned final gate. This is not full QSL-1338 completion or a measured residual performance claim.

## Coverage

Code-review ran with rust-review and rust-style baseline folded into this one artifact. Implementation, testing, safety, duplication, integrity and code/test agreement were inspected. No new public API, unsafe code, concurrency, wire conversion, vendoring or independent duplicate implementation is introduced. Spec-review and sub-analyses are inapplicable because no spec or plan changed. No aggregate gate or heavy check ran in this review.

## Evidence disposition round 1

This round supersedes the pending-evidence statements above; the original findings table remains unchanged. Frozen candidate: `ead667652fcb05e15627fb1430acfe7d92ceddb8`. Stacked base: `71fe04996288005be1781641741c26ec5b925226` (PR #1947, incorporating PR #1930 final `87788cfc2b665ec115b34f9735b986261ee4ac46`). The correct residual comparison is this stacked base against HEAD: only the original `fill.rs` +2/-1 and two custody review files. The broader origin/main comparison includes inherited work and is not this residual PR's review scope. No branch edits, builds, tests or aggregate gate were launched by this reviewer.

Verified the original reviewed `fill.rs`, `rc11_oracle.rs`, `simulation/key.rs`, and `simulation/protocol.rs` are byte-unchanged from `6320fe8381e6aa1db40a58d0651c20532a845888` to this candidate. The full canonical identity proof, traversal and oracle/assertion assessment remain valid. The new dependency pins are inherited unchanged from the stacked base: `quire-exact` `82e8f9b4d9583ba4eee505550ff80a211333696f`, `quire-contract-model` `21fd5e98ad8962d4ef8a82b7a79dabb83c0f5b21`; Cargo.lock has no residual diff. There are no introduced source findings, and no actual FND entries to disposition (FND-001 remains solely the schema placeholder).

Inspected `/home/peter/dev/.agents/tmp/pr1946-checks/lt16-release-integrated.log`: release no-run build exits 0 after 29m43.56s, separately from execution. The exact LT-16 invocation (`cargo test --locked --release -j1 -p qsl-analyze --test it rc11_oracle::lt16_collected_sc_predecessors_still_constrain_the_sc_load -- --exact --test-threads=1 --nocapture`) passes 1 test, 0 failed, 0 ignored, in 126.46s test time / 126.72s command wall time, peak RSS 1,936,044 KiB, exit 0. Thus the unchanged 7,607-state, 19,574-edge and forbidden-outcome assertions execute and pass with the integrated dependency set. This is focused stacked-candidate evidence, not aggregate or final-main proof.

Inspected prior debug evidence at the earlier dependency set: LT-16 passes in 402.05s test time / 402.45s command wall time, exit 0 (`lt16-debug.log`). Three focused helper callers pass with exit 0 (`callers-debug.log`): `protocol_system::fill_reaches_exactly_the_seven_states_of_its_table` (0.06s), `protocol_footprints::attempts_on_one_cell_are_dependent_and_on_two_cells_independent` (0.13s), and `weak_memory::a_send_waits_for_its_threads_buffer` (0.20s). These historical debug/caller results are not reruns on the integrated candidate. The previous clippy/fmt evidence also retains its original head scope.

Current method verdict: **PASS for the scoped source review and focused integrated LT-16 evidence; no introduced defects. Overall merge readiness remains CONDITIONAL on final main integration and the lead-owned aggregate gate.** Missing final-main/aggregate proof is an evidence condition, not a source defect. No new requirement or spec review is needed for this equivalent private helper change. No isolated before/after comparison establishes a speedup attributable to the residual three-line helper patch, and this does not complete all QSL-1338 work. The scoped manual matrix fallback and plan-completion limits recorded above remain in force.

## Evidence disposition round 2

Latest frozen candidate: `99172c531e0e988daa09d8cce6b5feef3d0da38a`, normal merge of child PR #1947 head `c19d214682d414ee481bfdea8f982ab4a71bf39b`, containing main `353dcea47`. This supersedes earlier integration/timing conditions for the narrowly reviewed candidate only. Compared with the child base, the residual remains the original `qsl-analyze/tests/it/fill.rs` +2/-1 and its two review artifacts. Cargo.lock has no residual diff. Verified the helper, LT-16 fixture/assertions, and both key API implementations are byte-identical to the previously reviewed `ead667652fcb05e15627fb1430acfe7d92ceddb8`. Full canonical identity, discovery order, successor edges and forbidden-outcome checks remain unchanged. No introduced source defects or actual findings require disposition; FND-001 remains a schema placeholder.

Inspected `/home/peter/dev/.agents/tmp/pr1946-checks/lt16-release-final.log`. Runner request and acquisition both occurred at 04:10:03Z under a 20 GiB cap. Although the runner header advertises larger default parallelism, the actual Cargo commands explicitly use `-j1`, and execution explicitly uses `--test-threads=1`. Release rebuild (`cargo test --locked --release -j1 -p qsl-analyze --test it --no-run`) exits 0 after 19m13.48s, peak RSS 1,938,624 KiB, separately from test time. The exact LT-16 release test exits 0 with 1 passed, 0 failed, 0 ignored: 143.41s test time / 143.69s command wall time, peak RSS 1,936,872 KiB. The 7,607-state / 19,574-edge and forbidden-outcome assertions are unchanged and execute successfully. This is focused evidence at the final stacked source candidate, not full CI or aggregate validation.

The earlier 126.46s release result and 402.05s debug result remain historical results at their own heads/dependency sets; debug and caller tests were not repeated for this round. No isolated before/after comparison establishes a speedup from the residual helper patch. Formatting/source-diff checks are reported passed by the lead; this reviewer did not rerun formatter, Cargo, an aggregate gate or a whole-main review. No source fixes, branch changes, pushes or merges were made by this reviewer.

Latest method verdict: **PASS for this PR's unchanged source and focused final integrated LT-16 evidence; no introduced defects.** Aggregate verification, any required baseline decisions, and landing/administrative authorization remain separate lead/user responsibilities. Per dispatch, child PR #1947 source is approved but user administrative approval is pending; this disposition does not authorize bypassing it or assert that the child is merged. No full-CI-green, final landed-main, measured residual speedup or complete QSL-1338 claim is made. The original scoped manual-mapping limitation and Plan completion: not assessed remain unchanged.

## Evidence disposition round 3

Latest candidate: `46b6d4a4baa093a80302b4ae820958d73fcbfffe`, incorporating repaired main `4bf9292c88e14a26d1f61b116a46d815fdd92355`. Verified the only residual over that exact main is the original `fill.rs` +2/-1 and these two SR files; helper, LT-16 fixture/assertions and key APIs are byte-identical to round 2. The lead identifies typed-read #1975, no_std #1979, Debug #1980 and math #1972 as normally inherited repairs; this review assesses the residual and its execution evidence, not those inherited changes. No introduced source findings, no open actual findings, and no source fix is required; the findings placeholder remains excluded from dispositions.

Inspected `pr1946-checks/lt16-release-repairedmain.log`: lane 3 request/acquisition 05:28:51Z, 20 GiB cap, actual Cargo `-j1` and test `--test-threads=1`. Release no-run rebuild exits 0 in 16m54.96s, peak RSS 1,942,580 KiB, separately from execution. Exact release LT-16 passes 1 test with 0 failed/ignored, 125.98s test / 126.22s command, peak RSS 1,936,256 KiB, exit 0. Its unchanged 7,607-state, 19,574-edge and forbidden-outcome assertions execute successfully with repaired main compiled into the current candidate. Earlier 143.41s/126.46s release and debug measurements remain historical at their own heads; no isolated residual speedup is established.

**PASS for both scoped source review and current repaired-main execution evidence.** This clears the focused release evidence condition; it does not grant landing/administrative authorization, establish a full aggregate/CI result, or complete QSL-1338. Aggregate and any baseline/authorization decisions remain separate lead/user responsibilities. No builds, tests, whole-main review, branch edits or aggregate gates were launched by this reviewer. Fmt/diff checks are reported passed by the lead. The scoped manual criterion-mapping limitation and Plan completion: not assessed remain unchanged.
