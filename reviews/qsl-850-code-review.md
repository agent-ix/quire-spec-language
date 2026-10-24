---
id: SR-12139
title: "QSL-850 PR #963 code review: protocol activation form, S3 obligation checks, Int-sum equality, model scalar names"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@49d2f331c508a7574315d3852f8fc1aedf06d1a7; PR #963: qsl-forms/src/protocol_clause.rs, qsl-forms/src/syntax.rs, qsl-forms/tests/it/protocol_member_forms.rs, qsl-replay/src/spine/refinement.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/protocol_clause/, qsl-semantics/src/check/protocol_obligation.rs, qsl-semantics/src/check/protocol_obligation/cause.rs, qsl-semantics/src/check/protocol_obligation/tests.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/region.rs, qsl-semantics/tests/fixtures/protocol_attempt.rs, qsl-semantics/tests/fixtures/protocol_obligations.rs, spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md"
review_set: subset
---

## Summary

Ticket: QSL-850. PR quire-spec-language#963, diff `origin/main...49d2f331c`
(22 files, +3188/-74). Code review with the rust-review lane folded in.
Static review only: no cargo, no make, per the brief. The author's gate
(check, clippy) is taken as reported.

Checked hard, per the brief:

1. `admit_sum_into_peer` (typing.rs). It fires only on the `=` peer path
   (`PeerForm::Equality`, `Pair::Second`), never on ordering. One side must
   be an `Integer`-typed `NodeKind::Arithmetic` node with no explicit
   `convert` target. The other side must be an unconverted `Int[..]`. The sum
   goes through `Typer::coerce`, which builds a `NodeKind::Coerce` into the
   interval. That is the same range-obligated node an `Int[..]` slot already
   gets, so `Definedness` must discharge it or refuse `unproved-range`. A bare
   `Integer` local, a literal, and a `convert<Integer>` operand all fall
   through to the equality matrix unchanged. The tests cover the guarded sum
   (checks), the unguarded sum (`unproved-range` at `earlier.tally + 1`), a
   bare `Integer` capture (`type-mismatch`) and the explicit convert
   (checks). The change is sound: it only turns an `ill_typed` refusal into
   a proved coercion or an `unproved-range` refusal, and it can never make an
   equality evaluate to a wrong value. It applies to every clause that goes
   through the typer, not only protocol stages. That is what the lead ruled
   (STD-157), and it is not raised here.
2. Model scalars (assemble.rs). `model_object_types` names each
   `ScalarType` record `alias::T` through `model_value_type` with a 1..1
   multiplicity. That is the same mapping a field of that type gets, so
   `M::Total` is `Int[0, 1000]`. The name goes into `model_composite_names`,
   `names.types` and `package.aliases`. `package.aliases` only feeds
   `ScopeIndex::named_type`, so a scalar becomes a nameable type with no
   declaration identity and is not emitted. A role `on M::Total` now reaches
   #956's `RoleModel` refusal, not `MissingRoleType`.
3. `Box<ProtocolDeclarationForm>`. Every match site was checked: the one
   constructor (qsl-forms/src/protocol_clause.rs:389), the one consumer that
   moves the form out (`*protocol`, assemble.rs:702), and the test `let ...
   else` sites in qsl-forms/tests/it, which deref-coerce. No other crate
   matches `DeclarationForm::Protocol`.
4. Merge with #956 (`protocol_controls`). `CheckedProtocol` keeps both
   `obligations` and `channels`. `covered_construct` admits `Capture`,
   `ActivationEach` and `Channel`. The pipeline runs `protocol_clause::check`,
   then `protocol_obligation::check` (anchors when resolved), then `content`,
   then stores the obligations, then `protocol_controls::check`. The
   `Unresolved` arm raises an internal fault only when `content` refused
   nothing, which is the right invariant. A new cancel check after each
   protocol returns early.
5. Stage tables. Every `Site` binds exactly the names FR-381's row lists. The
   `over` input is bound in every stage, and the four barred stages refuse it
   through `model_state_reads`. `Universe::classify` puts `forward` before
   `foreign` before `stage` before `unbound`, as the criteria say.
   `NodeKind::Attribute` is only object-attribute reads (record fields are
   `NodeKind::Field`), so a record binder's fields stay readable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Every template-level "checks" assertion reads only `ProtocolObligation` causes of a protocol that never compiles (its `effect`/`commit` nodes are uncovered). A pass-through refusal (limit, fault, any cause outside `wraps`) or a silently incomplete template (`Obligations::Unresolved`, swallowed when `content` refused) makes those assertions pass vacuously. | qsl-semantics/src/check/protocol_obligation/tests.rs:73-88; qsl-semantics/src/check/protocol_obligation.rs:1135 |
| FND-002 | low | A model scalar whose `model_value_type` errs is dropped with `.ok()?`, so a later `M::T` gives a user-facing `missing-name` for what is a broken intake invariant. | qsl-semantics/src/check/assemble.rs:1160-1177 |
| FND-003 | low | A pass-through refusal (limit or fault) keeps the typer's `Location` under `Location::root(Origin::Expression)`. `refusal_region` then resolves it with `self.region(&location)` against a protocol that has no expression tree, so its region is not the expression's. | qsl-semantics/src/check/protocol_obligation.rs:1133-1135; qsl-semantics/src/check/region.rs:270 |

## Verdict

PASS. No high-severity bug. The Int-sum equality exception is narrow and
sound, the scalar naming matches `model_value_type`, the boxing touches every
site, and the #956 merge keeps both halves in a sensible order. FND-001 is a
real test-intent weakness that the gap-analysis artifact (SR-12140) relies on.
FND-002 and FND-003 are recorded only.

## Dispositions

Round 2, reviewed at 379bf1dbdf19902daadc11834ae19e20573d641f (fix range `49d2f331c..379bf1dbd`).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | The pass-through half is fixed in 379bf1dbd: `obligation_causes` now panics on any refusal other than a `ProtocolObligation` cause or `content`'s `Unimplemented`/`BinderType`. The remaining half (a silently `Unresolved` template is swallowed while `content` refuses the uncovered `effect` node) cannot close until a template protocol compiles. The PR body names it under "Not in this PR"; it lands with C1b2. |
| FND-002 | accepted-no-change | Low; recorded only under the major-bugs-only bar. A failing scalar intake is a broken-invariant path, with no wrong result on valid input. |
| FND-003 | accepted-no-change | Low; recorded only under the major-bugs-only bar. It affects only the region of a limit or fault refusal, never whether a protocol is accepted. |
