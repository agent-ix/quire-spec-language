---
id: SR-20261105101082
title: QSL-1795 code-review review of complete native predecessor authority
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; QSL-1795; spec/functional/FR-052-publish-native-temporal-evaluation-owner.md;
  spec/test-cases/TC-144-checked-package-temporal-evaluation-owner.md; src/temporal.rs;
  src/temporal/owner.rs; src/temporal/owner/predecessor.rs; src/temporal/owner_codec.rs;
  tests/it/main.rs; tests/it/native_owner_report.rs; tests/it/native_full_predecessor.rs
review_set: subset
---

## Summary

Reviewed all changed Rust production/test files, actual callers, old codec invariants and public export paths. The native producer retains the full direct predecessor as flat Arc bytes; v2 strict reading compares all prior bytes with the supplied actual Report before admitting the current value. No production/test stubs, copied external definitions, unsafe code, new dependency or weakened CI gate found. The new admission path repeats a pre-existing ledger-key encoding before native-owner precharge (FND-001). Bounds precharge logical row/support/predecessor/final frames and comparisons, with native-domain limits separate from the unchanged shared IR NativeJoinBudget.

## Verdict

FND-001 is fixed statically in disposition round 1; overall merge clearance is CONDITIONAL on actual focused compilation/tests and required final fresh-main gate evidence. No runtime pass is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | New predecessor admission repeats ledger/obligation encoding before its native-owner resource budget exists, so zero limits still perform added uncharged allocation/work. | src/temporal/owner/predecessor.rs:143 |

## Coverage

Reviewed frozen head `2a1fa9113a37e89696248e4afeb3b121d84f18f0`, against origin/main. No applicable AssuranceProfile was found in the repository. No source edits/build/commit/push operations performed by this reviewer. Compilation, Clippy, tests, cargo-deny and runtime execution are unavailable in this static pass while the shared Cargo lock blocks the coder; root owns the final combined-tree gate. Jev criterion-strength analysis was not selected; no dependency edges or domain-object vocabulary changed.

Plan completion: not assessed

```yaml
scope:
- id: FR-052-AC-19
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: A genuine correction requires the actual full direct predecessor report;
    its retained predecessor owner bytes equal the complete prior report bytes, and
    non-evaluating validation accepts clones/moves of that value. A different full
    outcome/support value with equal request/result identity, a skipped or cross-binding
    report, and a missing actual predecessor refuse. Strict reading rejects mutated
    predecessor value bytes and unsupported old format rather than inferring authority
    from labels.
- id: FR-052-AC-20
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: Full-predecessor issuance, reading and validation charge their stated native-owner
    work/storage limits before inspection or allocation; exact measured limits admit,
    one less returns the original typed resource/ceiling/attempt with no partial relation,
    and existing caller NativeJoinBudget consumption is never reset.
- id: FR-052-AC-21
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: Repeated native corrections retain complete prior representations as flat
    bytes; clone, compare, encode and drop do not recursively traverse or own Report
    chains, and work/copies of retained predecessor bytes are explicitly charged.
- id: FR-052
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: context_only
  excerpt: When a checked temporal protocol obligation is evaluated through the Layer-4
    checked-package route, the QSL evaluator SHALL remain the sole owner of its temporal
    truth, settlement and decision support, with outcomes and failures consistent
    with FR-043 and activation inputs consistent with FR-044.
- id: TC-144
  path: spec/test-cases/TC-144-checked-package-temporal-evaluation-owner.md
  role: examined
  excerpt: Native correction with the actual prior Report and explicit finite ReportPredecessorLimits;
    then same identity but a different genuine complete report, a missing actual prior,
    skipped/cross-binding report, a clone and a moved actual prior
```

## Finding details

FND-001: `admit_predecessor` invokes `self.predecessor_entry()?` before reading the operation limits or constructing `Budget`. That helper serializes the selected node/occurrence and constructs an owned obligation key. Existing issuance already obtains the predecessor via `subject.predecessor()`, and strict reading already checks the ledger predecessor. The added repeated serialization/lookup is new work and storage outside AC-20 even when both caller limits are zero. Pass the already established predecessor identity into admission and preserve the existing lineage validation and first-denial behavior.

## Disposition round 1

Static re-review at `e672f6518dfd33565b137047a6b0195c97ad520f`: examined the complete fix delta, both admission callers, new zero-snapshot-budget request/lineage precedence regression and unchanged frame/copy accounting. No new defects found. Native admission now consumes an already-established exact predecessor identity; it does not repeat the ledger lookup or key serialization. The reader checks parsed predecessor identity against the actual request authority before passing it to admission. Evaluation preserves request validation and original lineage refusal before new snapshot work. Original bounds and shared NativeJoinBudget ownership remain unchanged.

Compilation is now progressing in dependencies; no successful compile/test/Clippy/runtime or final-gate evidence is available. Overall clearance remains CONDITIONAL pending those checks, and no integration consumer/runtime completion is claimed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Static closure at e672f6518dfd33565b137047a6b0195c97ad520f: admission borrows the already-validated exact predecessor identity. The repeated ledger/key encoding is removed; original request and lineage refusals precede snapshot work. New genuine zero-budget revision/stale-ledger controls are present, unexecuted. |

## Focused verification supplement

Focused verification supplement at `eeea720433bab6ad80ab778ae8618fe61b0bcddb`: inspected the test-only delta and successful logs `/tmp/QSL-1795-{test-check,clippy,default-check,predecessor-tests,owner-tests}.log`. Checked-handoff and default test compilation passed; root library/test Clippy with `-D warnings` passed (invocation reported by root, successful log inspected). The genuine full-predecessor suite passed 11/11; existing native-owner suite passed 11 with its existing independent two-build test ignored. The IR precharge now calls actual `project_exact_type` and asserts nonzero consumed work before native evaluation; the capture-free native operation correctly preserves these counters exactly. The checked-handoff cfg matches the existing genuine emitter fixture lane and default test compilation passes. Production code is unchanged since the reviewed FND-001 fix. No new defects found. FND-001 remains fixed.

**Mergeable once root’s final fresh-main combined-tree gate passes.** That final gate is still pending; this supplement does not claim integration consumer completion or an independently executed ignored two-build lane.

```yaml
reviewed_head: eeea720433bab6ad80ab778ae8618fe61b0bcddb
production_unchanged_since: e672f6518dfd33565b137047a6b0195c97ad520f
checked_handoff_test_compile: passed
default_test_compile: passed
root_lib_test_clippy: passed (-D warnings; root-reported invocation, successful log
  inspected)
native_full_predecessor:
  passed: 11
  failed: 0
  ignored: 0
native_owner_report:
  passed: 11
  failed: 0
  ignored: 1
  ignored_lane: existing independent two-build lane
runtime_clearance: true
final_combined_tree_gate: pending
merge_clearance: mergeable once root final fresh-main combined-tree gate passes
```
