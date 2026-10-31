---
id: SR-1933005
title: "Code review (Rust lane) of QSL-559 PR #1940: instance-bounded protocol liveness"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-559; AGENTS.md, qsl-analyze/src/analyze/items.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/src/model_check/liveness.rs, qsl-analyze/tests/it/protocol_roles.rs; PR #1940 at be07deb2c51d9ea332d81560ef8cabe78bd0cb02"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
---
# SR-1933005: Code review (Rust lane) of QSL-559 PR #1940

## Summary

Ticket: QSL-559. PR quire-spec-language#1940, branch peter/qsl-559-instance-liveness, head be07deb2c51d9ea332d81560ef8cabe78bd0cb02. Review covers only the five-file diff from origin/main...HEAD. The Rust review lane is folded into this artifact. The diff removes the prior unsupported refusal for on each liveness, carries the live-instance bound through SC/TSO liveness, tracks unique limited states, and adds three traced tests.

Checks: cargo fmt --all --check passed; cargo test --locked -p qsl-analyze --test it protocol_roles:: passed (26 tests); cargo deny check failed on AGPL-3.0-or-later dependencies not explicitly allowed by the existing deny.toml (advisories, bans, sources passed). GitHub checks for this head were green. No CI workflow changes. The PR body also reports the targeted liveness suite and clippy passed; those were not independently rerun here.

## Verdict

**CONDITIONAL.** The implementation preserves the intended order for memory-bound, fair-lasso, instance-bound, and horizon outcomes, and the targeted tests pass. One medium test-intent gap remains: the AC-6/AC-7 tests use a no-op touch attempt and the inverse property from the criterion examples, so they do not exercise the specified state-setting transition in those lasso witnesses.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-209-AC-6/7 liveness tests use touch and a different always eventually predicate from the acceptance scenarios, so neither lasso exercises the specified state-setting attempt. | qsl-analyze/tests/it/protocol_roles.rs:1287-1344 |

## Coverage

- Scope examined: all changed files and FR-209-AC-5, AC-6, and AC-7; no applicable AssuranceProfile was found for this liveness behavior.
- Test bindings examined: liveness_over_on_each_settles_instance_bound_after_no_fair_lasso → FR-209-AC-5; a_fair_liveness_lasso_may_pass_through_an_instance_limited_state → FR-209-AC-6; an_on_each_repeat_self_loop_is_fair_at_the_instance_bound → FR-209-AC-7. All three trace tags name the relevant criterion.
- No production stub, panic, unsafe operation, duplicated helper, or weakened CI gate was found in the diff.
- cargo deny check is an existing repository policy failure against transitive AGPL-3.0-or-later dependencies; no deny configuration changed in this PR.

### Finding detail

- **FND-001 (medium, confidence: medium).** FR-209-AC-6 and AC-7 specify a set attempt that changes the model value and claims always eventually holds(k.v = 0). The new tests instead construct touch("T"), leave the value at zero, and claim always eventually holds(k.v = 1). They do assert the instance-limited state and fair lasso shape, and world.settle replays the reported trace, but the tagged tests do not cover the acceptance scenario's state mutation inside those lassos. A regression in the lasso's treatment of a value-changing attempt could therefore pass these tests. Use the specified setter and claim (or add those cases alongside the generic touch cases).

## Dispositions

Disposition pass 1 at head 4c1d5a9a06829d244670be9a441c2b27237f817c.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit 4c1d5a9a0 replaces touch with the state-setting set attempt and uses the specified always-eventually claim in both AC-6 and AC-7 tests. Both tests still assert the lasso outcome and settle through world.settle, which replays the counterexample. |

FND-001 after_excerpt:

```text
attempt("T", "set", "SetPost", "t", "true")
on origin { always eventually holds(k.v = 0) }
```
