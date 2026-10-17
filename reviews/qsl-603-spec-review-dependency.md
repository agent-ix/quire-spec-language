---
id: SR-4953
title: QSL-603 dependency review
type: SpecReview
analysis: dependency
scope: agent-ix/quire-spec-language@725ffd66691c962e1bdcf6260a31d03430cfa5ed; FR-350–353,
  TC-876–881; QSL-603
review_set: subset
---

## Summary

Ticket: QSL-603. Reviewed only the static specification deliverable for QSL-603; implementation acceptance remains pending. All 16 FR criteria have corresponding TC-875–TC-881 procedures; these are planned procedures, not executable evidence. No applicable AssuranceProfile exists. Changed validation: exit 0, 10/10 grammar-clean; full spec validation: exit 0, 1536/1544 grammar-clean with 17 advisories outside this diff. git diff --check passed. No builds, lock jobs or executable tests ran. quire matrix --scope . --format json --strict exits 1: all 16 QSL FR-350–353 criteria are untagged with zero executable binders, consistent with the declared pending implementation. Source matches naming QSpec FR-351/352 are different requirements.

## Verdict

**PASS** — New edges are FR-255 → FR-350 and FR-255 → FR-353. Local dependency order is FR-096/FR-255 → FR-350 → FR-351 → FR-352 → FR-353; no cycle is introduced. FR-255 is shared settings enablement; FR-350–353 are qualification features. QSpec FR-311/336 and interface_019 own corpus semantics. Current FR-255 routes execution budgets through request fields; open PR #665 adds bare accounting names and request override semantics. The changed prose delegates to the effective FR-255 contract without inventing either vocabulary. #665 is still OPEN; no implementation readiness or merge of that dependency is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'When a maintainer runs the `qualify` repository gate (FR-353) over a

    conformance corpus,

    the runner SHALL execute every vector whose applicability names a QSL

    subject through the QSL public entry point for that subject, and SHALL

    compare the typed result with the vector''s expected disposition.'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'The corpus is QSpec''s (QSpec FR-336, interface_019). QSpec owns its vector

    shape, its classes (positive, boundary, just-outside, refusal, incomplete,

    metamorphic) and each vector''s expected disposition. This requirement

    specifies how QSL executes and compares them.'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: "- The corpus, read from the location the caller names (the QSpec\n  repository's\
    \ published corpus). The runner reads the corpus in place.\n- An optional capability\
    \ scope (FR-351).\n- Resource limits for each vector: the stage limits (FR-096)\
    \ and the\n  evaluation budget the compiler and runtime already take, each with\
    \ its\n  published default, each raisable by the caller."
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'One outcome for each executed vector: `passed`, `failed`, `unsupported`,

    `incomplete` or `tool-failure`, each holding the vector id, its capability

    id and, unless `passed`, the expected and actual typed result. `incomplete`

    and `tool-failure` are QSpec FR-311 `unexecuted` results, kept apart so the

    report names why the vector did not run to a result.'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'The QSL subjects are three QSpec FR-311 consumer types, each reached through

    the entry point a caller of that subject uses:'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'QSL reads QSpec FR-336 `applicability` at two levels. A capability is
    in

    the default scope (FR-351) when its applicability names a QSL consumer type

    and a semantic profile QSL''s definition catalog holds. A vector SHALL run

    when its capability is in the selected scope and its own applicability

    names a QSL consumer type, through that type''s entry point. Vectors of an

    out-of-scope capability SHALL NOT execute. The runner calls no internal

    function a caller cannot reach.'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'The runner SHALL read the selected QSpec corpus and the inputs it names

    without rewriting its vectors, expected dispositions, `results` or

    `coverage`. Expected dispositions come from the corpus''s independent oracle;

    the runner SHALL NOT replace them with results calculated by QSL.'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'The runner SHALL resolve `--limit` operands through the existing

    [FR-255 settings operation](FR-255-name-the-setting-that-raises-a-reached-limit.md).

    Q1 owns no additional setting names or defaults. The subject receives those

    effective limits at its public entry point; a setting for a stage that the

    subject does not run does not cause that stage to run. Evaluation budgets

    retain the subject''s accounting contract, including any override the

    FR-255 settings operation defines.'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'If executing a vector panics, returns an internal fault, or cannot read
    a

    file the vector names, then its outcome SHALL be `tool-failure`, holding

    the panic message, the fault or the I/O error. Otherwise the runner SHALL

    settle the vector by the first of these, in this order, that holds:'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: "1. `passed`: the actual result matches the expected disposition. A value,\n\
    \   verdict or checked package identity matches when it is equal. A refusal\n\
    \   matches when it has the same catalog code and cause, and the same locus\n\
    \   when the vector names one. This includes an expected limit refusal\n   (a\
    \ boundary or `incomplete`-class vector) and an expected\n   `unknown_required_feature`\
    \ or `unsupported_projection` refusal.\n2. `unsupported`: QSL returned `unsupported_projection`\
    \ or\n   `unknown_required_feature`, holding the code and cause QSL returned.\n\
    3. `incomplete`: the vector reached a"
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: " resource limit, retaining its kind,\n   bound and actual charge/count,\
    \ and the setting or request field through\n   which the subject's public contract\
    \ raises it.\n4. `failed`: any other result, holding the expected and the actual\
    \ result.\n   A refusal with the right code and a different cause is `failed`,\
    \ as is a\n   success where a refusal was expected and the reverse."
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'The runner SHALL settle each vector independently: a `tool-failure`,

    `incomplete` or `failed` vector SHALL change no other vector''s outcome, and

    the runner SHALL continue with the next vector.

    Each vector SHALL start with fresh resource accounting at its effective

    limits; a previous vector''s consumption cannot reduce its remaining budget.'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'The runner SHALL give the same outcome for each vector on every run over

    the same corpus with the same QSL build and limits. It reads no clock,

    environment variable or search path to decide an outcome.'
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: "- QSpec FR-311 (qualification behaviour), FR-336 and interface_019 (the\n\
    \  corpus and its vectors).\n- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)\n\
    \  (stage limits and catalog-coded refusals the comparison reads).\n- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md)\n\
    \  settles each capability from these outcomes."
