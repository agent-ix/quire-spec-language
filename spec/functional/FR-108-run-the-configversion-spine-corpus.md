---
id: FR-108
title: "Run the ConfigVersion corpus through the spine with typed dispositions"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-032
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-180
    type: depends_on
---
# FR-108: Run the ConfigVersion corpus through the spine with typed dispositions

## Description

The ConfigVersion corpus SHALL run through the spine (FR-109) from a
`1-draft` unit, a Semantic IR 2.0.0 domain package and FR-106 observation
documents, and each case SHALL give its independently specified typed
disposition. The report SHALL retain stage, category, truth and code across
FR-267's native-run-result/2 writer and reader (QSpec FR-352). This is QSL's
share of QSpec FR-180-AC-1, AC-4 and AC-5 for ConfigVersion (QSpec TC-209's
ownership table: QSL #121).

The corpus is FR-032's 13 original cases in
`examples/config-version/cases.rs`, extended with the four domain boundaries
QSpec FR-180-AC-4 names (0, 1000, -1, 1001). The spine test owns the
independent `expected()` oracle.

## Inputs

- The case catalog `examples/config-version/cases.rs`, extended by the four
  boundary cases below. It stays the one catalog both paths generate from.
- `examples/config-version/model.semantic-ir.json`: a newly authored,
  AGPL-3.0-only Semantic IR 2.0.0 domain package `example/config-version`
  version `1.0.0`, the spine counterpart of `model.json`, with the declarations
  FR-103-AC-1 lists. It and the generated unit carry `AGPL-3.0-only`, not the
  repository's `AGPL-3.0-or-later`, because they are example content beside
  `model.json` and the generated `program.native`, which FR-032 authors as
  `AGPL-3.0-only`; one example keeps one licence.
- One `1-draft` unit, generated with the fixtures. Its header selects the
  `root` profile FR-110 resolves, `quire.value.complete/v1`, by identity,
  and the domain package by its identifier and the canonical content digest
  the supplied document is checked against (refusing
  `stale_dependency`/`content-mismatch`, QSpec FR-272 as amended by
  quire-specification#174). The `model` line is QSpec's shared-grammar
  `model` production, with no `version`:

```text
// SPDX-License-Identifier: AGPL-3.0-only
language "ix:native" edition "1-draft";
profile v = "quire.value.complete/v1";
model Config = "example/config-version" digest "sha256-jcs:<the package's digest>";
invariant ParentOrder using v on Config::ConfigVersion at current { present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }
invariant NoCycle using v on Config::ConfigVersion at current { not reaches(self, self, parent) }
post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate { self.versionNumber = pre(self.versionNumber) }
function sameIdentity using v(a: Config::ConfigVersion, b: Config::ConfigVersion): Boolean pure { a = b }
```

## Outputs

- Per case: FR-106 snapshot and invocation documents and a
  `ClauseRunRequest`, written by the spine corpus generator
  (`examples/config-version/spine_corpus.rs`).
- Per case: one `ClauseRunReport` (FR-109).

## Corpus

The corpus table gives the clause-spine stage and category directly. Work
exhaustion uses FR-100's outcome
`{"kind": "incomplete", "limit": "work_units"}`. Every `Current` case selects
the anchor `handler validate`.

| Case | Spine selection | Disposition (stage, category, truth or code) | Exit |
| --- | --- | --- | --- |
| healthy-parent | `ParentOrder`, self `child` | evaluate, success, true | 0 |
| violating-parent | `ParentOrder`, self `child` | evaluate, violation, false | 10 |
| absent-parent | `ParentOrder`, self `root` | evaluate, success, true | 0 |
| cycle | `NoCycle`, self `child` | evaluate, violation, false | 10 |
| self-loop | `NoCycle`, self `child` | evaluate, violation, false | 10 |
| distinct-identities | `sameIdentity` with `a` `child`, `b` `root` | evaluate, violation, false | 10 |
| dangling-parent | `ParentOrder`, self `child` | admit, refusal, `dangling_reference` | 20 |
| incomplete-population | `ParentOrder`, self `child` | admit, incomplete, `incomplete_population` | 22 |
| missing-model | `ParentOrder`, no package | compile, refusal, `missing_import` | 20 |
| exhausted-work | `ParentOrder`, `work_units` 0 | evaluate, incomplete, `limit: work_units` | 22 |
| unchanged-version | `VersionUnchanged`, self `child` | evaluate, success, true | 0 |
| changed-version | `VersionUnchanged`, self `child` | evaluate, violation, false | 10 |
| forbidden-parent-change | `VersionUnchanged`, self `child` | admit, refusal, `frame_violation` | 20 |
| boundary-zero (new) | `ParentOrder`, root 0, child 1 | evaluate, success, true | 0 |
| boundary-max (new) | `ParentOrder`, root 999, child 1000 | evaluate, success, true | 0 |
| below-range (new) | `ParentOrder`, root -1 | admit, refusal, `invalid_runtime_input` | 20 |
| above-range (new) | `ParentOrder`, child 1001 | admit, refusal, `invalid_runtime_input` | 20 |

The historical native fixture used a model State value `other` for
distinct-identities (`self = other`). `1-draft` has no such value, so the spine
case compares the same two objects through `sameIdentity`, over the same
snapshot data (ADR-012 §15.1).

## Behavior

- The spine corpus generator SHALL write the spine files from the same catalog
  rows as the historical native files, so a case's object keys, field values,
  completeness, frame and limits remain one datum.
- The spine test SHALL check each report against the independent expected
  dispositions above, through an exhaustive `match` over the case enum, as
  so a new catalog case fails to compile until it has an expected spine result.
- For every case, the test SHALL write the report as native-run-result/2 and
  read it through FR-267's strict reader, then compare stage, category, truth
  and code against the typed report.
- The independent expected table SHALL include the four boundary cases:
  boundary-zero and boundary-max success/true, below-range and above-range
  admission refusal with `invalid_runtime_input`.
- With feature `quire-extraction`, each case's unit SHALL also be embedded in
  a Markdown fence and run through the I3 adapter. Its disposition (stage,
  category, truth and exit code) SHALL equal the direct run's, and its
  `package_id` SHALL be the extracted body's own: the one the body compiles
  to under its body identity (FR-109-AC-6), since a source's identity gives
  its declarations their owner and so the `package_id` (FR-001). The two
  runs agree on outcome and keep their distinct source and package
  identities (FR-032-AC-4).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-108-AC-1 | Every one of the 17 cases gives the disposition and exit code in the Corpus table through `run_clause`, checked against the independent expected table. | Test (TC-469) |
| FR-108-AC-2 | For each of the 17 cases, the FR-267 native-run-result/2 writer and strict reader retain the report's stage, category, truth and code. | Test (TC-469) |
| FR-108-AC-3 | Below-range and above-range refuse at admission with `invalid_runtime_input`; their native-run-result/2 records name the object (`root`, `child`) and field `versionNumber`. Boundary-zero and boundary-max complete with `true` (QSpec FR-180-AC-4, QSL's share). | Test (TC-469) |
| FR-108-AC-4 | Generating the corpus twice gives identical files, and running it twice gives identical reports, each holding the disposition, the `package_id` and the usage FR-109 states, so the case is fixed by its inputs (QSpec FR-180-AC-5). A run whose request carries the `package_id` that spine `compile` emits for the unit gives the same report (FR-032-AC-4's package half). | Test (TC-469) |
| FR-108-AC-5 | With `quire-extraction`, the Markdown run of each case, its unit extracted through I3 under its own body identity, gives the direct run's stage, category, truth and exit code, and the `package_id` that a direct `spine::compile` of the extracted body's bytes under that body identity emits. | Test (TC-469) |
| FR-108-AC-6 | Every case's direct-run report agrees on the unit's `package_id` (the one spine `compile` emits for the FR-108 unit and package, independently re-derived and compared against every case's report); the emitted package bytes admit through QSpec I04 `read` (QSpec FR-180's reference verdict contract). | Test (TC-469) |

## Dependencies

- FR-102 to FR-107 and FR-109.
- FR-032 (the native corpus and its catalog), FR-031 (native extracted run).
- QSpec FR-180 and TC-209. QSL's share is the reference dispositions; the
  generated-oracle, property and proof consumers of FR-180-AC-1 are CG's and
  IR's (ADR-012 §15.7).
- STD-111 (QSpec), which FR-105 names, for the emitted package's `state`
  bodies, frame entries and `reaches_field` member; AC-1 to AC-6's
  `package_id` half run over the in-process `CheckedPackage` and need none of
  it. AC-6's I04 `read` half needs `quire-contract-model`'s
  `reaches_field` reference-edge check.
