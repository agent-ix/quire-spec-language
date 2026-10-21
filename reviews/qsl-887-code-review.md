---
id: SR-12261
title: "Code review of quire-spec-language PR #1148 (QSL-887): protocol abstract subject and the refinement internal list"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@07c53b5365c12c2a537acfac7b603b20ecce7459; PR #1148 diff origin/main...HEAD: qsl-cst/src/{cst,grammar}.rs, qsl-forms/src/refinement.rs, qsl-semantics/src/check/{check,mod,region,refinement}.rs, qsl-semantics/src/check/refinement/internal_list.rs, qsl-replay/src/{certificate/simulation,refinement/step,refinement/subject,spine/refinement}.rs, qsl-analyze/src/model_check/refine.rs, tests and spec/functional/FR-135"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: reviews
---
# Code review of quire-spec-language PR #1148

## Summary

Ticket: QSL-887. Ticket, PR and comment text were read as data. The scope is
the PR diff only. This file includes the `rust-review` lane.

What was checked:

- **S1/S2 header.** `opt(s([::, member()]))` in the grammar and the
  position-based header match in `refinement_declaration` are correct. The
  `::` guard picks the protocol arm, and `member-name` admits reserved words,
  so reading the clause by position rather than by token kind is right. The
  FR-310-AC-5 syntax vectors are refused at the tokens QSpec FR-375-AC-11
  names.
- **Check order.** Model-subject refinements are checked in the old place
  with `protocols = &[]`, so they do not wait. Protocol-subject refinements
  are checked after every protocol clause, against views over all of
  `self.protocols`, so source order does not matter. The output is re-sorted
  by declaration index. This is sound. The defect-reporting gap is FND-001.
- **`abstract_internal`.** The spellings are built from the protocol and the
  member, never from the alias. `AbstractStepClass` derives `Ord` with
  `spelling` first, and `String` order is UTF-8 byte order. The lower-case
  alias vector shows the alias-free order wins.
- **Conflicting binding.** A repeated object and a repeated spelling are
  detected by raw text, which is the canonical spelling. The list/node-row
  conflict compares the same `(spelling, kind)` class, at the row span,
  naming the entry pointer. `effect` and `commit` nodes are accepted as
  right sides because ADR-027 ST-2 makes them `event` steps.
- **JSON-pointer loci.** Pointers are built only from array indices, so no
  RFC 6901 escaping is needed. `refusal_region` returns `None` for
  `InternalList`, so no defect gets a fake source span.
- **Support set.** `RefinementSupport::BUILD` holds both features. Every
  refinement is still refused as not emitted at S4, and replay refuses a
  protocol side, so the flip produces no wrong artifact. The diagnostics it
  gives for the declared remainder are FND-003 and FND-004.
- **Replay and analyze.** `open_systems` refuses `ProtocolAbstractSide`
  before `subject_models`, so a protocol subject is never decided as a model.
  `StepTarget::Node` in `decide_step` is a fault, not a verdict.
- **Old test removed.** The hand-built `AbstractSideForm::Protocol` test is
  replaced by a from-source FR-135-AC-5 test that is stronger.
- **Rust lane.** There is no `unwrap`, `expect`, `panic!`, `unsafe` or lossy
  `as` in non-test code. Slice patterns cannot panic. The
  `check_one_refinement!` macro exists because the shared body returns early
  and borrows several locals mutably. That is acceptable.
- **Ran at the reviewed sha.** `cargo test -p qsl-semantics --all-features
  --test it refinement`, through `locked-build.sh`. The result is in the
  Verdict.

## Verdict

No major finding. The S1/S2 header, the check order, the alias-free byte
order and the conflict detection are correct. Four medium findings: list
defects are dropped on an early return, a refused clause is still resolved
against, a dependency-package clause gets a false `missing-name` while the
build claims the side, and binder typing is missing while the build claims
the side. Five low findings. Fix all of them in this PR.