- id: FR-351
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: 'When a conformance run completes, the runner SHALL settle one outcome
    for

    each capability in scope from the outcomes of that capability''s vectors

    executed in the same run (FR-350).'
- id: FR-351
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: "- The per-vector outcomes of FR-350.\n- The scope: the capability ids\
    \ the caller selects, or, when the caller\n  selects none, every capability the\
    \ corpus lists whose applicability names\n  a QSL consumer type and a semantic\
    \ profile in QSL's definition catalog\n  ([FR-350](FR-350-run-the-conformance-corpus-against-qsl.md))."
- id: FR-351
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: 'One outcome for each capability in scope: `passed`, `failed`,

    `tool-failure`, `unsupported`, `incomplete` or `uncovered`, holding the ids

    of the vectors that decided it.'
- id: FR-351
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: "- If a capability in scope has no vector whose applicability names a QSL\n\
    \  consumer type,\n  then its outcome SHALL be `uncovered`.\n- Otherwise its outcome\
    \ SHALL be the first of these that holds:\n  1. `failed`, when any of its vectors\
    \ is `failed`;\n  2. `tool-failure`, when any is `tool-failure`;\n  3. `unsupported`,\
    \ when any is `unsupported`;\n  4. `incomplete`, when any is `incomplete`;\n \
    \ 5. `passed`, when every one is `passed`.\n- A non-`passed` capability SHALL\
    \ hold the ids of every vector with that\n  outcome.\n- Each capability outcome\
    \ is computed from vectors executed in this run.\n  "
- id: FR-351
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: "The corpus's recorded `results` member (QSpec FR-336) is not an input\
    \ to\n  any outcome.\n- A caller-selected scope SHALL be the set of the named\
    \ capability ids:\n  repeating an id cannot execute its vectors twice or count\
    \ the capability\n  twice. Selection does not grant a new consumer type to its\
    \ vectors. An\n  explicitly selected capability with no QSL-applicable vectors\
    \ is\n  `uncovered`, even when its monitor-only vectors have recorded passes.\n\
    - If the caller selects a capability id the corpus does not list, then the\n \
    \ runner SHALL refuse the run with `unknown_required_feature`/\n  `un"
