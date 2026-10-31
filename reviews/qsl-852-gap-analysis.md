---
id: SR-1955002
title: "gap-analysis review of QSL-852 partial protocol acceptance"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-852; PR1955 frozen 7f6406d; qsl-eval/, qsl-semantics/, FR-390, TC-1070, TC-1073"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-390", type: references }
---

## Summary

Scoped acceptance reconciliation of FR-390 and the two protocol-slot test files. The delivered AC-1/2/3/4/5/9/10 cases have real behavioral oracles; this is partial acceptance and does not qualify the whole ticket or repository.

## Verdict

CONDITIONAL for this method and the stated partial scope. QSL-852 remains In Progress; this review does not authorize whole-ticket completion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reconciliation: quire matrix 0.37.1 (engine 0.50.3) over an exact scratch copy of FR-390 and qsl-eval/tests/it/protocol_slots.rs plus protocol_slots/acceptance.rs. This bounded copy was explicitly authorized to avoid the whole-repository matrix hang. Engine output has 11 tagged and 2 untagged criteria; AC-12/13 appear untagged because their source suites are outside this copy, so those two statuses do not diagnose the repository. No run-evidence store was read. The seven delivered criteria AC-1/2/3/4/5/9/10 all compute tagged. Neither AC-6 nor AC-11 being tagged establishes full completion.

Plan completion: not assessed

AC-1 checks both compensation templates, activation/protocol/registration/activation captures, true and false trigger/retry/recovery outcomes, effect/commit/finish, related ends, visible entries and no/yes canonical case indexes. AC-2 uses the actual returned five, subsequently returns nine from registration, and proves recovery false with the retained five versus true with the returned nine. AC-9 drives actual T, navigate-only C, and explicit-deref D through exact/all-one-short charged-counter limits; required nonzero subsets and retain kinds are independently asserted. D's two read charges each assert value_occurrences=1. AC-10 measures R/A/G/T/V separately, concatenates actual logs, uses sum and sum-minus-one at V, and reverses call order with equal totals.

Owned remaining work: AC-11 read-back evaluation parity is not implemented; the real reader admitting bytes is not an evaluation-parity oracle, and typed Unsupported does not satisfy it. QSL-852 remains In Progress, dependent on IR-1193/IR-1194 and STD-611 for admitted terms and the lossless explicit-deref distinction. The bare and explicit forms emit equal graphs despite distinct checked evaluation logs; this is existing upstream scope, not a new defect of this slice. Complete AC-6 observation/reference/refusal qualification and AC-12 source/reader malformed-capture qualification remain with QSL-852; no full qualification claim is made. AC-7/8/13 are context only. The separate shared-view contract PR1954/QSL-1371 is not reviewed here.

Reverse ownership: changed slot selection and admission map to FR-390; model vectors map to FR-107 and value-accounting; related lowering/ordering map to FR-383 and QSpec FR-466. No new unowned behavior or source/test stubs found in the changed slice. Intent/test/code checks were explicitly requested by the dispatch and performed in this combined review; no separate whole-repository semantic audit is claimed.

## Examined units

Each unit below records its complete acceptance text or the specific changed statement. Examined partial AC-6/11 subproperties do not claim full criterion completion; see the gap report's owned remainder.

