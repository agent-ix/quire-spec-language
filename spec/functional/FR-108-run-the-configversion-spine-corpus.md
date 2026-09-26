---
id: FR-108
title: "Run the ConfigVersion corpus through the spine with native-equal typed dispositions"
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
# FR-108: Run the ConfigVersion corpus through the spine with native-equal typed dispositions

## Description

The ConfigVersion corpus SHALL run through the spine (FR-109) from a
`1-draft` unit, a Semantic IR 2.0.0 domain package and FR-106 observation
documents, and each case SHALL give the typed disposition native-run/1 gives
today for the same case data. This is QSL's share of QSpec FR-180-AC-1,
AC-4 and AC-5 for ConfigVersion (QSpec TC-209's ownership table: QSL #121),
and it is the end-to-end test ADR-011 §7.3 M-6c needs before native `run` is
deleted (ADR-012 §15.8).

The native corpus is FR-032's: 13 cases in
`examples/config-version/cases.rs`, run by `tests/it/config_version.rs`
through `quire-spec run` against the independent `expected()` oracle. It has
no case at the domain boundaries QSpec FR-180-AC-4 names (0, 1000, -1,
1001).

## Inputs

- The case catalog `examples/config-version/cases.rs`, extended by the four
  boundary cases below. It stays the one catalog both paths generate from.
- `examples/config-version/model.semantic-ir.json`: a newly authored,
  AGPL-3.0-only Semantic IR 2.0.0 domain package `example/config-version`
  version `1`, the spine counterpart of `model.json`, with the declarations
  FR-103-AC-1 lists. It and the generated unit carry `AGPL-3.0-only`, not the
  repository's `AGPL-3.0-or-later`, because they are example content beside
  `model.json` and the generated `program.native`, which FR-032 authors as
  `AGPL-3.0-only`; one example keeps one licence.
- One `1-draft` unit, generated with the fixtures:

```text
// SPDX-License-Identifier: AGPL-3.0-only
language "ix:native" edition "1-draft";
profile v = "quire.value.complete/v1" version "1" digest "sha256:<the value-complete definition digest>";
model Config = "example/config-version" version "1" digest "sha256-jcs:<the package's digest>";
invariant ParentOrder using v on Config::ConfigVersion at current { present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }
invariant NoCycle using v on Config::ConfigVersion at current { not reaches(self, self, parent) }
post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate { self.versionNumber = pre(self.versionNumber) }
function sameIdentity using v(a: Config::ConfigVersion, b: Config::ConfigVersion): Boolean pure { a = b }
```

## Outputs

- Per case: FR-106 snapshot and invocation documents and a
  `ClauseRunRequest`, written beside the native files by the fixtures
  generator.
- Per case: one `ClauseRunReport` (FR-109).

## Corpus

Stage maps native `validate` to spine `admit` and native `link` to spine
`compile`; category maps native `completed` to `success` or `violation` by
truth, and `refused` and `incomplete` to themselves. Native's
`resource_exhausted` diagnostic maps to the spine's FR-100 outcome
`{"kind": "incomplete", "limit": "work_units"}`. Every `Current` case selects
the anchor `handler validate`, native's `point` for these clauses.

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

native-run/1 has a model State value `other` for distinct-identities
(`self = other`). `1-draft` has no such value, so the spine case compares
the same two objects through `sameIdentity`, over the same snapshot data
(ADR-012 §15.1).

## Behavior

- The fixtures generator SHALL write the spine files from the same catalog
  rows as the native files, so a case's object keys, field values,
  completeness, frame and limits are one datum for both paths.
- The spine test SHALL check each report against the independent expected
  dispositions above, through an exhaustive `match` over the case enum, as
  `tests/it/config_version.rs`'s `expected()` does, so a new catalog case
  fails to compile until it has an expected spine result.
- A parity test SHALL run each case through native `quire-spec run` and
  through `run_clause` and compare (stage, category, truth, code, exit
  code) under the map above. It is deleted in the M-6c PR with native `run`;
  the spine test stays.
- The native test's `expected()` SHALL gain the four boundary cases:
  boundary-zero and boundary-max `Completed(true)`, below-range and
  above-range `Validation { code: "invalid_runtime_input", incomplete: false }`.
- With feature `quire-extraction`, each case's unit SHALL also be embedded in
  a Markdown fence and run through the I3 adapter; its disposition SHALL equal
  the direct run's, while its source identity and digest differ and its
  report carries the extraction's original identity (FR-032-AC-4's spine
  half).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-108-AC-1 | Every one of the 17 cases gives the disposition and exit code in the Corpus table through `run_clause`, checked against the independent expected table. | Test (TC-469) |
| FR-108-AC-2 | For each of the 17 cases, native `quire-spec run` and `run_clause` agree on stage (under the map), category, truth, code and exit code. | Test (TC-469) |
| FR-108-AC-3 | below-range and above-range refuse at admission in both paths with `invalid_runtime_input`, and both name the object (`root`, `child`) and the field `versionNumber`; boundary-zero and boundary-max complete with `true` in both (QSpec FR-180-AC-4, QSL's share). | Test (TC-469) |
| FR-108-AC-4 | Generating the corpus twice gives identical files, and running it twice gives identical reports; each report's provenance names the source digest, `package_id`, the domain package's `sha256-jcs` digest, every observation's identity and digest, the selection and the limits, so the case is fixed by its inputs (QSpec FR-180-AC-5). A run whose request carries the `package_id` that spine `compile` emits for the unit gives the same report (FR-032-AC-4's package half). | Test (TC-469) |
| FR-108-AC-5 | With `quire-extraction`, the Markdown run of each case gives the direct run's disposition, a different source identity and digest, and the extraction's original identity and digest in its provenance. | Test (TC-469) |
| FR-108-AC-6 | The expected table pins the unit's `package_id`, the one spine `compile` emits for the FR-108 unit and package, and every case's report carries exactly that value; the emitted package bytes admit through QSpec I04 `read` (QSpec FR-180's reference verdict contract). | Test (TC-469); pending STD-111 |

## Dependencies

- FR-102 to FR-107 and FR-109.
- FR-032 (the native corpus and its catalog), FR-031 (native extracted run).
- QSpec FR-180 and TC-209. QSL's share is the reference dispositions; the
  generated-oracle, property and proof consumers of FR-180-AC-1 are CG's and
  IR's, and IR admits no `state` node at 48ab5dc (ADR-012 §15.7).
- STD-111 (QSpec), which FR-105 names, for the emitted package's `state`
  bodies, frame entries and `reaches_field` member. Only AC-6, the pinned
  `package_id` and its I04 `read`, waits on it; AC-1 to AC-5 run over the
  in-process `CheckedPackage`.

## Status

Specified under QSL-273. AC-6 is pending STD-111: the pinned `package_id` is
fixed once the QSpec spellings of the `state` bodies land, because those
spellings enter every `state` node's id.
