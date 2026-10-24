<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-7877 round=1 reviewed=1526e180019bd3f24424dabc5854fa330149ea61 model=claude-opus-5-5 run=bcd3a000-c7d0-41d3-976e-ff88d49053f8 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | waits on PR #796; site list in SR file |
| FND-002 | fixed | 1526e1800 |
| FND-003 | fixed | 1526e1800 |
| FND-004 | fixed | 1526e1800 |
| FND-005 | fixed | 1526e1800 |

New findings this round (no outcome yet):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-006 | low | other | TC-996 step 4 still names `Snap Shot` and `Snap_Shot` |
| FND-007 | medium | ambiguous | The new mis-minted text says intake names "the later case node (the case whose identity is not minted)" with path `TypeDefinition.variants[i].identity` |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: still-open
  reason: 'waits on PR #796 (planned order: #796 first, then #799 adopts). The site list grows with the
    fix round. Typed `member` assertions are needed for: payload `Variant.payloadType` (`a_payload_on_a_case...`,
    and both cases in `every_payload_case_of_one_type_is_reported`); `TypeDefinition.variants` (`a_variant_type_with_no_case...`);
    `TypeDefinition.variants[0].origin` (`a_case_with_no_source_span...`); `TypeDefinition.kind` (`a_variant_meaning_on_a_record_shaped_construct...`);
    both `TypeDefinition.kind` and `TypeDefinition.variants` (`an_empty_and_mis_shaped_variant_type_reports_both`);
    `TypeDefinition.variants[1]` via `ConflictingBinding.member` (`equal_case_names_with_distinct_identities...`);
    the mis-minted case path, root per FND-007 (`a_case_identity_that_is_not_minted...`); `Field.typeRef`
    (`a_field_typed_by_a_variant_type...`); and the POPULATION node `TypeDefinition.kind` (`refuses_a_type_definition_bound_to_the_population_meaning_as_malformed`).
    Every `read_variant_type` site needs an explicit path, because #796''s derived path at `$.types[i]`
    stops at `TypeDefinition`'
- fnd: FND-002
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "/// TC-996 step 3: the variant-type meaning on a construct of shape `record`.\n#[trace(\"\
    TC-996\", \"FR-056-AC-26\")]\n#[test]\nfn a_variant_meaning_on_a_record_shaped_construct_refuses_as_malformed()\
    \ {\n    let document = single_type(|d| {\n        d[\"constructs\"][0][\"construct\"][\"shape\"]\
    \ = json!(\"record\");\n    });\n    assert_one_malformed(&document, \"Tier\");\n}"
- fnd: FND-003
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "fn two_cases_sharing_one_identity_are_refused_by_the_reader() {\n    let document =\
    \ single_type(|d| {\n        d[\"types\"][0][\"variants\"] = json!([case(\"Tier\", \"Gold\", 10),\
    \ case(\"Tier\", \"Gold\", 11)]);\n    });\n    let found = refusals(&document);\n    assert_eq!(found.len(),\
    \ 1, \"{found:?}\");\n    assert_malformed(&found[0], \"variant/Tier-Gold\");\n    assert!(\n    \
    \    !found\n            .iter()\n            .any(|refusal| matches!(refusal.cause, ModelRefusalCause::ConflictingBinding\
    \ { .. })),\n        \"the reader's diagnostic is kept, not intake's conflicting-binding\"\n    );\n\
    }"
- fnd: FND-004
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "#[trace(\"TC-996\", \"FR-056-AC-26\")]\n#[test]\nfn enumeration_rows_that_slug_alike_are_refused_by_the_lift()\
    \ {\n    let refused = lifted(&[\"Snap_Shot\", \"Snap__Shot\"]).expect_err(\"slug-alike rows are refused\"\
    );\n    let qsl_semantics::model::intake::LiftFailure::Blocked(diagnostics) = refused else {\n   \
    \     panic!(\"a blocking diagnostic, got {refused:?}\");\n    };\n    assert!(\n        diagnostics\n\
    \            .iter()\n            .any(|d| d.code.to_string() == \"agent-ix.extraction-frontend.UNSLUGGABLE_NAME\"\
    ),\n        \"{diagnostics:?}\"\n    );\n}"
- fnd: FND-005
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "    // The package the refused unit selected carries the declaration and\n    // exports\
    \ of step 1's package, which no clause names.\n    let step_one = admitted(&document);\n    let (unit,\
    \ packages) = config_unit_with_body(\n        &bytes,\n        \"invariant OnTier using v on Config::Tier\
    \ at current { true }\",\n    );\n    let built = parse_and_build(&unit);\n    let models = admit_unit(\n\
    \        &built.selections().models,\n        &packages,\n        ModelNormalizationLimits::default(),\n\
    \    )\n    .expect(\"the package is admitted although the clause is refused\");\n    assert_eq!(models[0].view.doma..."
new_findings:
- fnd: FND-006
  severity: low
  confidence: high
  method: gap-analysis
  check: other
  artifact_id: TC-996
  related_ids:
  - FR-056-AC-26
  path: spec/test-cases/TC-996-refuse-malformed-and-duplicate-variant-cases.md
  lines: step 4
  excerpt: 4. `Tier` with two cases named `Gold` and one identity `variant/Tier-Gold`. Then lift a bundle
    whose enumeration rows slug alike (`Snap Shot`, `Snap_Shot`).
  finding: TC-996 step 4 still names `Snap Shot` and `Snap_Shot`. `Snap Shot` is not a valid enumeration
    value under the lifted module's `Identifier` pattern, so the TC cannot be run as written. The test
    uses `Snap_Shot` and `Snap__Shot`. Update the TC example.
- fnd: FND-007
  severity: medium
  confidence: medium
  method: gap-analysis
  check: ambiguous
  artifact_id: FR-056
  related_ids:
  - FR-056-AC-26
  - TC-996
  path: spec/functional/FR-056-admit-domain-package-model-declarations.md
  lines: 500-504
  excerpt: 'that share one identity. If a case''s identity is not the minted

    `variant/<Name>-<value>` of its owner and name, then intake SHALL refuse the

    case with `invalid_model_binding`/`malformed-declaration`, naming the later

    case node (the case whose identity is not minted) and the path

    `TypeDefinition.variants[i].identity` for that case.'
  finding: 'The new mis-minted text says intake names "the later case node (the case whose identity is
    not minted)" with path `TypeDefinition.variants[i].identity`. "Later" only makes sense next to another
    case. A lone mis-minted case, which the test uses, has no earlier case. The path is also rooted at
    `TypeDefinition` while the refusal names the case node. The payload refusal also names the case node,
    but its path is `Variant.payloadType`. #796''s typed `member` will need one convention for refusals
    at a case node. Say "the case node whose identity is not minted" and pick one root (`Variant.identity`,
    as the payload uses) in FR-056 and TC-996.'
```

+++
