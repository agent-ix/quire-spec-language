---
id: SR-12933
title: "Code review of quire-spec-language PR #970 round 4: refinement S1-S3 reshape and fair strong (QSL-543, QSL-874)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d22cb40341ae6df53c4e534d2b888124e6476b06; PR #970; qsl-cst/src/{cst.rs,grammar.rs}, qsl-forms/src/{refinement.rs,dispatch.rs,syntax.rs,value.rs}, qsl-semantics/src/check/{refinement.rs,refusal.rs,assemble.rs,mod.rs,check.rs,region.rs}, qsl-replay/src/spine*.rs, tests"
review_set: subset
evaluated_revision: "d22cb40341ae6df53c4e534d2b888124e6476b06"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
---
# Code review of quire-spec-language PR #970 round 4: refinement S1-S3 reshape and fair strong (QSL-543, QSL-874)

## Summary

Ticket: QSL-543 (and QSL-874 for `fair strong`). Round 4 code review with the rust-review lane folded in. It covers only `git diff origin/main...HEAD` (merge base 4978f722a) at head d22cb4034. That head is the branch after the origin/main merge and the reshape of S1-S3 onto the merged grammar and FR-310. Rounds 0-3 (SR-12814) are closed, and this file does not reopen them.

What I ran: `cargo test -p qsl-cst -p qsl-forms -p qsl-semantics --features qsl-semantics/test-support --test it refinement` (15 + 8 + 28 passed), `cargo clippy` on qsl-cst, qsl-forms, qsl-semantics and qsl-replay with `--all-targets -D warnings` (clean), and `git diff --check` (clean).

What I checked: the S1 fairness row against QSpec FR-375 on origin/main (a2230e1, the grammar now has `weak | strong` and AC-10 refuses a kindless row). The S2 RefinementForm against FR-310 Outputs: AbstractSideForm::{Model, Protocol}, StepSourceForm::{Named{kind,name}, Memory}, StepTargetForm::Node, HistoryRowForm and HistoryLiteralForm. The S3 fairness duplicate rule, the keyword or memory step source over a model subject, the node right side, the new Unsupported cause and how the spine classifies it. I also checked string edges, typed-ness, dead code, shims and the deleted source_grammar_vectors tests.

## Verdict

No high finding. Fairness semantics are right. A kindless row refuses at S1 (`assume fair Impl::T::op;` at `Impl`), as QSpec FR-375-AC-10 requires. S2 reads the kind and granularity from the row's own tokens, so an operation named `weak` or `each` cannot confuse it, because the operation name is a child node. S3 keys a fairness row by the triple of kind, resolved FairnessOperation (declaring type and name) and granularity, with an unwritten granularity read as Whole. So `fair weak op` and `fair weak whole op` conflict, a weak row and a strong row on one operation do not, and two spellings of one inherited operation conflict too. The conflict refuses at the second row, names the first row's span, and drops neither row's diagnostics. The kind is always Some in a refinement form. A None kind is an internal fault, not a silent Weak. That replaces the old `None | Some(Weak) => Weak` default, so no shim remains. A keyword or memory step source over a model subject refuses missing-name, and a `node` right side refuses malformed-declaration (FR-136, FR-147-AC-2). History rows and a protocol abstract side are refused, not ignored. The two deleted source_grammar_vectors tests asserted that a refinement had no S2 dispatch entry. That is false now that the entry exists, and refinement_forms.rs covers every row kind building, so the deletion is justified. The FormsCause::NoDispatchEntry tests for relation and hyper clauses remain. The only value.rs changes are two `pub(crate)` visibility widenings, which refinement.rs uses.

One medium finding: the refusal code for the specified-but-unimplemented rows classifies as a content refusal in the FR-341 spine (FND-001). This repo's own convention gives a different answer, and STD-199 is open for QSpec to rule. Two low findings follow.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new RefinementCause::Unsupported (a history row, or an abstract protocol side) refuses unknown_required_feature/unsupported-feature. No QSL requirement names a code for this case: FR-138 and FR-147 specify these rows as valid, and STD-199 item 4 asks QSpec to choose between this code and unimplemented_capability (QSpec FR-271 defines the latter as 'valid complete-V1 meaning is not implemented by the selected producer'). The code is defensible under QSpec FR-271/272, but it has a concrete consequence in this repo. qsl-replay classify_refusal (FR-341, qsl-replay/src/spine/refinement.rs:453-475) treats every code other than limit, unsupported_construct and unsupported_projection as a typed content refusal. So a unit whose only fault is a valid history row classifies `refused` (the source is wrong), not `unsupported` (a QSL implementation gap). The PR's own not-emitted refusals (TerminalNotEmitted, RefinementNotEmitted) use unsupported_construct/not-yet-implemented, which FR-341-AC-5 names as 'S3's refusal of a construct it has no checker for', and that code classifies `unsupported`. Fix: refuse unsupported_construct/not-yet-implemented until STD-199 rules, or keep the code and add it to FR-341's `unsupported` rule (a spec change for the owner). Either way, say which in STD-199. | qsl-semantics/src/check/refinement.rs:116-125, 147, 160, 487-498, 581-592 |
| FND-002 | low | Two trace bindings are wrong. a_history_row_is_refused_until_it_is_checked is an S3 test (qsl-semantics) of a refusal FR-138 does not specify, but it is tagged TC-889, the S2 form-building TC of FR-310. That puts an S3 assertion under FR-310 in the matrix. It should carry no TC, or the TC that owns the refusal once STD-199 names one. protocol_step_rows_refuse_over_a_model_subject cites FR-136-AC-1 in its doc for the keyword and memory left-side cases. FR-136-AC-1 is the positive CasRefinesCounter/RingIsQueue check, and the cases fall under FR-136's 'a row whose left side names no concrete operation' sentence, which has no AC of its own. The FR-147-AC-2 tag covers only the node-over-model variant of that AC; see the gap analysis FND-001. | qsl-semantics/tests/it/refinement_declarations.rs:1205-1211, 1257-1262 |
| FND-003 | low | The Protocol arm of check_refinement has no test. S2 never builds AbstractSideForm::Protocol from source (FR-310 states that the side waits on STD-182), so the arm is reachable only through a hand-built RefinementForm, and no test builds one. Nothing shows the refusal fires, or that it is the only refusal. After refusing, the arm keeps checking with the protocol's model alias as the abstract model, so a hand-built protocol form may also produce MissingRow or Unresolved noise that no test has looked at. Fix: add a unit test in refinement.rs's tests module that builds a Protocol form over CasRefinesCounter's rows and asserts the single unsupported refusal at the clause span. Alternatively, return right after the refusal. | qsl-semantics/src/check/refinement.rs:485-498 |
