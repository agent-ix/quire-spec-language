---
id: SR-12175
title: "Code review of quire-spec-language PR 1017: the EN-1 closure checker resolves ClauseIdentity from the package"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@00ca7115731e145fefe3ec1e5856be7005a09b75; PR 1017: qsl-replay/src/model_check/{identity,item,settle}.rs, qsl-replay/src/model_check.rs, qsl-replay/src/certificate/closure.rs, qsl-replay/src/lib.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/tests/it/{closure_certificate,counter,model_check}.rs"
review_set: subset
---
# Code review of quire-spec-language PR 1017

## Summary

Ticket: QSL-880. Code review with the rust-review lane folded in. It covers
the Rust files of `git diff origin/main...HEAD` only. No build was run; the
review reads the code.

- Identity. `ClauseIdentity::resolve` finds the request's clause among the
  package's `temporal_clauses()` by `CheckedTemporalClause` equality. It builds
  the `[node, "claim", ordinal]` triple from that package node, emits the
  package, reads it back through `read_import_view`, and returns what
  `resolve_clause_root` gives. Its fields are private and it has no other
  constructor, so a request cannot name an occurrence or an `over` node
  (FR-337-AC-5, FR-338 Inputs).
- Soundness question 1 (equality ignores the `holds` condition text). This is
  not a hole. A `CheckedGraph` has one source (`graph.source()`), so a
  declaration's span is unique in its package. Equality on name, span,
  profile, `over`, clock, operators, claim, formula arena, activation and
  fairness picks at most one declaration. The identity returned is always
  that package node's own triple and `over` node. A request clause taken from
  another package whose condition is `<= 2`, matched against a package
  declaring `<= 3` at the same span, gets the `<= 3` package's node. Every key
  context, the certificate and the settled `package_id` then name that
  package's clause. A certificate produced against the `<= 2` package carries
  that package's node, so it is a foreign context to this one and is rejected
  `ShapeMismatch`. The clause value is used only for its formula shape, and
  equality guarantees that shape is the package's. What the atoms mean comes
  from the caller's `ModelLetters`. That seam existed before this PR and stays
  trusted in exactly the same way: lying letters break a correctly matched
  clause just as easily. Matching on the condition text would not close it.
- Soundness question 2. An unresolved root cannot settle `Certified` or
  `Rejected`. `check_model` resolves before exploring, so no `Holds` outcome
  exists for it. In `settle_model_check`, `check_closure` returns
  `Err(ClosureRefusal::Item(..))` before any key is used, and the catch-all
  `Err(_) => failed()` arm settles it `Failed`. TC-1080's new test asserts
  `Failed`/`Failed`/`Execution::Failed` for a `Holds` outcome settled against
  a request whose clause the package does not declare.
- Soundness question 5 (no shim). The `identity` field of
  `ModelCheckItem::Clause`, the public `ClauseIdentity` literal, the
  `counter::identity()` fixture (`[0x51; 32]` / `[0x52; 32]`), the
  `counter::clause()` helper and the "S4 emits no temporal clause node yet"
  module note are all deleted. A repository grep finds no remaining struct
  literal, fixture id or `identity:` item field. The occurrence-moved vector in
  `a_certificate_of_another_item_is_not_admitted` is gone too. It is now
  covered by the real two-declaration vector in TC-1079's test (`C`'s
  certificate offered for `D` is rejected `ShapeMismatch`).
- `check_closure` maps `ClauseRootRefusal::Cancelled` to
  `Stopped(Cancelled{source})` and every other item refusal to
  `ClosureRefusal::Item`. The pre-cancelled `check_closure` tests now stop at
  the resolution's emission and still assert the right source.
