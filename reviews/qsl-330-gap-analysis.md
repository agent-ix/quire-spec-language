---
id: SR-817
title: "QSL-330 gap analysis of PR 535 (typed OperationName resolver, ADR-017 TK-1)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@62a311d3a84b4264764964283846aac1ba2c7ddb; qsl-replay/src/spine/clause.rs; qsl-replay/src/execute/frame.rs; qsl-replay/src/spine/clause/tests/frame.rs; qsl-replay/src/spine/clause/tests/frame_replay.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/state_clause.rs; spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md (unchanged); spec/functional/FR-116-replay-a-frame-counterexample.md (unchanged); spec/test-cases/TC-514-run-an-operation-frame-over-an-invocation.md (unchanged); spec/test-cases/TC-515-replay-a-frame-counterexample.md (unchanged); spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md (unchanged; PF-3, G-1, TK-1)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
## Summary

Ticket: QSL-330. PR: quire-spec-language#535 at 62a311d3. There is no plan
bundle for this ticket. The work item is ADR-017 §6 TK-1, so this check
compares TK-1's fix and exit criteria, FR-115/FR-116 and TC-514/TC-515 with
the tests and code. This includes the semantic check the brief asked for.

TK-1 fix items, against the code:

- `OperationName` is (model `Identifier`, object `Identifier`, operation
  `Identifier`). Done (clause.rs:93-101).
- There is one resolver in `check` to (`DomainPackageRef`, object
  `DeclarationKey`, operation `Identifier`), `None` when the alias or type does
  not resolve. Done (mod.rs:1818-1836, state_clause.rs:71-75).
- `operation_frame` takes the resolver's output. Done (mod.rs:1852-1874).
- `resolve_frame`, both entry paths, `execute/frame.rs::operation_name`, and
  the `frame_replay.rs` identities helper are updated. Done. No string path is
  left beside the new one.

Exit criteria:

- An inherited operation selects its declaring frame through the resolver.
  Met, by
  `an_inherited_operation_selects_its_declaring_frame_through_the_resolver`.
  Mutation M5 (frame only when `declaring == context`) turns it red.
- A formatted string no longer type-checks as a selection. Met by the type
  change. The E0451 doctest guards the invariant: mutation M1 (fields `pub`)
  turns it red. The E0061 doctest is weaker (SR-816 FND-002).
- TC-514 and TC-515 pass unchanged. Met. The existing tests changed only how
  they build `OperationName` (`identifier(..)` in place of `to_owned()`), and
  no assertion changed. Reviewer focused run: 26 passed, exit 0.

Traceability. `quire coverage` was not rerun: the PR adds no matrix rows, and
the two new tests reuse an existing tag. Both new tests carry
`#[trace("TC-514", "FR-115-AC-4")]`. FND-001 covers how honest that tag is.

ADR-017 G-1 still shows the old `operation_frame(context: &str, operation:
&str)` signature. That is not a gap. G-1 records the defect as found, with
file:line citations at the ADR's base, and TK-1's row states the fix. This PR
does not need to change it.

## Verdict

CONDITIONAL. Every TK-1 item and exit criterion is met. Neither FR-115 nor
TC-514 states the TK-1 behaviour (select via an inherited operation; refuse an
unbound alias or undeclared type), so both new tests trace to an AC whose text
they do not verify.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `an_inherited_operation_selects_its_declaring_frame_through_the_resolver` is tagged FR-115-AC-4, but it verifies a positive selection: an inherited operation reaches its declaring type's frame, with the subtype as context. AC-4 (and TC-514 step 4) names only `Config::ConfigVersion::missing`, an operation no clause or attempt names, and an absent pre snapshot. No FR-115 AC states the TK-1 exit criterion, so a coverage report shows AC-4 as backed by a test of other behaviour. Fix, spec-only in this PR: add an FR-115 AC (for example FR-115-AC-6) and a TC-514 step saying that `Frame` on `Config::Sub::attemptUpdate` over the Sub fixture selects `ConfigVersion`'s frame with `Sub` as context. Then retag the test. | qsl-replay/src/spine/clause/tests/frame.rs:571-617; spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md:107; spec/test-cases/TC-514-run-an-operation-frame-over-an-invocation.md:32-34 |
| FND-002 | low | `an_unresolved_model_alias_or_object_type_refuses_at_select` is tagged FR-115-AC-4, but AC-4 names neither an unbound alias (`Nope::ConfigVersion::attemptUpdate`) nor an undeclared type (`Config::Missing::attemptUpdate`). Only FR-115 Behavior bullet 1 ("names no operation") covers them, by reading, and TK-1 names them explicitly. Fix: extend FR-115-AC-4 and TC-514 step 4 to list both cases, `select`, `missing_declaration`/`missing-name`. | qsl-replay/src/spine/clause/tests/frame.rs:534-569; spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md:107 |

## Coverage

- TK-1 fix items: 4 of 4 implemented.
- TK-1 exit criteria: 3 of 3 met in code. The inherited-selection and
  alias/type refusal tests exist but trace to an AC that does not state them
  (FND-001, FND-002).
- TC-514 tagged tests at head: 12 (10 existing, 2 new). TC-515 tagged tests:
  14, unchanged except for how they build `OperationName`.
- Semantic review: run inline for FR-115-AC-4 and the TK-1 criteria, with
  mutation checks M1, M3 and M5 (red) and M2 (survives; SR-816 FND-001).
