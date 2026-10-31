---
id: SR-1955001
title: "code-review review of QSL-852 partial protocol acceptance"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-852; PR1955 frozen 7f6406d; qsl-eval/, qsl-semantics/, FR-390, TC-1070, TC-1073"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-390", type: references }
---

## Summary

Reviewed all nine changed Rust files and their callers against the locked contracts, Rust conventions, and FR-390. The behavior and oracles are sound for the delivered slice; one added copy of the kernel counter vocabulary violates the mandatory duplication rule.

## Verdict

FAIL for this method and the stated partial scope. QSL-852 remains In Progress; this review does not authorize whole-ticket completion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new COUNTERS array duplicates the locked kernel's public LimitKind::ALL; remove the array and iterate LimitKind::ALL so the complete counter vocabulary has one owner. | qsl-eval/tests/it/protocol_slots/acceptance.rs:514-525 |

## Coverage

The finding is scoped to the newly added array. Search covered qsl-eval/tests and the locked quire-exact 6ae189e checkout: its src/accounting.rs:106 exports LimitKind::ALL with exactly these members in this order. qsl-eval/tests/it/accounting_boundaries.rs:48 also has an older local copy; that older debt does not require expanding this fix. Replace the new array and its uses with LimitKind::ALL, preserving independent expected nonzero subsets.

The implementation charges model reads before object work, adds no charge point, and retains typed admission precedence and unchanged meters on refusals. Related records sort by NodeKey order then canonical JSON target text, which preserves the JCS order of reference terms; canonical emitted record identities drive selection and duplicate removal. Case labels sort as UTF-8 bytes. Trigger frames include protocol captures and input before canonical binder-level mapping. Finish accepts only its constraint. Wrong record declarations select wrong-value-kind using the existing type environment.

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
    },
    {
      "id": "qsl-eval/tests/it/protocol_slots/acceptance.rs::COUNTERS",
      "path": "qsl-eval/tests/it/protocol_slots/acceptance.rs",
      "role": "examined",
      "excerpt": "const COUNTERS: [LimitKind; 10] = [\n    LimitKind::IntegerBits,\n    LimitKind::DecimalDigits,\n    LimitKind::ScaleExpansion,\n    LimitKind::TextInputBytes,\n    LimitKind::TextScalars,\n    LimitKind::NormalizedScalars,\n    LimitKind::UnitEdges,\n    LimitKind::ValueOccurrences,\n    LimitKind::WorkUnits,\n    LimitKind::ResultUnits,\n];"
    }
  ]
}
```

## Validation evidence

Independent reviewer checks: cargo fmt --all -- --check passed; scoped locked clippy with -D warnings passed; protocol_slots filter passed 16 tests (including two fixture-support tests); explicit model-read unit filter passed one test. Each touched spec validated at its explicit path through make check-spec-validation. Logs: /tmp/codex1-qsl-852-review-{fmt,clippy,protocol,model,fr390,tc1070,tc1073}.log. Full aggregate gate remains the code lead's responsibility; no aggregate or whole-repository qualification claim is made. Installed modules emit duplicate-archetype and inline-data-schema advisory diagnostics, with exit zero.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Disposition pass 1: fb234c80dc5df4f71683815e8f0efed6aa9fd934 removes the added COUNTERS array and uses the locked kernel's LimitKind::ALL at all five sites. Independent T/C expected nonzero sets and retain-point assertions remain. |

## Disposition pass 1

Current code-review verdict: PASS for the reviewed partial acceptance slice at fb234c80dc5df4f71683815e8f0efed6aa9fd934. The original finding and review-pass FAIL verdict above remain immutable history; the latest disposition resolves the only merge-blocking finding. The fix diff contains only the catalog substitution and verbatim custody copies of all five review artifacts. No new defect or regression was found in that diff.

Verified the removal and all five replacements directly in the fix commit, the locked public LimitKind::ALL's identical ordered members, and unchanged independent expected T/C subsets. Reviewed coder round-1 logs: formatting passed with empty diagnostics; scoped locked Clippy with -D warnings finished successfully; protocol_slots passed all 16 tests. Logs: /tmp/codex1-qsl-852-round1-{fmt,clippy,protocol}.log. No aggregate gate or matrix was run for this disposition pass.

The slice is mergeable from this review's perspective after the lead's required aggregate checks and commitment of this updated artifact. The scoped gap verdict remains CONDITIONAL: AC-11 read-back evaluation parity and complete AC-6/12 qualification remain open with QSL-852, including dependencies IR-1193/IR-1194 and STD-611. QSL-852 remains In Progress; this disposition grants no whole-ticket completion.


