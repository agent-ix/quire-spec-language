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

Failure-domain recheck of the closed wire/input boundary at 23a5892 — trust
boundaries, entity identity, evaluation purity, topological robustness. Purity
and topology remain the strong axes and are unchanged. Of the three original
medium findings, one is resolved by a real static rule, one is resolved by an
ownership statement in the contract, and one is **withdrawn** as a conflation of
runtime and static identity. Three low items remain, one of them new.

## Verdict

**CONDITIONAL** — no high or medium finding; three low unenforced or overly
permissive constraints, each an explicit producer or consumer obligation.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | `BindingRequirement.authority` is still accepted whenever any dependency shares its reference key; no rule ties that dependency's `ArtifactKind` to the `BindingKind` it authorizes. The contract now scopes this deliberately — the reader retains selected bytes, provider capability is a family-admission/consumer obligation — so it is a stated boundary, not a silent gap | src/protocol_artifact/validate.rs:640-654; docs/compiled-protocol-v1.md:370-386 | missing-requirement |
| FND-002 | low | `pub mod wire` still lets a consumer deserialize a `Package` directly, skipping the census, limits and every invariant. The module header says so and `AdmittedPackage` keeps private fields, but no type-level distinction separates decoded from admitted data; `encode_candidate`'s public signature requires the module to stay public | src/protocol_artifact/wire.rs:2; src/protocol_artifact/mod.rs:18 | correct-requirement-no-evidence |
| FND-003 | low | The new self-`Origin::Selected` allowance is unconditional: any value may name itself, including a leaf `Boolean`/`Text` or a single-origin node that the solver can never mark `Selected` (it emits that marker only where two child origins differ). The reader admits a provenance marker the producer cannot emit | src/protocol_artifact/validate.rs:774-781; src/checking/composed/solver/origins.rs:126-136 | missing-requirement |

## Withdrawn

**Compensation identity uniqueness (was medium).** The original finding asserted
that "nothing forbids two compensations naming one `forward_effect`" and that a
non-null `commit` must forbid a later registration/recovery declaration. Checked
against the accepted shared requirements, both halves conflate a runtime
observation rule with a static identity rule:

- `quire-specification` FR-056 requires the evaluator to "register **exactly the
  compensation obligations** paired with that effect" — plural, per obligation.
  FR-056-AC-3's uniqueness is per replayed **effect identity at runtime**, not a
  static global `forward_effect` key. Multiple authored obligations paired with
  one effect are admitted by design.
- FR-057 governs **observed** ordering: it rejects or violates "registration after
  a forbidding commit" and "compensation after a forbidding commit". `proposals/
  quire-v1/protocol-contract.md:98-100` states the same three conditions as
  protocol violations against observations. Neither bans a static recovery
  declaration merely because a commit is selected.

The contract now says this explicitly: distinct compensation definitions may name
the same forward control, and a non-null commit "forbids the selected subsequent
runtime registration/recovery under standard FR-057. Its presence does not forbid
declaring a recovery relation." The static reader checks the commit's kind and
owner; the consumer checks ordering against bound observations. The finding is
withdrawn on that evidence.

## Resolved since the original review

**Family-admission obligations had no owner (was medium).** Choice ownership,
branch labels, non-overlap, decision visibility and repeat progress are now
assigned in the contract to compiler family admission under FR-042, consuming the
accepted standard's FR-050–059, and are stated as establishable only by the
constructor-private producer admission before emission. They remain producer
obligations owned by a requirement rather than inferred from graph shape; the
reader verifies derived data and its independently selected producer, not the
source proof. No population of D/F and no producer correspondence is fabricated.

**Await timeout authority was unrepresentable (was medium).** `Await` still carries
only `profile`, `clock` and `within`, but the authority is no longer absent — it
is a separate static association the reader now requires. `timeout_authority`
demands a `Progress` or `Closure` binding requirement whose `subject` is
`Control(this await)` and whose `requires` contains that await's clock **binding
index**, refusing a missing association, a foreign subject or an absent clock
dependency as `Invalid::Binding`. The clock index itself is already kind-checked
as `BindingKind::Clock`. The contract states the rule, and TC-121 step 5 states
the three negatives. This is a static prerequisite association only: it is not a
runtime timeout proof and not a provider capability claim, and the contract says
so.

## Extension points and trust boundaries

Unchanged. One trust boundary, strict policy throughout: any refusal aborts,
nothing is repaired, no partial package escapes. The offer cannot designate its
own acceptance — seal, contract, baseline, producer, language, sources, dependency
bytes and admitted models are all caller-supplied. The contract now names the
adapter boundary out loud: `ix:native` edition `1-draft` with registered
definitions and actual `NativeModel` views, with producer/native correspondence
and relationship/population/component/endpoint/reference exports as explicit
`Unsupported` prerequisites "until their authoritative producer adapters exist".
Those refusals keep full FR-042 emission and handoff acceptance open rather than
closing it by assertion.

## Entity identity

Uniqueness keys remain explicit and enforced for dependencies, sources,
definitions, models, exports, declaration loci, binder names per scope, binding
names, control original nodes, value original expressions, resolved types and
causal edges. Separate binding kinds keep delivery, effect, attempt, commit,
registration and closure distinct; role/channel/compensation handles cannot be
substituted across tables, and the local-table lookup now proves that structurally
— `(kind, body)` is matched exhaustively, so a control/role/channel/compensation
handle against a predicate, state or temporal body returns `Invalid::Owner`
instead of reaching an `unreachable!()`. FND-001 is the one identity rule the
contract describes and the reader deliberately does not enforce.

## Evaluation purity and topological robustness

Unchanged and still the strongest axis. The reader performs no I/O, holds no
shared state, uses no clock and mutates no input; retry constructs a fresh `Work`,
asserted by the zero-limit test. Termination holds on every graph: scopes, value
operands, temporal nodes, binding prerequisites, wrapper types, dependencies,
declaration requirements, control children and the non-progress causal projection
each run through `acyclic` with an explicit stack, charged depth and per-edge
charge. The self-origin exemption does not weaken this — operand edges are still
added, `self.local` still applies the owner and range checks, and
`selected_provenance_marker_is_distinct_from_an_evaluation_cycle` asserts that a
foreign owner refuses `Invalid::Owner`, an out-of-range index refuses
`Invalid::Reference`, and a two-node non-self origin cycle still refuses
`Invalid::Cycle`. The load-bearing coupling between the census's `Depth` charge
and `disable_recursion_limit` is now documented at the call site.
