---
id: SR-620
title: "EARS review of the QSL-148 FR-065 reconciliation"
type: SpecReview
analysis: ears-conformance
scope: "The SHALL statements the working-tree change on spec/148-fr065-reconcile adds or rewrites in FR-065: the Description, the application-check and one-S3-checker Behavior sections, CON-2 and CON-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-163
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-164
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-376
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-377
    type: reviews
---

## Summary

The rewritten statements have concrete subjects and testable responses:
`check::family::Application`, `Typer`, `ValueFunctionFamily::check` and
`PackageDeclarations::check`. They state the current design. They contain no
roadmap and no "SHALL NOT" rules about unsupported alternatives.
FR-065-CON-2 and FR-065-CON-3 are Design/Inspection constraints written as
present-tense facts, which matches how the other constraint rows are
written. The removed text had a SHALL that required a symbol to be deleted
and a SHALL that required an enum variant to be removed. Both are gone.

The problems are grammatical. Statements pack several obligations into one
sentence. Obligations are written without SHALL, so it is unclear whether
they are normative. One obligation is stated in three places.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The application-check statement puts five obligations in one sentence and uses the pronoun "it" as the subject: resolve the name, refuse the arity mismatch, refuse the missing name, supply the parameter types, and build the node. Its trigger ("For a call `name(arguments)`") is not in EARS event form. Fix: use the event form, one obligation per sentence. "When the S3 typer checks an `Expression::Call` `name(arguments)`, `check::family::Application` SHALL resolve `name` to a declared function callable by name." "When the argument count differs from that function's parameter count, `Application` SHALL refuse the call as `ill_typed`/`type-mismatch`." Write the missing-name, parameter-type and result-type obligations the same way. | FR-065:91-97 |
| FND-002 | low | Three sentences in the amended Behavior section are written in the indicative mood, with no SHALL: "It resolves a declared tuple type's constructor by the same rules", "A call therefore receives the same verdict from every entry point", and "The typer checks each argument against the expected type the application check supplies". The first and third describe behaviour a test depends on. The second is what AC-5 tests. Fix: make each one a SHALL with a named subject, or mark it as a consequence of the statements before it. For the second sentence, use the narrowed wording in SR-619 FND-001. | FR-065:97, :103-106 |
| FND-003 | low | The Description's SHALL has five obligations joined by "and": check, package and evaluate through the contract; give each form one S3 checker; carry identity and provenance; reach callers through the producer. Most of it is older text, but the amendment added two more obligations. Fix: one SHALL per obligation. Each can refer to the Behavior section that states it in full. | FR-065:23-30 |
| FND-004 | low | The rule "one S3 path per form" is stated three times: in the Description ("give each form exactly one S3 checker"), in the "Each function form has one S3 checker" SHALLs, and in FR-065-CON-2. The last two also repeat the application check's obligations from the section above. Three copies of one rule can drift apart. Fix: make CON-2 the only normative statement of the rule. In the Behavior section, keep one sentence that refers to CON-2, and drop the repeated application-check SHALL. | FR-065:26, :152-159, :199 |

## Resolution

- FND-001: the application check is one "When the S3 typer checks a call"
  trigger with one SHALL per obligation, each with `Application` as subject.
- FND-002: the tuple-constructor and argument-checking sentences are SHALLs
  with named subjects; the same-verdict sentence is stated as a consequence.
- FND-003: the Description has one SHALL per obligation, each pointing to its
  Behavior section.
- FND-004: FR-065-CON-2 is the one normative statement of "one S3 path per
  form"; the Description and Behavior refer to it.
