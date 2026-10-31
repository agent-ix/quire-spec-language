---
id: SR-1939004
title: QSL-1364 holds-node fault code and Rust review
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1364; PR1945 frozen 04ef7ec205858543a8a76b0f2f839813e7a5dfb3; origin/main...HEAD; qsl-eval/src/value/expression/mod.rs and qsl-eval/tests/it/named_population.rs"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/TC-136, type: reviews}
  - {target: ix://agent-ix/quire-spec-language/FR-049, type: references}
---

## Summary

Ticket: QSL-1364. Independent reviewer-only review of the frozen two-file delta. The code-review method includes its Rust lane. No requirement, dependency, CI or public signature changes. Repository AGENTS.md, CLAUDE.md, CONTRIBUTING.md, Makefile and applicable rust-style references were examined. No applicable AssuranceProfile was found.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

PASS for this delta. The existing shared InternalFault constructor and with_named builder retain stage S6a and invariant holds-atom-resolved-by-clause, adding precisely the rejected formula index under node. No fault vocabulary or shared dictionary implementation is duplicated or modified. The valid path and accounting path are unchanged.

## Review Evidence

TemporalClauseNode::condition uses a checked map lookup by atom.index(), returning None for either invalid case. The new detail reads the identity newtype's usize through its existing accessor and formats it without indexing the formula or narrowing a number. Fault construction remains inside ok_or_else, before Machine construction; invalid input returns through ? before meter access. The pinned quire-semantic-value 74ab6f8 catalog defines with_named as a consuming builder replacing the detail vector, which is initially empty for InternalFault::new. No kernel cause is lost.

The regression uses the real checked source/package and admitted watcher snapshot. It verifies a non-holds root and a beyond-count handle from a larger clause, then unconditionally checks Err, the Fault variant, exact stage, invariant and single node detail, and unchanged meter for both. The valid holds control asserts Completed Boolean true. The exact detail assertion rejects empty details, a wrong key, a constant/wrong index and surplus entries. The test's expected index comes from its selected handle; the actual value comes from the production fault. Existing TC-136 trace remains a partial test-case classification, not a full FR-049-AC-1 completion claim.

Reviewer verification: cargo fmt --all -- --check passed. Independent nine-test named_population run passed (9 passed, 0 failed) in /tmp/qsl1364-reviewer-green.log. Independent cargo clippy --locked -p qsl-eval --all-targets --all-features -- -D warnings passed in /tmp/qsl1364-reviewer-clippy.log. Coder logs inspected: /tmp/qsl1364-clippy.log (locked qsl-eval all targets/all features, warnings denied), /tmp/qsl1364-compile.log (integration target compile), /tmp/qsl1364-final-green.log (9 passing tests). Coder old-production controls /tmp/qsl1364-red-non-holds.log and /tmp/qsl1364-red-out-of-range.log fail at the exact new identity assertion, showing empty details versus node 0 and node 3 respectively; reviewer inspected these controls but did not mutate production code to repeat them.

Per dispatch's explicit baseline exclusions, no aggregate gate rerun: current-main C14 MalformedWire node/node_id, string-edge and checked-input failures are outside this two-file change. No gates, assertions, lint settings or safety policy were weakened by the diff. No new unsafe, concurrency, external-service, wire decoding, vendoring, duplicate helper or stub surface exists.

## Scope

```yaml
scope:
  - id: TC-136-H3
    path: spec/test-cases/TC-136-admit-composed-state-views.md
    role: examined
    excerpt: "| H3 | `evaluate_holds` | a formula node id that is no `holds` atom of the clause, and an id beyond the clause's node count | an `Err` fault naming the node; no `Evaluation` |"
  - id: TC-136-handle-assertions
    path: spec/test-cases/TC-136-admit-composed-state-views.md
    role: examined
    excerpt: "Each row asserts the typed refusal or fault as a field and the unchanged meter, never message text."
  - id: FR-049-AC-1
    path: spec/functional/FR-049-admit-composed-evaluation-inputs.md
    role: context_only
    excerpt: "Each checked-term evaluation selects an exact checked declaration/expression identity; raw bytes, freely constructed wire records and a foreign or out-of-range checked handle cannot enter an evaluation session."
bindings:
  - {test_id: "named_population::invalid_holds_handles_fault_without_spending_the_meter", ac_id: TC-136, trace: correct}
```

The H3 and handle-assertions ids above are local scope labels within TC-136, not additional minted artifacts.
