---
id: SR-1316
title: "IR-601 code review of PR #639 (member-only division, quire-exact 2ec5e1e3)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1d27057837c187c1106d7445e7e806faa9aa669b; PR #639 diff against origin/main (merge base bcca43356); Cargo.lock; qsl-semantics/src/value/definition.rs; qsl-foundation/src/diagnostic.rs; qsl-semantics/tests/it/integer_division.rs; qsl-foundation/tests/kernel_refusal_record.rs; qsl-eval/tests/it/kernel_refusal_payloads.rs; qsl-replay/src/spine/call/tests.rs"
review_set: subset
---
## Summary

Ticket: IR-601. PR: quire-spec-language#639. Code review with the rust-review
lane folded in, scoped to the PR diff.

Checked:

- **Pair-rule removal.** `git grep -n -i "division_pair\|pair_out_of_domain\|both-outside-domain\|domain-pair\|result-pair\|QuotientRemainder\|DomainPair\|ResultPair"`
  at the head, excluding `reviews/`, returns nothing.
- **One source for causes.** `kernel_refusal_record` builds the code and cause
  from `refusal.code()`/`refusal.cause()` (diagnostic.rs:1010). QSL has no
  local cause table for division. At quire-exact 2ec5e1e3,
  `Refusal::cause()` maps `DivisionMember::Quotient` to
  `quotient-outside-domain` and `Remainder` to `remainder-outside-domain`
  (src/outcome.rs:254-256). The catalog category row is renamed to
  `division_out_of_domain`.
- **Lock.** `Cargo.lock` has one `quire-exact` package, at
  `2ec5e1e3da0223cda262289bf143756245d7381a`.
- **Public API.** `qsl_semantics::value::divide` now takes a
  `DivisionMember` and returns `Outcome<Integer>`. It is a thin delegate to
  `quire_exact::divide(profile, member, ..)`. The only callers are tests. No
  compatibility layer was added.
- **Test oracles.** The signed table holds literal `(q, r)` cells for all three
  profiles. The signed-64 test uses literal `i64::MIN`/`MAX` bounds and checks
  that `min div -1` refuses with `Quotient` while `min rem -1` completes with
  0. The generated test uses an `oracle` built on Rust's `/` and `%`, plus a
  separate `assert_law` check, so it does not depend on quire-exact.
  Accounting asserts the renamed `IntegerDivisionDomain`/`ResultRetain` points
  with `result_units` 1.
- **Focused runs at the head.** `qsl-semantics --test it integer_division`
  passed 12 of 12. The foundation, eval and replay tests touched by the PR are
  reported in the handback. The full `make ci` exit 0 was reported by the coder.

## Verdict

Approve with one low finding. The code change is correct and minimal, and the
causes come from the kernel. FND-001 is a wrong statement in a test doc comment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The doc comment on `only_the_exposed_member_must_lie_in_the_consumer_domain` says `10 div y` over `1..=10` at `y = 5` "evaluates to 2 although the remainder 0 and quotient are both fine". The remainder 0 is outside `Int[1, 10]`, and that is the point of the case (QSpec FR-147-AC-10: "returns 2 although the remainder 0 is outside the domain"). Fix: "evaluates to 2 although the remainder 0 is outside that domain". | qsl-semantics/tests/it/integer_division.rs:724-727 |

## Dispositions

Round 1, reviewed at 2156c32cedfe0b8da4a07cc0202d5746589fa298 (fix commit
2156c32ce on 1d2705783; `git diff 1d2705783 2156c32ce`). No builds were run in
this round. The coder reports `make ci` exit 0 (qsl-601-make-ci2.log), and the
log ends with exit=0. The round adds no new code-review finding.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2156c32ce |