```yaml
{
  "scope": [
    {
      "id": "FR-390-AC-1",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "Over the refund protocol of TC-1070, each slot of the table evaluates to the value TC-1070 gives: the activation guard, the three capture stages, the trigger guard, the retry relation and the recovery predicate of two templates (each with the trigger ready, expecting false), the constraints of an `effect`, a `commit` and the `finish`, and the `related` ends, `visible` entry and case guards of a `choice`."
    },
    {
      "id": "FR-390-AC-2",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A capture is a frozen value: a capture slot returns the value its initializer gives from its stage's supply, and a later slot reads only the value the caller supplies for it. A retry relation or recovery predicate called with the total registered as 5 sees 5 when the record the registration read now holds 9, and with 9 supplied sees 9."
    },
    {
      "id": "FR-390-AC-3",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "An empty supply refuses `invalid_runtime_input`/`missing-member` naming the lowest source of the slot; a supplied binder the expression does not read, including the forward binder at a retry, refuses `unknown-member`."
    },
    {
      "id": "FR-390-AC-4",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A node that does not exist, a node of another tag, a member the operator lacks, an index out of range, a missing index and a surplus index each refuse before any charge with the code, cause and locus TC-1071 lists."
    },
    {
      "id": "FR-390-AC-5",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A duplicate, an unknown and a missing source refuse in that order, each naming the binder; a binder of another template is unknown, and the order among several is ascending level."
    },
    {
      "id": "FR-390-AC-6",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A value of the wrong shape, a record of another type, a value outside its type's domain, a reference to another model's population and a dangling reference each refuse with the code and cause TC-1071 lists; no observation with a reference, and an incomplete population, return `Incomplete`; an observation FR-106 refuses is refused with its record."
    },
    {
      "id": "FR-390-AC-7",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "A source supplied `Unavailable` and read returns `Unavailable` naming the binder and the read, with the charges already made; the same source in a call whose short-circuit never reads it completes."
    },
    {
      "id": "FR-390-AC-8",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "A capture initializer whose dispatched call has a false precondition gives the capture slot an `Undefined` result with reason `precondition-false`, category undefined, exit 10; the same slot with the precondition true completes."
    },
    {
      "id": "FR-390-AC-9",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "With usage `U` from a completed call, a meter with every charged counter at exactly its measured usage completes; with each charged counter in turn one below its usage and the others at theirs, the call returns `Incomplete` naming that counter with no value; a zero `work_units` returns `Incomplete`; a fresh meter completes the same call with the same value. Forwarded scalars and literal constants with no existing retain point have zero `result_units` by S6a; tests assert those zero usages independently of the charged-counter boundary loop."
    },
    {
      "id": "FR-390-AC-10",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "One meter passed to a compensation's registration capture, activation capture, trigger guard, retry relation and recovery predicate in turn accumulates exactly the sum of the calls' charges; a `work_units` budget of that sum completes all of them and one below it returns `Incomplete` `work_units` at the last call; the calls in another order give the same total."
    },
    {
      "id": "FR-390-AC-11",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "Running one call twice gives equal results and charge logs; a call over the package the contract reader returns for FR-383's emitted nodes gives the same result and charge log as over the checked package that was emitted; a refused call leaves the meter as it was."
    },
    {
      "id": "FR-390-AC-12",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "A package whose capture initializer reads itself, a later capture or another template's capture is refused at S3 (FR-381) and no package is built; a package mutated in its bytes so an activation capture's initializer reads its own binder, or another template's capture binder, is refused by the contract reader with `missing_declaration`/`missing-name` (`read` `forward` and `foreign`), and no evaluation call is possible."
    },
    {
      "id": "FR-390-AC-13",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "With the S6a probe variant of FR-107-AC-6 added, the build fails with E0004 at exactly the checked-in S6a seam list: the arm serving state clauses and protocol slots is the one `ProtocolClause` arm."
    },
    {
      "id": "FR-390-retention",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A completed value is retained and charged only at the existing S6a retain points of its expression. A forwarded scalar or literal constant that reaches none has zero `result_units` by S6a; returning it from a protocol slot does not add a retain charge."
    },
    {
      "id": "TC-1070-rows-29-33",
      "path": "spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "Case indexes name the emitted UTF-8 label order, so `no` is index 0 and `yes` is index 1 even though the source declares `yes` first."
    },
    {
      "id": "TC-1073-C-D",
      "path": "spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md",
      "role": "examined",
      "excerpt": "**D** is the variant of C whose initializer is `deref(view).total`, with the same observation and supply. C is navigate-only; D charges both `model.deref` and `model.navigate`."
    },
    {
      "id": "TC-1073-retention",
      "path": "spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md",
      "role": "examined",
      "excerpt": "Assert these point sets and the zero usages independently of the measured-boundary loop: a missing charge must not remove its own test coverage by making a counter appear uncharged."
    },
    {
      "id": "FR-107-model-reads",
      "path": "spec/functional/FR-107-evaluate-state-clauses-at-s6a.md",
      "role": "context_only",
      "excerpt": "The evaluator SHALL charge `model.deref` for each `deref(...)` and `model.navigate` for each field read, as `value-accounting.md` states."
    },
    {
      "id": "QSpec-FR-466-related",
      "path": "quire-specification/spec/objects/interfaces/FR-466-checked-package-protocol-control-bodies.md",
      "role": "context_only",
      "excerpt": "sorted by the relationship's node id, then the JCS bytes of `from`, then those of `to`"
    }
  ]
}
```

## Validation evidence

Independent reviewer checks: cargo fmt --all -- --check passed; scoped locked clippy with -D warnings passed; protocol_slots filter passed 16 tests (including two fixture-support tests); explicit model-read unit filter passed one test. Each touched spec validated at its explicit path through make check-spec-validation. Logs: /tmp/codex1-qsl-852-review-{fmt,clippy,protocol,model,fr390,tc1070,tc1073}.log. Full aggregate gate remains the code lead's responsibility; no aggregate or whole-repository qualification claim is made. Installed modules emit duplicate-archetype and inline-data-schema advisory diagnostics, with exit zero.

