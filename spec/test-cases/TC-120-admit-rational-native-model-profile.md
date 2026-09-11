---
id: TC-120
title: "Distinguish explicit rational native models from historical models"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: verifies
---
# TC-120: Distinguish explicit rational native models from historical models

## Description

Public Rust controls for
[FR-041](../functional/FR-041-admit-rational-native-model-profile.md), using the
existing source-aware JSON frontend, real IR constructors, native admission and
historical/composed linking boundaries. Fixed original JSON and typed IR/role
inputs provide independent expected profiles, numeric components and loci.
The numbered groups correspond to the requirement's acceptance criteria.

## Test Procedure

1. Read/admit the existing rule-model fixture through `model_source::FORMAT`
   and `NativeModel::new`; compare its `/1` artifact against the existing
   independent historical byte expectation. Select `/2` explicitly over the
   same compatible source/declarations and inspect the profile through draft
   admission and in the artifact. Try unknown format/profile values and rational
   input under both `/1` entry points; require refusal without upgrading.
2. Construct real `ir::RationalType` sites sharing one role and exact unit.
   Independently omit the role, duplicate a site across roles, repeat a scalar
   identity, remove all sites, point at an absent site or select Integer/Text
   kind. Change only one site's numerator minimum, maximum or maximum
   denominator. Each mutation refuses the whole model; the unchanged rational
   control admits and exposes the actual IR representation at every site.
3. Read a `/2` rational scalar with numerator [-1,1], maximum denominator 2
   and explicit unit. Then use i64::MIN/i64::MAX, 2^53-1, 2^53 and 2^53+1 as
   exact numerator endpoints, and denominator 1 and i64::MAX. Check exact
   retained integers against literal expectations. Independently supply
   denominator 0, -1, i64::MAX+1 and u64::MAX; reversed numerator bounds;
   missing each required field; duplicate each field/tag; unknown fields/tags;
   strings, null, fractions/exponents and integers outside i64/u64. Distinguish
   strict decoder failures from the actual IR InvalidNumericBounds diagnostic.
4. Put rational sites in values and record fields through optional and ordered
   sequence wrappers; retain existing integer/text/enum/object/operation cases
   in the same valid `/2` model. Check exact unit and nominal owner/site
   identity, with absent/null unit dimensionless and a supplied named unit
   unchanged. Use whitespace, CRLF, escaped names and repeated textual names
   at different occurrences; compare original scalar and field/value loci with
   independently calculated source spans. No reserialized source provides loci.
5. Import a real admitted `/2` model through historical `link_native`, first
   with rational content and then with only historical integer/text content.
   Both selected `/2` imports refuse before a historical LinkedPackage exists.
   Keep a matching `/1` control. Bind real `/2` exports through composed
   `bind_models`; independently substitute digest, nominal owner and export
   selection and require the existing typed refusal with original source locus.
6. Compare `/1` and `/2` artifacts over identical compatible typed inputs:
   their profile fields and raw native digests differ. Mutate rational bounds,
   unit and source bytes individually; reorder set-like role/site inventories
   separately. Compare actual artifact content and upstream IR canonical bytes,
   requiring mutation sensitivity and deterministic permitted reordering.
   Substitute IR canonical and source-byte digests at the native import slot;
   neither selects the model. No manual checksum catalog is needed.
7. Derive source-byte, entry, role/site/type-node/depth and output-size expectations
   from controlled fixed inputs and existing accounting rules, including wrapped
   rational leaves and profile-bearing artifact content. Run zero, exact,
   one-step-insufficient and above-hard capacities at each boundary; exercise
   the actual IR canonicalization/content limit. Expected limits cannot come
   from that run's reported consumption. Require refusal before excess work and
   no partially admitted model, changed input or fallback to `/1`.

## Expected Results

The selected `/2` producer preserves exact IR rational representation, source
and nominal/unit identity without changing `/1` artifacts or admitting new
semantics into historical linking. Unsupported selections, invalid fields,
IR bounds, role mismatches and exhausted limits retain typed causes and original
source correspondence. Actual admitted `/2` exports reach composed model binding;
that outcome does not certify expressions, execute rational arithmetic or
complete compiler #40. Public controls are implemented in
`tests/native_model_profiles.rs` and pass locally. PR review corrections and
broader assurance acceptance remain separate from these executed controls.