- id: FR-351
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: known-feature`, naming the id, before it executes any vector.
- id: FR-351
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: "- [FR-350](FR-350-run-the-conformance-corpus-against-qsl.md) (per-vector\n\
    \  outcomes).\n- QSpec FR-311 (a missing, unsupported, failed or unexecuted vector\
    \ is a\n  gap) and FR-336 (capability ids, applicability, the `results` member)."
- id: FR-352
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: 'When a conformance run completes, the runner SHALL write a report that

    gives the capability coverage of the run, the outcome of each capability in

    scope, and each non-passing vector with its expected and actual result.'
- id: FR-352
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: 'Capability coverage is measured the way test coverage is: the share of

    in-scope capabilities whose vectors all passed.'
- id: FR-352
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: The capability outcomes of FR-351 and the vector outcomes of FR-350.
- id: FR-352
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: 'A report, written to standard output as JSON, or as text when the caller

    asks for text. It holds:'
- id: FR-352
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: "- the QSL version that ran;\n- the scope;\n- the count of capabilities\
    \ with each outcome, and the coverage, which is\n  the `passed` count divided\
    \ by the in-scope count;\n- each capability's outcome and its deciding vector\
    \ ids, and, for each\n  non-passing capability, every non-passing vector with\
    \ its own outcome,\n  expected result and actual result. A vector with a lower\
    \ settlement\n  precedence remains visible even when it did not decide the capability."
- id: FR-352
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: "- The report SHALL list capabilities in ascending capability-id order\
    \ and,\n  inside each capability, vectors in ascending vector-id order.\n- The\
    \ report SHALL be equal across two runs over the same corpus with the\n  same\
    \ build, limits and scope.\n- An `uncovered` capability SHALL count in the in-scope\
    \ total and not in\n  the `passed` count."
- id: FR-352
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: "- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md)\n\
    \  (capability outcomes).\n- QSpec FR-311 (per-capability results and coverage\
    \ report)."
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: 'When the `qualify` repository gate finishes a conformance run, the gate
    SHALL

    report QSL qualified for the run''s scope if and only if every capability in

    that scope is `passed` (FR-351).'
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: 'Complete-V1 qualified is that verdict over the default scope defined by

    [FR-350](FR-350-run-the-conformance-corpus-against-qsl.md): every capability

    the corpus lists for a QSL consumer type and a semantic profile QSL''s

    definition catalog holds.'
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: The capability outcomes of FR-351 and the scope.
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: 'A verdict in the FR-352 report: `qualified` or `not-qualified`, with the

    scope it covers, and the gate''s exit status.'
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: '`qualify` is an xtask repository gate, run beside the repository''s other

    xtask gates:'
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: '`--corpus` names the QSpec conformance corpus, read in place (FR-350).

    Each `--capability` adds one capability id to a caller-selected scope;

    with none, the scope is the default scope. Each `--limit` raises or lowers

    one FR-255 setting. `--text` writes the FR-352 report as text instead of

    JSON.'
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: 'Before any vector executes, the gate SHALL validate all arguments, resolve

    the complete `--limit` operand list through the

    [FR-255 settings operation](FR-255-name-the-setting-that-raises-a-reached-limit.md),

    validate the corpus contract and resolve the selected capability ids.

    An unknown setting, a non-integer or negative bound, or a repeated setting

    is a usage refusal naming the offending operand. Omitted settings retain

    FR-255''s defaults. A missing file named by an otherwise valid vector is

    the per-vector `tool-failure` of FR-350, not a corpus preflight refusal.'
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: "- If every in-scope capability is `passed`, then the gate SHALL report\n\
    \  `qualified` and exit 0.\n- If any in-scope capability is not `passed`, then\
    \ the gate SHALL report\n  `not-qualified`, naming every such capability with\
    \ its outcome, and SHALL\n  exit 1.\n- If the corpus location cannot be read,\
    \ if the corpus fails QSpec FR-336's\n  validation (a `contract_version` other\
    \ than `quire.conformance-corpus/v1`,\n  a missing required property, a duplicate\
    \ key, an out-of-domain value or a\n  non-canonical encoding), if a `--capability`\
    \ id names no capability the\n  corpus lists (FR-351), or if "
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: "an argument is malformed, then the gate SHALL\n  report the refusal, naming\
    \ its cause and locus, give no verdict and exit 2.\n- A verdict for a caller-selected\
    \ scope SHALL name that scope. Only a run\n  over the default scope SHALL report\
    \ `complete-v1: qualified`.\n- A non-passing capability outside the scope SHALL\
    \ change no verdict.\n- A completed run containing only `tool-failure` vector\
    \ outcomes SHALL\n  still report `not-qualified` and exit 1. Exit 2 does not replace\
    \ a\n  completed run's verdict because the per-vector failure was an I/O error,\n\
    \  panic or internal fault."
- id: FR-353
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: "- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md),\n\
    \  [FR-352](FR-352-report-capability-coverage.md).\n- QSpec FR-311 (a gap prevents\
    \ only its containing scope from claiming\n  complete success, FR-311-AC-2)."
- id: TC-876
  path: spec/test-cases/TC-876-the-runner-separates-unsupported-incomplete-and-tool-failure.md
  role: examined
  excerpt: Verify the `unsupported`, `incomplete` and `tool-failure` outcomes, that
    a failing vector leaves its siblings alone, and that two runs agree.
- id: TC-876
  path: spec/test-cases/TC-876-the-runner-separates-unsupported-incomplete-and-tool-failure.md
  role: examined
  excerpt: 'Scope: FR-350-AC-3, FR-350-AC-4, FR-350-AC-5.'
- id: TC-876
  path: spec/test-cases/TC-876-the-runner-separates-unsupported-incomplete-and-tool-failure.md
  role: examined
  excerpt: '1. Add a vector that expects a success and whose unit requires a capability
    id QSL does not know, so QSL refuses `unknown_required_feature`/`unknown-feature`;
    add the same vector expecting that pair; add it again expecting `ill_typed`/`type-mismatch`.

    2. Run TC-875''s positive `run` vector, which expects a success, with an evaluation
    budget of zero, then with the default budget. Add a boundary vector that expects
    the zero-budget limit refusal and run it with a budget of zero.

    3. Add a vector naming a source file absent from the corpus, between two passing
    vectors; run; remove it and r'
- id: TC-876
  path: spec/test-cases/TC-876-the-runner-separates-unsupported-incomplete-and-tool-failure.md
  role: examined
  excerpt: "un again.\n4. Run the TC-875 corpus twice with the same build and limits.\n\
    5. Use two distinct vector ids over identical successful run inputs. Give\n  \
    \ each exactly the budget its independently derived expected accounting\n   requires.\
    \ Run in both vector orders. Compare the original corpus and\n   named input bytes\
    \ directly before and after all runs, including the\n   corpus's recorded `results`\
    \ and `coverage`.\n6. Run TC-875's positive compile vector with `--limit s3.nodes=0`,\
    \ then\n   with that setting raised to fit. Compare each actual typed result to\
    \ a\n   direct public compile call unde"
- id: TC-876
  path: spec/test-cases/TC-876-the-runner-separates-unsupported-incomplete-and-tool-failure.md
  role: examined
  excerpt: "r limits from the same FR-255 settings\n   operation."
- id: TC-876
  path: spec/test-cases/TC-876-the-runner-separates-unsupported-incomplete-and-tool-failure.md
  role: examined
  excerpt: Tag the tests `#[trace("TC-876", "FR-350-AC-n")]`.
