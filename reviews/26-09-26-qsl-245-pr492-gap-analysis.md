---
id: SR-746
title: "PR 492 gap analysis (QSL-245 remainder)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@caa1520a4393c132583accda17aa9f8c01c14949; diff 2df75ab6...caa1520a; FR-096 AC-4, AC-5, AC-7, AC-15; TC-427; TC-428; qsl-eval; qsl-foundation; qsl-replay; qsl-semantics/src/check/region.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-428
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#492. Gap analysis of the new and changed tests against FR-096. The tests were mutation-checked in a detached scratch worktree at caa1520a with its own `CARGO_TARGET_DIR`. Both were deleted afterwards.

The unmutated tree passes all six region tests, the stage.rs, bounds.rs and AC-15 unit tests, and `pre_of_a_binding_with_no_pre_anchor_refuses_wrong_anchor`.

Mutation results. Every claimed proof holds.

| Mutant | Killed by |
| --- | --- |
| `stopped` CheckedInvariant check disabled (`if false && ...`) | `checked_invariant_is_an_internal_fault_at_s6a` (got `Kernel(Refused(CheckedInvariant))`) |
| `refusal_region` branch order swapped (location first) | `a_checking_limit_stop_is_located_at_its_node` (InputBytes gave the body text, not the declaration text), and also `package_checking_keeps_the_family_limit_region` |
| `refusal_region` location branch dropped for a `None` region | `a_lowering_stop_is_located_by_its_location` (got `None`) |
| spurious extra key in `LimitExceeded::catalog_fields` | `limit_exceeded_reports_stage_limit_exceeded_per_kind` |
| `"pre"`/`"post"` swapped in `select_anchor` | `pre_of_a_binding_with_no_pre_anchor_refuses_wrong_anchor` |

**Item 3 deletion.** No coverage is lost. The deleted `limit_exceeded` test checked the kind, bound, actual and region for the same four limits. The rewritten test checks the same `StageLimitCause` kind, bound and actual, plus the code and cause and the exact region text. The deleted function had no production caller. `qsl-replay/src/spine.rs:936` uses `refusal_region` directly.

**Item 4 tags.** `TC-428`/`FR-096-AC-7` on the stage.rs and bounds.rs tests is correct: the `LimitExceeded` and `BoundExceeded` key-table rows, and both now assert the whole map. The WrongAnchor assertion stays under its FR-090-AC-7/TC-388 test, which is correct. AC-7's `wrong-anchor` row is already backed in `causes.rs:226`.

## Verdict

**Approve with changes.** One medium finding: an untested locus that the Status section claims is tested. Three low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test checks the locus of `Typer`'s package-wide node-count stop (`Typer::enter`, `region: None`, located by `refusal.location`). The node-count case in the region test is the family's per-declaration `check_node_count`: actual 7 is the preimage count, and it fires before `Typer` for any single-declaration package because both use `limits.nodes()`. Yet FR-096 Status says package checking reports "`Typer`'s node count and depth ... tested for each stop", and FR-096's row for `Typer`/lowering names this locus. Add a two-declaration case whose second declaration passes the running count, and assert the node whose entry failed. | qsl-semantics/src/check/check.rs:1127-1147; qsl-semantics/src/check/region.rs:440-446; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:437-440 |
| FND-002 | low | The fault conversion silently changes `qsl_replay::replay` (FR-098). A kernel `CheckedInvariant` used to settle `ProofCategory::Refusal`, an `Ok` result that is inconclusive with `NoValue` (execute.rs:264). It is now `Err(ReplayRefusal::Fault)` through `CallFailure::Fault` (execute.rs:246). This agrees with FR-096's "SHALL become an `InternalFault`", but no test covers it and FR-098 does not mention it. `spine::run`'s output is unchanged. | qsl-replay/src/execute.rs:238-264 |
| FND-003 | low | `a_lowering_stop_is_located_by_its_location` is tagged FR-096-AC-4 and AC-5. AC-4 is input bytes, which the test does not touch. AC-5 is a declaration work charge located at the declaration's span, and the test asserts that a lowering work stop is not at the declaration span. Tag it to the requirement that states the lowering locus, once the spec states one (see the spec review). | qsl-semantics/src/check/region.rs:483 |
| FND-004 | low | FR-096-AC-15 names the code `runtime_invariant`. The test asserts the stage, category and invariant but not `fault.catalog_code()`. | qsl-eval/src/value/expression/mod.rs:543-545 |

