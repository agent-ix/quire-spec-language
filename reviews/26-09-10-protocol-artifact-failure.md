---
id: SR-330
title: "Failure-domain review of the compiled protocol artifact contract"
type: SpecReview
analysis: failure-domain
scope: "FR-042; TC-121; docs/compiled-protocol-v1.md; src/protocol_artifact/"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Failure-domain analysis of the closed wire/input boundary at 51506ed, covering
trust boundaries, entity identity, evaluation purity and topological robustness.
Purity and topology are the strong axes: the reader is a pure function of
(bytes, selections, limits), every recursive structure is checked by an iterative
tri-color traversal with charged depth, and the one deliberately cyclic edge kind
is excluded before that check. The gaps are in identity uniqueness and in
obligations the contract states but assigns to a "family admission" stage that
exists in no artifact or interface yet.

## Verdict

**CONDITIONAL** — no high finding; three medium unstated or unowned constraints.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Choice non-overlap/exhaustiveness, decision visibility and repeat progress are declared "family-admission obligations" with no owning requirement, interface or artifact; a package with overlapping guards and a progress-free repeat body is admitted | docs/compiled-protocol-v1.md:307; src/protocol_artifact/validate/control.rs:577 | missing-requirement |
| FND-002 | medium | Compensation identity is not unique: nothing forbids two compensations naming one `forward_effect`, although the contract says registration happens "once per effect identity"; a non-null `commit` likewise does not forbid the later registration/recovery the contract prohibits | docs/compiled-protocol-v1.md:365; src/protocol_artifact/validate/control.rs:638 | correct-requirement-no-evidence |
| FND-003 | medium | Await timeout "requires the selected progress/completeness authority", but `Await` carries only `profile`, `clock` and `within`; there is no static field that authority could occupy, so absence is unrepresentable rather than refusable | docs/compiled-protocol-v1.md:318; src/protocol_artifact/wire.rs:478 | wrong-requirement |
| FND-004 | low | `BindingRequirement.authority` is accepted when any dependency shares its reference key; no rule ties the dependency's `ArtifactKind` to the `BindingKind` it is claimed to authorize | src/protocol_artifact/validate.rs:655; docs/compiled-protocol-v1.md:347 | missing-requirement |
| FND-005 | low | `pub mod wire` lets a consumer deserialize a `Package` directly, skipping the census, limits and every invariant; the module header says so, but no type-level distinction separates decoded from admitted data | src/protocol_artifact/wire.rs:2; src/protocol_artifact/mod.rs:274 | correct-requirement-no-evidence |

## Extension points and trust boundaries

There is exactly one trust boundary and its policy is strict throughout: any
refusal aborts, nothing is repaired, and no partial package escapes. The offer
cannot designate its own acceptance — seal, contract, baseline, producer,
language, sources, dependency bytes and admitted models are all caller-supplied.
`AdmittedPackage`'s private fields keep the distinction, and the module header
states that admission is data integrity, not proof of native emission. The two
honest unsupported refusals (`ProducerCorrespondence`, `Export` for
relationship/population/component/endpoint and `Type::Reference`) prevent a tag
from inventing authority the native model adapter does not export.

## Entity identity

Uniqueness keys are explicit and enforced for dependencies, sources,
definitions, models, exports, declaration loci, binder names per scope, binding
names, control original nodes, value original expressions, resolved types and
causal edges. Separate binding kinds keep delivery, effect, attempt, commit,
registration and closure distinct, and role/channel/compensation handles cannot
be substituted across tables. FND-002 and FND-004 are the two identity rules the
contract states that the reader does not enforce.

## Evaluation purity and topological robustness

The reader performs no I/O, holds no shared state, uses no clock and mutates no
input; retry constructs a fresh `Work`, which the zero-limit test asserts.
Termination is guaranteed on every graph: scopes, value operands, temporal
nodes, binding prerequisites, wrapper types, dependencies, declaration
requirements, control children and the non-progress causal projection each run
through `acyclic`, an explicit stack with charged depth and per-edge charge.
Deep nesting is bounded before recursion — the decode census charges `Depth`
before descending, which is the only reason `disable_recursion_limit` is safe
here; that coupling is load-bearing and undocumented in the code path that
relies on it.
