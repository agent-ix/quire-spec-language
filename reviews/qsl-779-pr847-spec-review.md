---
id: SR-9131
title: "base spec review of QSL-779 FR-277 supplied numeric magnitude bound"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a2ad347d348fa7d6db1cdbfb5f92c0aff87673da; spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md"
review_set: subset
---

## Summary

Ticket: QSL-779. PR quire-spec-language#847 adds the caller-owned magnitude
bound B (`supplied.numeric_magnitude_bits`, default 4096) to FR-277's
`limits.supplied`, a new FR-277-AC-7 and TC-906 step 10. Every FR-277
statement about B was compared with QSpec FR-323 (merged), quire-exact FR-380
and QSV FR-109: default 4096 in the caller API, omitted equals stated 4096,
refusal classes, the `limit` cause for the over-B stop, zero as one bit, the
charged event first, the 3/6 value-in-domain operands and the type-link norm
all agree. FR-277 places the default in the caller limits and never gives the
kernel or the QSV helper a default, which matches FR-380 Inputs and AC-6.

One blocking defect: the step 10 / AC-7 boundary vector does not name the
Integer parameter's type. Under QSL's literal `Integer` (unranged) there is no
value-in-domain comparison, so B never applies and the 4097-bit value admits.
Under the step 1 `Int[0, 9]` field the 4096-bit value fails domain membership.

## Verdict

FAIL. Fix FND-001, then pass. FND-002 and FND-003 are recorded only.

## Examined Scope

```yaml
scope:
  - {id: FR-277, path: spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, role: examined, excerpt: "supplied.numeric_magnitude_bits (B) is the caller-owned finite magnitude bound for Integer and Rational supplied-value admission ... An omitted B in the caller API resolves to the published default 4096, so an omitted and a stated 4096 are the same request"}
  - {id: FR-277-AC-6, path: spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, role: examined, excerpt: "Omitting any premise blocks implementation qualification, not a fabricated runtime result."}
  - {id: FR-277-AC-7, path: spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, role: examined, excerpt: "At the default B a supplied Integer of 4096 bits admits and one of 4097 bits stops with the limit cause for supplied.numeric_magnitude_bits"}
  - {id: TC-906, path: spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md, role: examined, excerpt: "10. Magnitude bound B: ... replay an Integer parameter given a 4096-bit value, then a 4097-bit value (zero is one bit)."}
  - {id: FR-255, path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, role: context_only, excerpt: "FR-277's supplied.admission_work_units and supplied.conversion_work_units, which are not setting-table rows"}
  - {id: FR-098, path: spec/functional/FR-098-execute-a-replay-request.md, role: context_only, excerpt: "Supplied admission/conversion ceilings SHALL instead use the paired QSpec FR-323 typed limits.supplied carrier"}
  - {id: FR-321, path: spec/functional/FR-321-admit-supplied-union-values.md, role: context_only, excerpt: "Supplied admission SHALL use the distinct public supplied.admission_work_units ceiling"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-277-AC-7 and TC-906 step 10 say "an Integer parameter" with no declared interval. QSL's `Integer` is unranged (ADR-014), and QSV FR-109 treats an unranged Integer as a fixed kind check with no interval comparison; QSpec FR-323 checks B only on operands of a value-in-domain comparison. So the 4097-bit value admits and the vector cannot fail. Read as the step 1 `Int[0, 9]` field, the 4096-bit value fails domain membership instead of admitting. Fix: name an interval type whose endpoints are each at most 4096 bits and that contains the 4096-bit value (for example `Int[0, 2^4096 - 1]` in literal digits), with the 4097-bit value stopping on operand `value`. | spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:393; spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md:143-148,223-227 |
| FND-002 | medium | FR-255 Behavior 1's supplied-value row lists only A and H as settings. B belongs to the same carrying type and gives the same pre-call limit record. The row is incomplete but not false, and FR-255-AC-1/3/4 still hold. FR-255 is parsed through include_str! by qsl-replay/src/limits.rs, so run that test if the row is edited. Recorded. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:156 |
| FND-003 | low | The TC-906 step 10 expected result gives `limit` 4096 and zero evaluation consumption only. It leaves out FR-323's `operand: value`, `consumed` 0, `next_charge`/`counter` 4097 and `event: value-visit`. It does not contradict them. Recorded. | spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md:223-227 |

## Evidence and Limits

The FR-323, TC-278, FR-380 and FR-109 oracles were read from each owner's
origin/main. No Rust test parses FR-277 or TC-906 (`git grep 'FR-277\|TC-906'
-- '*.rs'` shows trace tags and doc comments only, with no include_str!). The
stale-text sweep found no `two ceilings` or `both ceilings` phrasing anywhere.
FR-098:109-111, FR-321:66 and TC-830:33 name A and/or H only and are still
true. No build ran.