## Dispositions

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.24.1 module=spec-artifacts-process@v0.26.0 id=SR-746 pr=quire-spec-language#492 reviewed=d5cf7b9487eec13c3f08d469cb239a582b54796d base=caa1520a4393c132583accda17aa9f8c01c14949 date=2026-09-26 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d5cf7b94 — `a_typer_stop_reaches_the_package_wide_node_count`, reviewer-mutation-proved |
| FND-002 | fixed | d5cf7b94 — `call_failure_to_replay_refusal` extracted and tested (Fault -> `ReplayRefusal::Fault`) |
| FND-003 | fixed | d5cf7b94 — lowering test retagged `TC-427`/`FR-096-AC-16` |
| FND-004 | fixed | d5cf7b94 — AC-15 test asserts `catalog_code()` == `runtime_invariant`/`established-invariant-broken` |

Verified by the reviewer, not from the coder's claims. The scratch worktree was detached at d5cf7b94 with its own `CARGO_TARGET_DIR`, and both were deleted afterwards.

- **FND-001 reaches `Typer`, not the family precheck.** The fixture is `g1` and `g2`, each `not not a` (3 nodes), under `CheckingLimits::new(4, 64)`. Each passes `check_node_count` on its own (3 <= 4). `nodes_used` is seeded with 3 from `g1` (check/mod.rs:911), so `Typer::enter` (check.rs:1142) refuses the second node of `g2`. The asserted fields rule out the family precheck: `region: None`, where the family precheck gives `Some(declaration span)`; actual `5 = g1_count+2`, where the preimage count would be 3; location `Body{g2,1} path [0]`; text `not a`.
- **Mutation.** `if *self.nodes >= self.limits.nodes` was changed to `if false && ...`. With that change `a_typer_stop_reaches_the_package_wide_node_count` FAILED ("g2's Typer walk crosses the package-wide node bound": check returned Ok) and the other 4 region tests passed. The mutation was reverted and all 5 pass.
- **FND-002.** `qsl-replay` `a_call_fault_settles_as_a_replay_fault_not_a_refusal` passes. The test covers the mapping function that `replay` now calls at execute.rs:244. FR-098's text is unchanged, and that is not blocking: FR-096-AC-15 is the owning requirement.
- **FND-004.** `qsl-eval` `checked_invariant_is_an_internal_fault_at_s6a` passes with the new `catalog_code` assert.

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    fix_sha: d5cf7b94
    verification: "reviewer mutation: check.rs:1142 `if false && *self.nodes >= self.limits.nodes` -> a_typer_stop_reaches_the_package_wide_node_count FAILED, 4 others pass; unmutated 5/5 pass"
    after_excerpt: |-
      let refusals = declarations
          .check(CheckingLimits::new(g1_count + 1, 64).unwrap())
          .expect_err("g2's Typer walk crosses the package-wide node bound");
      ...
      assert_eq!(limit.kind, CheckingLimitKind::Nodes);
      assert_eq!(limit.limit, g1_count + 1);
      assert_eq!(limit.actual, u128::from(g1_count + 2));
      assert_eq!(limit.region, None, "Typer's own node stop names no region");
      assert_eq!(refusal.location, Location { origin: Origin::Body { function: "g2".into(), index: 1 }, path: vec![0] });
      assert_eq!(&TWO_FUNCTIONS[start..end], "not a", "g2's inner not, not either declaration");
  - fnd: FND-002
    outcome: fixed
    fix_sha: d5cf7b94
    after_excerpt: |-
      .map_err(call_failure_to_replay_refusal)?;
      ...
      fn call_failure_to_replay_refusal(failure: CallFailure) -> ReplayRefusal {
          match failure {
              CallFailure::Input(refusal) => ReplayRefusal::Input(refusal),
              CallFailure::Fault(fault) => ReplayRefusal::Fault(fault),
          }
      }
  - fnd: FND-003
    outcome: fixed
    fix_sha: d5cf7b94
    after_excerpt: |-
      #[trace("TC-427", "FR-096-AC-16")]
      #[test]
      fn a_lowering_stop_is_located_by_its_location() {
  - fnd: FND-004
    outcome: fixed
    fix_sha: d5cf7b94
    after_excerpt: |-
      assert_eq!(
          fault.catalog_code(),
          qsl_foundation::diagnostic::CatalogCode::new(
              "runtime_invariant",
              "established-invariant-broken"
          ),
          "FR-096-AC-15 names the code runtime_invariant"
      );
```

+++