Test run at the reviewed sha: `cargo test -p qsl-semantics --all-features
--test it refinement` gave 58 passed, 0 failed (all 11
`refinement_protocol` tests included). This covers the coder's note that the
suites were not re-run after the last edits, for the refinement filter.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `refinement_internals` defects are pushed only after the protocol-subject loop. The stage gate `if !refusals.is_empty() { return Err(refusals) }` runs in between, so any refusal from a model-subject refinement, or from an earlier declaration, drops every parse defect of the value. FR-147 "Precedence" says S3 SHALL report every defect. Fix: push `internal_lists.defects` right after `ParsedLists::parse`, since they depend on no protocol. Add a test: a model-subject refinement with a missing row plus a malformed value reports both. | qsl-semantics/src/check/mod.rs:1678; qsl-semantics/src/check/mod.rs:1754; qsl-semantics/src/check/mod.rs:2077 |
| FND-002 | medium | A clause refused by its slot or control checks is still pushed to `checked_protocols`, with empty `slots` or `channels`. Its `ProtocolView.checked` is therefore `Some`, and the refinement resolves against it. A list entry `lose Spec::Twice::ch` over a clause whose channel check failed then gives a false `missing-name` beside the clause's own refusals. FR-147 says that when the clause was refused, S3 reports the clause's own refusals and gives no `CheckedRefinement`. Fix: record per clause whether its own checks added refusals, and build the view with `checked: None` for such a clause. | qsl-semantics/src/check/mod.rs:2006; qsl-semantics/src/check/mod.rs:2038-2040; qsl-semantics/src/check/refinement.rs:1076 |
| FND-003 | medium | `resolve_protocol` looks only among the unit's own clauses. With `BUILD` holding `protocol-abstract-side`, `abstract Spec::Twice` naming a clause of the abstract package refuses `missing_declaration`/`missing-name` for a clause that exists. That is FR-147's own AC fixture (`Twice` in `example/counter-spec`) and QSpec FR-375's primary case ("resolves among the protocol clauses of the abstract subject package"). The PR declares this case as remainder, but the build reports a false name error for it instead of saying it is unbuilt. Fix: when the alias is a dependency's model and no unit clause matches, refuse `unsupported_construct`/`not-yet-implemented` naming `protocol-abstract-side` with a `subject` (for example `dependency-protocol-clause`). Otherwise, resolve the dependency's checked clauses. Add a vector either way. | qsl-semantics/src/check/refinement.rs:1059-1080 |
| FND-004 | medium | `node_target` types each argument with `required = None` and never compares the count with the node's binder record. So `-> node Spec::Twice::a1(1, true, self, 7)` is admitted while the build claims `protocol-abstract-side`. FR-147 says the expressions SHALL be typed against the binder record and refuse `ill_typed`/`type-mismatch`. The rule is open upstream (STD-249), but FR-147 and FR-135's edited support-set sentence both claim it. Fix: state the gap in FR-147 as `Remaining work: STD-249` in the Behavior bullet, so the support claim is honest. | qsl-semantics/src/check/refinement.rs:1955-1980; spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:222-225 |
| FND-005 | low | A shape defect (another key, or a missing key) of an object whose `refinement` names a declared refinement is recorded with owner `None`, and the object is still inserted. A malformed object's entries then resolve: `abstract_internal` is set, extra `missing-name` defects can appear, and a model-subject object gets `malformed` at `/0` twice (parse plus the model-subject rule). Fix: read `refinement` first, attribute the shape defect to it, and do not insert a defective object. | qsl-semantics/src/check/refinement/internal_list.rs:286-292; qsl-semantics/src/check/refinement/internal_list.rs:370 |
| FND-006 | low | Under a model subject, a node row is refused at `row_span`. QSpec FR-432 says the refusal is "at the node". The test asserts only the code and the cause. Fix: refuse at `node.span`, and assert the located text in the model-subject vector. | qsl-semantics/src/check/refinement.rs:1921; qsl-semantics/tests/it/refinement_protocol.rs:321-330 |
| FND-007 | low | Replay and analyze map `SubjectRefusal::ProtocolAbstractSide` to `InternalFault("the-subjects-of-a-refinement-open")`. That is the label for a broken invariant, but a protocol subject is a legitimate checked input that is not yet decided (FR-148). Fix: give it its own refusal (not-yet-implemented `protocol-abstract-side`), or at least its own invariant label, so it does not read as `NoModels`. | qsl-replay/src/certificate/simulation.rs:385-389; qsl-analyze/src/model_check/refine.rs:167-169 |
| FND-008 | low | `PackageDeclarations::refinement_support` is a `pub` field. The doc says "only a test replaces it", and the setter is `cfg(test-support)`, but any crate can still assign the field. Fix: make the field `pub(crate)`. | qsl-semantics/src/check/check.rs:499-502 |
| FND-009 | low | `refinement_declarations.rs` widens ten helpers and the `Refused` fields to `pub(crate)`. `refinement_protocol.rs` imports only `assemble`, `counters`, `mutated`, `SPEC` and `CAS`. Fix: revert the widenings that nothing uses. | qsl-semantics/tests/it/refinement_declarations.rs:32-575 |

## Dispositions

| FND | Outcome | Disposition |
| --- | --- | --- |
| FND-001 | fixed | The defects of the value are pushed right after `ParsedLists::parse`. Test: `the_defects_of_the_value_are_reported_beside_another_refinements_refusal`. |
| FND-002 | fixed | A clause whose own checks added a refusal is built into its view with `checked: None`. Test: `a_clause_refused_by_its_controls_check_gives_no_name_error`. |
| FND-003 | documented | The S3 inputs hold the unit's protocol clauses only; a dependency's checked clauses are not an input, and a model alias names a domain package that declares none, so the missing-name is the answer for every input this build can receive. FR-147 now records the dependency clause as Remaining work. |
| FND-004 | documented | FR-147 "Step rows and observations" states `Remaining work: STD-249` and what this build checks. |
| FND-005 | fixed | A shape defect is attributed to the refinement `refinement` names, which then has no list. Test: `a_shape_defect_of_a_named_object_is_reported_once`. |
| FND-006 | fixed | Refused at `node.span`; the model-subject vector asserts the located text. |
| FND-007 | fixed | `ModelCheckRefusal::ProtocolAbstractSide` and `ClosureRefusal::ProtocolAbstractSide`, both `unsupported_construct/not-yet-implemented`. |
| FND-008 | rejected | Every field of `PackageDeclarations` is `pub` because other crates build it with `..PackageDeclarations::new(..)`, which needs every field visible (`qsl-eval` does); a private field fails to compile there. The field's doc says only a test replaces it. |
| FND-009 | fixed | The unused widenings are reverted. |