- id: TC-876
  path: spec/test-cases/TC-876-the-runner-separates-unsupported-incomplete-and-tool-failure.md
  role: examined
  excerpt: "- Step 1: `unsupported` holding `unknown_required_feature`/`unknown-feature`;\
    \ then `passed`; then `unsupported`.\n- Step 2: `incomplete` retaining the evaluation\
    \ budget kind, bound 0,\n  actual denied count and the subject's setting or request\
    \ field that\n  raises it; then `passed`; the boundary vector is `passed`.\n-\
    \ Step 3: `tool-failure` holding the I/O error; the two neighbouring vectors are\
    \ `passed` in both runs.\n- Step 4: equal per-vector outcomes.\n- Step 5: both\
    \ vectors pass in both orders with independent accounting;\n  all compared corpus\
    \ and input bytes are unchanged.\n- Step 6: "
- id: TC-876
  path: spec/test-cases/TC-876-the-runner-separates-unsupported-incomplete-and-tool-failure.md
  role: examined
  excerpt: "`incomplete` naming `s3.nodes`, then `passed`; actual results\n  equal\
    \ the corresponding direct public calls."
- id: TC-877
  path: spec/test-cases/TC-877-capability-outcomes-follow-the-vector-precedence.md
  role: examined
  excerpt: Verify the per-capability settlement order and the `uncovered` outcome.
- id: TC-877
  path: spec/test-cases/TC-877-capability-outcomes-follow-the-vector-precedence.md
  role: examined
  excerpt: 'Scope: FR-351-AC-1, FR-351-AC-2.'
- id: TC-877
  path: spec/test-cases/TC-877-capability-outcomes-follow-the-vector-precedence.md
  role: examined
  excerpt: 'Build a fixture corpus with capabilities `CAP-A` (three passing vectors),
    `CAP-B` (one passing, one failing), `CAP-C` (one `unsupported`, one `incomplete`
    under a zero budget), `CAP-D` (one `tool-failure`, one failing), `CAP-E` (applicability:
    language, no vector) and `CAP-F` (applicability: a monitor subject only, one vector).
    Run over the default scope.'
- id: TC-877
  path: spec/test-cases/TC-877-capability-outcomes-follow-the-vector-precedence.md
  role: examined
  excerpt: 'Give `CAP-A` through `CAP-E` a profile held by QSL''s definition catalog.

    Add `CAP-G`, whose capability applicability names `language` but only a

    profile absent from that catalog. Include a vector for `CAP-G` whose source

    does not exist, so accidental execution is observable as `tool-failure`.'
- id: TC-877
  path: spec/test-cases/TC-877-capability-outcomes-follow-the-vector-precedence.md
  role: examined
  excerpt: Tag the tests `#[trace("TC-877", "FR-351-AC-n")]`.
