---
id: SR-20261105101076
title: spec-review/integrity review of QSL-1767 bare union construction
type: SpecReview
analysis: integrity
scope: agent-ix/quire-spec-language; QSL-1767; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md;
  spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md;
  spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md;
  spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md;
  spec/test-cases/TC-1333-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
review_set: subset
---

## Summary

Reviewed frozen source 0c5755042de18724267213c657c846ed4d16c2c8 against main 695eca34. Ticket: QSL-1767.

## Verdict

**PASS** — No defects found in this method’s changed scope; runtime verification remains pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

All five changed spec artifacts and the changed production/test surface were inspected. The carrier and evaluator obligations agree across FR-517, FR-531, FR-533, TC-1331 and TC-1333. Existing relationships are retained; no new relationship edges or domain-object definitions were introduced. Every added AC names its verification and has an authored test binding.

No applicable AssuranceProfile found under spec; no enforced review workflow selected. No dependency lens (no relationship changes), object lens (no domain-object changes), or Jev criterion-strength lens selected.

## Examined units

```yaml
scope:
- id: FR-517-AC-23
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: 'The writer emits the required `identities/3` source-record and source-union
    lists from the same final emission as the package: original opaque keys, final
    structural/type references, original member identities and order round-trip through
    native joins. The reader requires both lists and refuses the old format or an
    omitted list; no optional compatibility path or source fallback supplies them.'
- id: FR-531-AC-8
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: The handoff directory of **R** holds required `identities.json` with `format`
    `quire.checked-handoff.identities/3`; `ReadIdentityRows::read` over it and the
    package gives a slot call that evaluates; the file equals, byte for byte, the
    encoding above of the in-process unit's rows, and two writes of equal requests
    give equal files; omitting the file from a `BareSupply` makes the slot call refuse
    `MissingIdentityWitness` while the temporal call is unaffected.
- id: FR-531-AC-26
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: An authentic final emission's source-union rows round-trip original type
    handles, settled keys, sealed member identities, final structural and payload
    node IDs and ordered Exact payload types; the read rows equal the emitter rows,
    native `source_union_rows` retains them and rewriting is byte-identical.
- id: FR-531-AC-27
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: '`/2`, absent `source_unions`, malformed member identity, stale member
    name, foreign source owner and conflicting union rows return typed format, malformed
    or native-join refusals without a partial read. An empty union list never fabricates
    a producer witness from the graph.'
- id: FR-533-AC-20
  path: spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
  role: examined
  excerpt: 'A strictly admitted variant of **R** constructs `Shape::Circle(sum<M::Total>(receipt
    in recoveredFull.receipts: receipt))` and matches `Circle(r)` in `Full`''s recovery
    predicate. Matching, mismatching, empty and activated supplies produce the source
    route''s Boolean values and ordered charges. The bare environment preserves the
    source union''s original type handle, sealed declaration/member identities, member
    order and payload types; changing the payload changes the predicate, while adding
    an unrelated union leaves it unchanged. Missing required type projections refuse
    `MissingTypeRepresentation'
