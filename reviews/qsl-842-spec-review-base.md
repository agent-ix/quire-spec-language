---
id: SR-12096
title: "base review of PR 944 (FR-308, FR-218, FR-381 protocol S2/S3 spec)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8fdb69ac17e5f7c148c7317db892d127b3299ec7; spec/functional/FR-218-*.md, spec/functional/FR-308-*.md, spec/functional/FR-381-*.md, spec/test-cases/TC-1055..TC-1058, spec/spec.md"
review_set: subset
---

## Summary

Ticket: QSL-842. Base checklist over `git diff origin/main...HEAD`: ID formats and continuity, AC
testability (one behaviour, typed code/cause/member/stage/subject), AC to TC mapping, present-tense
state-what-is wording, and mechanical validation. No high findings in this lane; three medium and three low.
Mechanical: `quire validate --scope .` on all eight touched files clean (only module-registry warnings);
`make check-spec-validation` EXIT=0 with no warning in a touched file; `tools/check-index-completeness.sh`
EXIT=0. quire 0.36.2 (engine 0.50.2), quoin 0.28.3.

## Scope

Examined: FR-308-AC-7..AC-12, FR-218-AC-10..AC-19, FR-381-AC-1..AC-12, and every row of TC-1055 (13 rows),
TC-1056 (40), TC-1057 (22), TC-1058 (25). AC numbering continues each file (FR-308 from AC-6, FR-218 from
AC-9); TC-1055..1058 continue from TC-1054; FR-381 is the next free FR. Each TC states its AC range and
every AC has at least one row. Added lines carry no ticket ids, history or roadmap words.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-218-AC-18 states its four refusals only as "refuse as an out-of-scope read, naming the binder", with no code, cause or retained members; TC-1056 rows 35-36 repeat the prose. The Scopes section resolves it to FR-113's `missing_declaration`/`missing-name`, but the AC itself is not a typed assertion. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:330; TC-1056 rows 35-36 |
| FND-002 | medium | The FIFO key rule "the key's result type SHALL be one over which the clause checker admits `=`; another type refuses `ill_typed`/`operator-ineligible` ... retaining `operator` and the type" has no AC and no TC row. FR-218-AC-12 covers binder type, model read and `unordered` only. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:162-165,324 |
| FND-003 | medium | FR-381 says "every node body in the `run` tree ... may read the protocol's captures", while FR-218 says a choice or repeat guard and `visible` entry read eligible decision facts and constants only, and its eligibility table never lists captures. Whether a guard may read a capture, and with which refusal if not, is undefined. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:115-116; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:240-250 |
| FND-004 | low | The S1 overflow case (`attempts 18446744073709551616`) is asserted with no code or cause ("a parse diagnostic", "fails at S1") in FR-308-AC-11, FR-381-AC-7, TC-1055 row 11 and TC-1058 row 11; the FR-381 copy also restates an S1 behaviour inside an S3 FR. | spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md:162; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:235 |
| FND-005 | low | FR-381 Description's second paragraph ("The checks sit beside FR-218's rather than inside it ...") is a design rationale, not behaviour. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:47-50 |
| FND-006 | low | TC-1058 row 5 ("the base protocol, compiled: the package holds one anchor node and one frame node") asserts S4 package content from an S3 TC; FR-381-AC-5 says the protocol "holds" them. Also "Two equal scalar ids never satisfy a relationship end" (FR-218) has no AC. | spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md:row 5; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:199-200 |

## Verdict

No blocking issue in this lane; the blocking issues are in SR-12097 (integrity). Clean: FR-308-AC-7..12
(each a single typed-form assertion with spans, and `RecoveringCst` with no partial form), FR-218-AC-10..17
and AC-19 (typed code/cause/members at a named span), FR-381-AC-1..12 (typed code/cause with `stage`,
`owner`, `read`, `member`, `value`, `reason` members). Hand-checked the TC-1056 partition rows 37-40
valuation by valuation: each `failed` set is correct. TC-1056 row 21's swapped-end expectations match the
fixture's `pipe` end types.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-218-AC-18 now states `missing_declaration`/`missing-name` retaining `name`. |
| FND-002 | fixed | New FR-218-AC-20 and TC-1056 row 42 state the FIFO key `=` rule; the concrete type that admits no `=` is left to the test author. |
| FND-003 | fixed | FR-218's eligibility rules list protocol captures as readable in guards and `visible` entries; TC-1056 row 41. |
| FND-004 | recorded | The S1 overflow diagnostic belongs to FR-325's `u64` rule; the ACs cite it. |
| FND-005 | fixed | The rationale paragraph in FR-381's Description is deleted. |
| FND-006 | recorded | TC-1058 row 5 is the FR-114 package assertion; the scalar-id relationship end has no AC and is recorded. |