- id: TC-877
  path: spec/test-cases/TC-877-capability-outcomes-follow-the-vector-precedence.md
  role: examined
  excerpt: '- `CAP-A` `passed`; `CAP-B` `failed` holding the failing vector''s id;
    `CAP-C` `unsupported`; `CAP-D` `failed`; `CAP-E` `uncovered`.

    - `CAP-F` appears in no outcome and its vector does not run.

    - `CAP-G` appears in no outcome and its vector does not run.'
- id: TC-878
  path: spec/test-cases/TC-878-capability-outcomes-come-from-executed-vectors-only.md
  role: examined
  excerpt: Verify that a corpus's recorded results do not decide an outcome, and that
    a caller scope selects exactly its capabilities or refuses an unknown id.
- id: TC-878
  path: spec/test-cases/TC-878-capability-outcomes-come-from-executed-vectors-only.md
  role: examined
  excerpt: 'Scope: FR-351-AC-3, FR-351-AC-4.'
- id: TC-878
  path: spec/test-cases/TC-878-capability-outcomes-come-from-executed-vectors-only.md
  role: examined
  excerpt: "1. Take TC-877's corpus and set its `results` member to record `passed`\
    \ for `CAP-B`'s failing vector; run.\n2. Run TC-877's corpus scoped to `CAP-A`\
    \ and `CAP-E`.\n3. Run it scoped to `CAP-Z`, which the corpus does not list.\n\
    4. Repeat step 2 with `CAP-A` supplied twice; observe vector invocation\n   counts\
    \ through the runner's subject boundary as well as the report.\n5. Explicitly\
    \ select only the monitor-only `CAP-F`, with recorded passing\n   results for\
    \ its vectors."
- id: TC-878
  path: spec/test-cases/TC-878-capability-outcomes-come-from-executed-vectors-only.md
  role: examined
  excerpt: Tag the tests `#[trace("TC-878", "FR-351-AC-n")]`.
- id: TC-878
  path: spec/test-cases/TC-878-capability-outcomes-come-from-executed-vectors-only.md
  role: examined
  excerpt: '- Step 1: `CAP-B` `failed`.

    - Step 2: exactly `CAP-A` `passed` and `CAP-E` `uncovered`.

    - Step 2 executes no vector of `CAP-B`, `CAP-C`, `CAP-D`, `CAP-F` or `CAP-G`.

    - Step 3: the run refuses `unknown_required_feature`/`unknown-feature` naming
    `CAP-Z`, and no vector runs.

    - Step 4: the same vector invocation counts and outcomes as step 2.

    - Step 5: `CAP-F` is `uncovered`; none of its vectors executes.'
- id: TC-879
  path: spec/test-cases/TC-879-the-report-gives-counts-coverage-and-failing-vectors.md
  role: examined
  excerpt: Verify the report's counts, coverage, failure detail, order, determinism
    and version line.
- id: TC-879
  path: spec/test-cases/TC-879-the-report-gives-counts-coverage-and-failing-vectors.md
  role: examined
  excerpt: 'Scope: FR-352-AC-1, FR-352-AC-2, FR-352-AC-3.'
- id: TC-879
  path: spec/test-cases/TC-879-the-report-gives-counts-coverage-and-failing-vectors.md
  role: examined
  excerpt: 'Build a fixture corpus with four in-scope capabilities: two whose vectors
    pass, one with a failing vector, one with no vector. Write the capability ids
    out of order in the corpus. Run twice with JSON output and once with text output.'
- id: TC-879
  path: spec/test-cases/TC-879-the-report-gives-counts-coverage-and-failing-vectors.md
  role: examined
  excerpt: 'Then add three vectors to the failing capability: one with a missing input

    file (`tool-failure`), one expecting success but returning an unknown-feature

    refusal (`unsupported`), and one expecting success but exhausting its

    configured accounting budget (`incomplete`). Give the four non-passing

    vectors ids out of order and repeat both output modes.'
- id: TC-879
  path: spec/test-cases/TC-879-the-report-gives-counts-coverage-and-failing-vectors.md
  role: examined
  excerpt: Tag the tests `#[trace("TC-879", "FR-352-AC-n")]`.
- id: TC-879
  path: spec/test-cases/TC-879-the-report-gives-counts-coverage-and-failing-vectors.md
  role: examined
  excerpt: "- Counts 2, 1, 0, 0, 0, 1 for `passed`, `failed`, `tool-failure`, `unsupported`,\
    \ `incomplete`, `uncovered`; coverage 50 per cent.\n- The failing capability's\
    \ entry holds the failing vector's id with expected and actual result; passing\
    \ entries hold no vector detail.\n- The two JSON reports are byte-equal, list\
    \ capabilities and vectors in ascending id order and name the QSL version that\
    \ ran. The text report gives the same counts and coverage.\n- After adding the\
    \ three vectors, capability counts and coverage stay the\n  same. The failed capability\
    \ keeps the original failed vector as its onl"