```

## Validation evidence

`quire validate` accepted this explicit scratch artifact. Targeted specification validation accepted all five changed documents: 5/5 grammar-clean; zero grammar findings. Execution evidence remains pending; no tests or build gates were run by the reviewer.

## Disposition pass 1 scope

Static re-review of frozen 0915ab2f15f26587f51678405d311bcdab72988f. Main rebase documentation outside QSL-1767 is context only. No source edits, builds, tests, global matrix, Kani or new audit were performed.

### Current assessment

**PASS** — no new finding in the changed scope. Earlier evidence limitations remain.

The candidate set consists only of existing record projections with native record views and authentic joined source-union rows. Unrepresented composite/model-record/tuple/object declarations and projected Composite handles lacking a candidate seed the blocked set. Native reference and semantic-type reverse edges propagate blocking transitively. Each ID enters the pending set once; cycles terminate. A represented recursive strongly connected set with no missing dependency remains admitted. Aliases/containers follow their admitted native references and projected original handles, with no display-name or structural-key fallback. No tuple rows, type interpreter or identity reconstruction is added. Shared TypeEnvironment remains the final declaration validator.

The selected projection checks every node in its arena, including reachable function bodies, using shared nested-type membership. Unused unsupported unions are omitted from the world; selected use of an omitted union becomes MissingTypeRepresentation before evaluation charges. Tests explicitly compare genuine emitter member identities and payload types, so omitting an authored unsupported union cannot make the regression vacuous. Only new trace bindings were required to use TC-first pairs; the pre-existing legacy tag is preserved.

Observed evidence: /tmp/qsl1767-payload-red.log records the two intended pre-fix failures (global closure fault and non-Admission selected failure), with a third existing control passing. The lead reports earlier-head baseline recovery/bare/unit passes. No post-fix compile, green regression run or clippy receipt exists; these are not claimed here.

## Final evidence and narrow integration disposition

Final narrow integration review at 851f257ba19750040e04e2cd2f1c4d846a057c6d: PASS. The added source-record branch consumes native source_record_view, preserving record_key and producer member order, original_member_identity, value_type and presence. record_member_rows pairs these fields from the actual original declaration; this restores source-record membership without names-derived identity, new transport, relaxed type checks or a weakened fixture. Model record_view and source_record_view are distinct native projection families, and both pass through the same closed-composite filter. No new finding was identified.

Observed final logs: recovery 17 passed; bare_package 26 passed; source_union_rows 8 passed; source_record_rows 10 passed — 61 tests, zero failures. Both original high-defect regressions are explicitly green in final-recovery.log; the earlier intended-red log demonstrates both fail on the original bug. Package and root clippy logs finish successfully; the lead reports -D warnings and exit 0. fmt log is empty and the lead reports exit 0. The lead reports eleven changed docs validated with exit 0; the validation log contains module-discovery advisory diagnostics. /tmp/qsl1767-focused.log records all four passing test totals. Logs are /tmp/qsl1767-final-{recovery,bare-package,union-carrier,record-carrier,package-clippy,root-clippy,fmt,spec-validation}.log. No reviewer builds or tests were run. The final combined gate has not run and is not approved by this review.

Both original code-review findings remain fixed; their original text and round-1 dispositions are preserved. No new findings or dispositions are required for already-fixed items. Code review current verdict: **PASS** within the reviewed feature and narrow source-record integration. Gap-analysis retains **CONDITIONAL** solely for the previously unavailable computed matrix; no new global matrix or audit was requested or run. Other four spec methods remain **PASS**.

## Mechanical integration disposition

Narrow mechanical integration disposition at 188bbfc8d324ce1347b19b5ca59441ded6025eca: STATIC FIXED / CONDITIONAL pending focused execution. Commit changes exactly two lines in qsl-eval/tests/it/protocol_slots/acceptance/bare_slots.rs, adding rows.source_unions().to_vec() as the fifth ReadIdentityRows::of_rows argument in unknown_rows and stripped. Both use authentic supplied union rows; unknown_rows still supplies the conflicting declaration set, and stripped still removes the field rows. Neither adverse oracle nor the source algorithms changed.

The first combined workspace/all-targets gate at 8054b164b failed with exit 101 and exactly two E0061 missing fifth-argument errors before subsequent gate stages. The original receipt /tmp/QSL1767-combined-final-8054b164b.log was inspected and preserved. This commit directly supplies those two omitted arguments. Focused workspace check and bare_slots group are reported queued, not passed. The prior 61-test and clippy evidence applies to the earlier reviewed source; it does not establish this new head or an all-targets gate pass. The combined gate is PENDING. No builds, tests, matrix, broader audit or source edits were performed by the reviewer.

No new substantive review finding in this two-line fix. Both original findings retain their fixed dispositions. Current integration verdict: **CONDITIONAL** pending focused receipts and the final combined gate.

## Caller-fix verified evidence supplement

Caller-fix runtime supplement at 188bbfc8d324ce1347b19b5ca59441ded6025eca: scoped review PASS. Inspected /tmp/qsl1767-callers-workspace-check.log: workspace/all-targets check finishes successfully in 4.62s (lead reports exit 0). Inspected /tmp/qsl1767-callers-adverse-tests.log: bare_slots acceptance group 12 passed, zero failed, including every_witness_refusal_of_the_call_is_at_the_package_locus and the_call_reports_identity_then_equality_then_node_refusals. This verifies both authentic-union-row caller migrations. Earlier 61 focused passes remain recorded; the new 12 passes are separate evidence. Both original findings remain fixed; no new finding. The prior 8054 gate failure remains historical: it stopped at the first compiler stage and ran no test stages. The corrected final-head combined gate remains PENDING; this scoped PASS makes no combined-gate claim. No reviewer builds, tests, matrix, broader audit or source edits were performed.

The previously conditional two-line mechanical integration is now runtime verified. Gap-analysis retains its existing computed-matrix limitation; other method conclusions and all history remain unchanged.
