---
id: FR-077
title: "Remove backend negotiation from the composed linker"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-011
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: traces_to
---
# FR-077: Remove backend negotiation from the composed linker

## Description

The composed linker performed no backend negotiation in its request-report path: it recorded requested capability pairs as data only, took no `Backend` parameter and produced no `UnsupportedCapability` or `UnsupportedFamily` disposition. This is ADR-010 OBS-003 as resolved by ADR-012 §10: candidates and routing live only in the registry (FR-075), and dispositions live only in `quire-contract-codegen`'s `negotiate_*`.

**QSL-499 disposition:** the composed linker and its `requests::report` function are deleted under ADR-011, ADR-033 and FR-499, so the criteria below are retained as migration history and are not active acceptance obligations. The behavior is owned by FR-057: its admission entry point takes no registry or backend parameter, admitted pairs and static meaning do not depend on registered backends, a pair keeps its declaration, kind, `required` flag and request index, and `absent-kind` and `unknown-kind` remain admission-time refusals (FR-057-AC-4, AC-5).

## Behavior

The behavior, now owned by FR-057's admission, is that requested pairs are recorded as data (declaration, capability kind, `required` flag, request index) with no backend parameter, no backend-dependent per-pair disposition and no compatibility path for the pre-canonical four-kind label enum; `absent-kind` and `unknown-kind` remain admission-time refusals produced without reading backend state. The criteria below are historical. FR-057-AC-4 and AC-5 are the owning requirement; the unknown-kind half of FR-057-AC-5 has no tagged test yet, and that coverage gap is not claimed as covered.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-077-AC-1 | **RETIRED in QSL-499; historical only.** `requests::report`'s signature contains no parameter of a `Backend`-shaped type; a call site supplying no backend argument compiles and produces a complete request report. | Test (TC-199) |
| FR-077-AC-2 | **RETIRED in QSL-499; historical only.** The disposition type returned by, or reachable from, `requests::report` contains no `UnsupportedCapability` variant and no `UnsupportedFamily` variant. | Test (TC-199) |
| FR-077-AC-3 | **RETIRED in QSL-499; historical only.** Given a set of at least four admitted requested pairs spanning at least three distinct capability kinds and both `required` values, `requests::report` (with no negotiation code path present at all, per FR-077-AC-1 and FR-077-AC-2) records every pair's declaration, capability kind, `required` flag and request index in its output, unchanged from its pre-removal input, showing recording does not depend on the negotiation logic this requirement removes. | Test (TC-200) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §10
  (ADR-010 OBS-003) is the normative decision this requirement implements:
  "The composed linker performs no backend negotiation. `requests::report`
  records requests as data and has no `Backend` parameter, no
  `UnsupportedCapability` disposition and no `UnsupportedFamily`
  disposition."
- [FR-036](FR-036-link-composed-native-packages.md) owns the request report
  this requirement amends; ADR-012 §10 (OBS-003) records that FR-036's line
  97, AC-5 and AC-6, and TC-115, change with this removal.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md) is where
  candidates and routing now live, replacing the linker's prior direct read
  of backend support.