- id: TC-879
  path: spec/test-cases/TC-879-the-report-gives-counts-coverage-and-failing-vectors.md
  role: examined
  excerpt: "y\n  deciding id. Both output modes retain all four non-passing vectors\
    \ in id\n  order, each with its own outcome and expected and actual result."
- id: TC-880
  path: spec/test-cases/TC-880-the-verdict-is-qualified-only-when-every-capability-passes.md
  role: examined
  excerpt: Verify the qualified verdict, its exit status, and that one failing or
    uncovered capability makes the run not qualified.
- id: TC-880
  path: spec/test-cases/TC-880-the-verdict-is-qualified-only-when-every-capability-passes.md
  role: examined
  excerpt: 'Scope: FR-353-AC-1, FR-353-AC-2.'
- id: TC-880
  path: spec/test-cases/TC-880-the-verdict-is-qualified-only-when-every-capability-passes.md
  role: examined
  excerpt: Every step runs the gate as `cargo run --package xtask -- qualify --corpus
    <dir>` over a fixture corpus in the test's temporary directory, and reads its
    report and exit status.
- id: TC-880
  path: spec/test-cases/TC-880-the-verdict-is-qualified-only-when-every-capability-passes.md
  role: examined
  excerpt: "1. Run a fixture corpus whose in-scope vectors all pass, over the default\
    \ scope.\n2. Change one refusal vector's expected cause and run.\n3. Restore it,\
    \ remove every vector of one capability and run.\n4. Run separate one-capability\
    \ corpora whose sole vector settles\n   `unsupported`, `incomplete`, and `tool-failure`,\
    \ respectively, using\n   TC-876's inputs. The last corpus is valid and readable,\
    \ but its vector\n   names a missing source file."
- id: TC-880
  path: spec/test-cases/TC-880-the-verdict-is-qualified-only-when-every-capability-passes.md
  role: examined
  excerpt: Tag the tests `#[trace("TC-880", "FR-353-AC-n")]`.
- id: TC-880
  path: spec/test-cases/TC-880-the-verdict-is-qualified-only-when-every-capability-passes.md
  role: examined
  excerpt: "- Step 1: `complete-v1: qualified`, exit 0.\n- Step 2: `not-qualified`\
    \ naming that capability `failed`, exit 1.\n- Step 3: `not-qualified` naming that\
    \ capability `uncovered`, exit 1.\n- Step 4: each run reports `not-qualified`,\
    \ names the capability's actual\n  outcome and exits 1. The missing source does\
    \ not cause exit 2."
- id: TC-881
  path: spec/test-cases/TC-881-a-scoped-verdict-names-its-scope.md
  role: examined
  excerpt: Verify that a verdict over a caller scope names that scope and is not a
    complete-V1 verdict, and that each run the `qualify` gate refuses before any vector
    exits 2 with no verdict.
- id: TC-881
  path: spec/test-cases/TC-881-a-scoped-verdict-names-its-scope.md
  role: examined
  excerpt: 'Scope: FR-353-AC-3, FR-353-AC-4.'
- id: TC-881
  path: spec/test-cases/TC-881-a-scoped-verdict-names-its-scope.md
  role: examined
  excerpt: Every step runs `cargo run --package xtask -- qualify --corpus <dir>`,
    adding the `--capability` arguments the step names.
- id: TC-881
  path: spec/test-cases/TC-881-a-scoped-verdict-names-its-scope.md
  role: examined
  excerpt: "1. Over TC-880 step 2's corpus, run with one `--capability` per capability\
    \ except the failing one.\n2. Run over a corpus location that does not exist.\n\
    3. Run over TC-880 step 1's corpus with `contract_version` set to `quire.conformance-corpus/v0`;\
    \ then with one vector's expected disposition removed.\n4. Run over TC-880 step\
    \ 1's corpus with `--capability CAP-Z`, an id the corpus does not list.\n5. Make\
    \ three independent corpus mutations: duplicate a required key,\n   replace a\
    \ closed-domain value with a value outside that domain, and\n   encode a valid\
    \ document non-canonically under QSpec "
- id: TC-881
  path: spec/test-cases/TC-881-a-scoped-verdict-names-its-scope.md
  role: examined
  excerpt: "FR-336. Run each.\n6. Over the valid corpus, run separately with `--limit\
    \ s9.nodes=1`,\n   `--limit s3.nodes=ten`, `--limit s3.nodes=-1`, and two occurrences\
    \ of\n   `--limit s3.nodes=5`. Observe the subject invocation boundary in every\n\
    \   refusal case, including a bad operand after a valid operand."