- Counter formulas. `6 / (2 - c.value)` becomes `6 mod (2 - c.value)` in the
  TP-1 refutation and the `ContractUndetermined` vector. The atom oracle is
  the test's own `Letters` closure, which the formula text does not drive. At
  every state the asserted traces reach (values 0 to 2), `/` and `%` give the
  same truth, and both are undefined (`DivisionByZero`, "a divisor is zero")
  at 2. The deadlock half of FR-126-AC-9 already writes its division as `mod`
  for the same S3 reason. The asserted behaviour is not weakened. The only
  change is that in the `CONTRACT` vector, value 3 was a second, false bad
  state under `/` (`6 / -1 < 0`) and is a true state under `mod`. The test
  never asserted value 3, so nothing it checked is lost.
- Rust idiom. No `unwrap` or `expect` on the production path. No `#[allow]`,
  no ticket ids in comments, nothing vendored, no compatibility path. The new
  public types are `thiserror` enums with `#[from]` conversions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `check_model` turns a cancelled identity resolution into a refusal, which FR-126 does not allow. `instances()` now calls `ClauseIdentity::resolve`, which calls `emit_checked_with_cancel`. That function polls the handle on entry. `check_model` maps every `ItemRefusal` with `.map_err(ModelCheckRefusal::Item)`, so for a TP-1 clause item an already-cancelled (or mid-emission cancelled) handle returns `Err(ModelCheckRefusal::Item(ItemRefusal::ClauseRoot(ClauseRootRefusal::Cancelled(..))))`. It does not return `Ok(Stopped{cause: Cancelled, limit: None})`. FR-126 says "When the `Cancel` handle is cancelled ... the engine SHALL return `Stopped{cause: Cancelled, limit: None}`", and FR-126-AC-5 asserts that for an already-cancelled handle. Before this PR, `instances()` did not poll, so this is a regression for clause items. `check_closure` already handles the same case correctly. The only cancellation test, `model_check.rs`'s FR-126-AC-5 test, uses `ModelCheckItem::DeadlockFreedom`, which never resolves an identity. The fix is to mirror `check_closure`'s match arm in `check_model` and add a TP-1 already-cancelled vector. | qsl-analyze/src/model_check/check.rs:231-234; qsl-replay/src/model_check/identity.rs:76-80; qsl-replay/src/certificate/closure.rs:186-194 |
| FND-002 | low | Identity resolution emits and re-reads the whole package on every `instances()` call. A proved run pays three emissions and two reads in total. `check_model` emits and reads once. `settle_model_check`'s `Holds` arm emits again in `certificate_package`, then once more inside `check_closure`'s `resolve`, plus a full `read_import_view` (default `V2ReadLimits`, domain-package checks) each time. The results are correct and the cost is linear in the package size, but the settlement's emission could be passed to `resolve` instead of being rebuilt. | qsl-replay/src/model_check/identity.rs:76-96; qsl-replay/src/model_check/settle.rs:266-270 |
| FND-003 | low | `ClauseRootRefusal::Package(String)` erases typed causes. `EmitRefusal` (other than `Cancelled`), the `read_import_view` refusal and the `LibraryName` refusal are flattened to `to_string()`/`{:?}` text. The typed cause is lost to callers and tests, and the payload is Debug output instead of a stable message. Boxing the typed refusals in their own variants would keep them. | qsl-replay/src/model_check/identity.rs:78-93,121-123 |

## Verdict

PASS with one medium. The identity now comes from the package alone. The
fixture identity and the `identity` item field are fully deleted with no shim.
An unresolved root settles `Failed` and never `Certified` or
`CertificateRejected`. `CheckedTemporalClause` equality cannot confuse two
declarations in one package. A same-span clause from another package resolves
to this package's own node, so no certificate is accepted for a clause with
different semantics. The remaining trust in atom meaning is the existing
`ModelLetters` seam, which this PR does not change. FND-001 (medium) is a real
spec deviation: cancellation during resolution in `check_model` is reported as
a refusal, not `Stopped{Cancelled}`. It is not unsound, because no proof or
refutation results from it. FND-002 and FND-003 are low. None is a major, so
nothing blocks.
