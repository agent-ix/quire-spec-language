---
id: TC-084
title: "Select package versions and features"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
---
# TC-084: Select package versions and features

## Description

Property, priority P1. Verifies FR-020-AC-3, FR-020-AC-4. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Mutate each format/language/edition/profile/definition selector independently and recompute the raw byte selector. Permute unique features, introduce decoded duplicates/unknown features and remove each actually required feature from consumer support. Add unknown consumer support entries to ensure they cannot enable an unknown format or feature.

Combine unknown format with an otherwise valid JSON object that lacks version-1
fields or contains different fields. Combine that unknown format separately with
duplicate decoded keys, malformed JSON, an exhausted recognition budget and a
wrong raw digest. For version 1 combine two unsupported semantic selectors and
permute their member order. Include a recognition-only number beyond binary64's
range to establish that header selection does not interpret unknown payloads.

## Expected Results

Version/profile failures retain their selected codes with no decoder fallback. Reordered unique features are accepted; duplicate features are invalid_package, and unknown or unavailable required features are unknown_required_feature.

Unknown format wins over version-1 shape defects; earlier raw admission/digest
or JSON recognition failures win over format selection. Known-format closed
decoding precedes semantic selection, whose first failing selector follows the
documented fixed order independently of member order. A number outside Serde's
generic finite numeric recognition domain receives invalid_package before
format selection; exact u64 values above binary64 precision remain admitted
and retain their original bytes for the later typed pass.