- id: TC-881
  path: spec/test-cases/TC-881-a-scoped-verdict-names-its-scope.md
  role: examined
  excerpt: Tag the tests `#[trace("TC-881", "FR-353-AC-n")]`.
- id: TC-881
  path: spec/test-cases/TC-881-a-scoped-verdict-names-its-scope.md
  role: examined
  excerpt: "- Step 1: `qualified` for the named scope, no `complete-v1: qualified`\
    \ line, exit 0.\n- Step 2: the I/O refusal, no verdict, exit 2.\n- Step 3: a refusal\
    \ naming `/contract_version`, then one naming the missing member; no verdict,\
    \ no vector executed, exit 2 each.\n- Step 4: `unknown_required_feature`/`unknown-feature`\
    \ naming `CAP-Z`, no verdict, no vector executed, exit 2.\n- Step 5: each corpus\
    \ validation refusal names its cause and locus; no\n  verdict, no vector executed,\
    \ exit 2.\n- Step 6: usage refusal naming the offending operand; no verdict, no\n\
    \  vector executed, exit 2. A valid earl"
- id: TC-881
  path: spec/test-cases/TC-881-a-scoped-verdict-names-its-scope.md
  role: examined
  excerpt: ier operand starts no stage.
- id: FR-350-AC-1
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: Over a fixture corpus holding one positive compile vector, one positive
    `run` vector, one `replay` vector and one `format` vector, each with the expected
    result QSL returns, each vector is `passed` and each ran through its subject's
    public entry point.
- id: FR-350-AC-2
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: A refusal vector expecting `ill_typed`/`unit-mismatch` is `passed` when
    QSL returns that pair, and `failed` when QSL returns `ill_typed`/`type-mismatch`,
    the failure holding the expected and the actual code and cause. A refusal vector
    over source QSL admits is `failed`, holding the expected refusal and the actual
    success.
- id: FR-350-AC-3
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: A vector that expects a success, which QSL answers with `unknown_required_feature`/`unknown-feature`,
    is `unsupported`, holding that pair; the same answer to a vector that expects
    that pair is `passed`. A vector that expects a success, run with an evaluation
    budget of zero, is `incomplete`, retaining its budget kind, bound, actual count
    and the subject's setting or request field that raises it, and with the default
    budget is `passed`. A boundary vector that expects the zero-budget limit refusal,
    run with a budget of zero, is `passed`. A vector expecting `ill_typed`/`type-mismatch`
    th
- id: FR-350-AC-3
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: at QSL answers with `unknown_required_feature` is `unsupported`. A positive
    compile vector checked under `--limit s3.nodes=0` is `incomplete` naming `s3.nodes`;
    raising that setting to fit yields `passed`, with actual typed results equal to
    direct public calls under the same effective limits.
- id: FR-350-AC-4
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: A vector naming a source file absent from the corpus is `tool-failure`
    holding the I/O error, and the vectors before and after it in the corpus keep
    the outcomes they have when it is removed.
- id: FR-350-AC-5
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: Two runs over the same fixture corpus with the same build and limits give
    equal per-vector outcomes. Two identical successful vectors, each requiring the
    full configured budget, both pass in either corpus order. The corpus and its named
    input bytes are unchanged after execution, including its recorded results and
    coverage.
- id: FR-351-AC-1
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: A capability whose three vectors are `passed` is `passed`. A capability
    with one `passed` and one `failed` vector is `failed`, holding the failed vector's
    id. A capability with one `unsupported` and one `incomplete` vector is `unsupported`.
    A capability with one `tool-failure` and one `failed` vector is `failed`.
- id: FR-351-AC-2
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: A capability whose applicability names the `language` consumer type and
    a catalog-held profile, with no vector, is `uncovered`. A capability whose applicability
    names only the `monitor` consumer type, and a language capability whose applicability
    names only a profile absent from QSL's catalog, are outside the default scope
    and appear in no outcome.
- id: FR-351-AC-3
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: A corpus whose `results` member records `passed` for a vector QSL fails
    settles that vector's capability `failed`.
- id: FR-351-AC-4
  path: spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md
  role: examined
  excerpt: A run scoped to two capability ids settles exactly those two and executes
    no vector from another capability. Repeating one selected id gives the same vector
    executions and capability outcomes as naming it once. Explicitly selecting a monitor-only
    capability settles it `uncovered` without running its vectors. A run scoped to
    an id the corpus does not list refuses `unknown_required_feature`/`unknown-feature`
    naming the id and executes no vector.
- id: FR-352-AC-1
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: Over a fixture corpus with four in-scope capabilities, two `passed`, one
    `failed` and one `uncovered`, the report gives counts 2, 1, 0, 0, 0, 1 for `passed`,
    `failed`, `tool-failure`, `unsupported`, `incomplete` and `uncovered`, and coverage
    50 per cent.
- id: FR-352-AC-2
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: The `failed` capability's entry holds its failing vector's id with the
    expected and the actual result; the `passed` capabilities' entries hold no vector
    detail. Adding a `tool-failure`, `unsupported` and `incomplete` vector to the
    failed capability leaves it `failed` with the same deciding vector id, while the
    report retains all four non-passing vectors with their own outcomes and expected
    and actual results.
- id: FR-352-AC-3
  path: spec/functional/FR-352-report-capability-coverage.md
  role: examined
  excerpt: The JSON report of two runs over the fixture corpus is byte-equal, capabilities
    and vectors are in ascending id order, and the report names the QSL version that
    ran. The text report holds the same counts and coverage.
- id: FR-353-AC-1
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: 'A fixture corpus whose in-scope vectors all pass, run over the default
    scope, reports `complete-v1: qualified` and exits 0.'
- id: FR-353-AC-2
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: The same corpus with one vector's expected cause changed reports `not-qualified`,
    naming that vector's capability as `failed`, and exits 1; the same corpus with
    one capability's vectors removed reports `not-qualified`, naming it `uncovered`.
    Separate one-capability runs settling `unsupported`, `incomplete` and `tool-failure`
    each report `not-qualified` and exit 1; a missing vector input is the latter case,
    while a missing corpus is AC-4's exit 2.
- id: FR-353-AC-3
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: 'A run scoped to the capabilities that pass, over the corpus of AC-2, reports
    `qualified` for that named scope, does not report `complete-v1: qualified`, and
    exits 0.'
- id: FR-353-AC-4
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: A run over a corpus location that does not exist reports the I/O refusal,
    gives no verdict and exits 2; so does a run over a corpus whose `contract_version`
    is `quire.conformance-corpus/v0`, naming `/contract_version`, a corpus with one
    vector missing its expected disposition, naming that member, and a run with `--capability
    CAP-Z` that the corpus does not list, naming `CAP-Z`. Duplicate keys, out-of-domain
    values and non-canonical encodings each refuse before execution. Runs with `--limit
    s9.nodes=1`, `--limit s3.nodes=ten`, `--limit s3.nodes=-1`, or two `--limit s3.nodes=5`
    operand
- id: FR-353-AC-4
  path: spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md
  role: examined
  excerpt: s each refuse naming the offending operand. No vector executes and no verdict
    is emitted in any of them.
- id: FR-255
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: context_only
  excerpt: 'A limit outcome SHALL tell the caller everything needed to run the input

    again under a larger limit (ADR-030 D-3). Every configurable resource limit

    of QSL''s front end and engines has one stable setting name. The same name

    raises the limit from the library, from a replay request (every setting but

    `replay.input_bytes`) and from the driver CLI (`quire`, ADR-029 CB-1), which

    exposes the caller limits through QSL''s settings operation, wherever that

    entry point takes the setting. Every `stage_limit_exceeded` outcome names the

    limit, its value and its setting. An execution budget set in a'
- id: US-035
  path: spec/usecase/US-035-qualify-qsl-against-the-complete-v1-conformance-corpus.md
  role: context_only
  excerpt: '**As a** QSL maintainer, or a team that adopts QSL as its complete-V1

    compiler and reference runtime

    **I want** one repository gate that runs the QSpec complete-V1 conformance corpus

    against QSL and reports, for each capability, whether QSL passed its

    vectors, with a coverage figure and a qualified or not-qualified verdict

    **So that** "QSL is complete V1" is a statement the product measures on

    every run, and each capability that is missing, failing or unsupported is

    named with the vectors that show it.'
- id: TC-875
  path: spec/test-cases/TC-875-the-runner-executes-and-compares-vectors-through-public-entry-points.md
  role: context_only
  excerpt: Verify that the runner runs each vector through its subject's public entry
    point and compares the result with the expected disposition, including the code
    and cause of a refusal.
```

## Merge boundary

Frozen review: 725ffd66691c962e1bdcf6260a31d03430cfa5ed. Recommend resolving SR-4954 FND-001 before merge. GitHub initially reported MERGEABLE/BLOCKED and subsequently UNKNOWN/UNKNOWN while retaining the same head. This review does not grant an implementation PASS or close QSL-603. PR #665 remains open. Review comments were posted to the private Linear ticket, read back byte-for-byte and checked for YAML, finding ID and required-field correctness; has-review was newly added. No PR files were edited.
