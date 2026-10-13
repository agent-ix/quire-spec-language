<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-7876 round=1 reviewed=1526e180019bd3f24424dabc5854fa330149ea61 model=claude-opus-5-5 run=bcd3a000-c7d0-41d3-976e-ff88d49053f8 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 1526e1800 |
| FND-002 | fixed | 1526e1800 |
| FND-003 | fixed | 1526e1800 |
| FND-004 | fixed | 1526e1800 |
| FND-005 | fixed | 1526e1800 |
| FND-006 | fixed | 1526e1800 |

New findings this round (no outcome yet):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-007 | low | soundness | A case that repeats an earlier case's name stops after the `conflicting-binding` refusal |
| FND-008 | low | code-bug | The sink has two invariants the types do not enforce |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "    let forbidden_meaning = value_type.as_package().is_some_and(|key| {\n        package.records.iter().any(|record|\
    \ {\n            record.key() == key\n                && match record {\n                    DomainPackageRecord::Enum(_)\n\
    \                    | DomainPackageRecord::EventType(_)\n                    | DomainPackageRecord::StateMachine(_)\n\
    \                    | DomainPackageRecord::Process(_)\n                    | DomainPackageRecord::PersistenceInterface(_)\n\
    \                    | DomainPackageRecord::Namespace(_) => true,\n                    DomainPackageRecord::ObjectType(object)\
    \ => ..."
- fnd: FND-002
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: '/// `shape` is the `shape` of the type''s own construct. Every defect of the

    /// type is reported, not only the first (FR-154: every failing check of one

    /// node), in member-path order: the construct shape (`TypeDefinition.kind`),

    /// the empty `variants` list, then each case in turn -- a repeated name

    /// (`variants[i]`), then its `identity`, `origin` and `payloadType`. Records

    /// are emitted in ascending case-name byte order, so the document''s

    /// `variants` order changes no record.'
- fnd: FND-003
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "        // A population is read from `populations[]`; a `types[]` entry\n        //\
    \ bound to it is a node not valid for its construct's meaning\n        // (FR-056 \"Meaning binding\"\
    , FR-154's construct validity rule).\n        meaning::POPULATION => {\n            return Err(\n\
    \                ctx.malformed(\"kind: a type definition is not valid for the population meaning\"\
    )\n            );\n        }"
- fnd: FND-004
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "            DomainPackageRecord::Enum(variant_type) => {\n                work.bytes(artifact_id(&variant_type.key).len())?;\n\
    \                work.charge(Dimension::Entries, 1)?;\n                if result\n               \
    \     .insert(\n                        (\n                            w::ExportKind::Enum.as_str(),\n\
    \                            artifact_id(&variant_type.key),\n                            \"\",\n\
    \                        ),\n                        (w::ExportKind::Enum, DomainTarget::Enum),\n\
    \                    )\n                    .is_some()\n                {\n                    retu..."
- fnd: FND-005
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "                    if variant_types.contains(name) {\n                        // FR-150:\
    \ a model variant type referenced by source.\n                        errors.push(AssemblyError {\n\
    \                            cause: AssemblyCause::UnsupportedVariantType { name: name.clone() },\n\
    \                            span: target.span,\n                        });\n                   \
    \ } else if !object_types.contains_key(name) {\n                        errors.push(AssemblyError\
    \ {\n                            cause: AssemblyCause::UnresolvedTypeName { name: name.clone() },\n\
    \                            spa..."
- fnd: FND-006
  outcome: fixed
  fix_sha: 1526e1800
  after_excerpt: "    for (name, identity) in cases {\n        records.push(DomainPackageRecord::Variant(VariantRecord\
    \ {\n            key: declaration_key(package, identity),\n            owner: owner.clone(),\n   \
    \         case: name.to_owned(),\n        }));\n    }"
new_findings:
- fnd: FND-007
  severity: low
  confidence: low
  method: code-review
  check: soundness
  artifact_id: qsl-semantics/src/model/intake.rs#read_variant_type
  related_ids:
  - FR-056
  - TC-996
  path: qsl-semantics/src/model/intake.rs
  lines: 3629-3653
  excerpt: "        if let Some(&(first, first_identity)) = first_of.get(name) {\n            if identity\
    \ == first_identity {\n                // One identity declared twice: the reader's own\n        \
    \        // `duplicate_identities` diagnostic is retained for it\n                // (FR-056's reader-first\
    \ rule), so no second case is read.\n                continue;\n            }\n            refusals.push(ModelRefusal\
    \ {\n                code: Code::InvalidModelBinding,\n                cause: ModelRefusalCause::ConflictingBinding\
    \ { key: owner.clone() },\n                detail: format!(\n                    \"{c..."
  finding: A case that repeats an earlier case's name stops after the `conflicting-binding` refusal. Its
    identity, origin and payload checks are skipped, so a repeated case that also has no span or a payload
    reports only the conflict. A repeated name with a distinct identity always has a mis-minted identity,
    so skipping avoids a guaranteed second refusal. That is defensible, but FR-056 and TC-996 step 5 do
    not state the rule, and FR-154 says every failing check of a node reports. State it in FR-056 "Variant
    types" or report the other checks.
- fnd: FND-008
  severity: low
  confidence: medium
  method: code-review
  check: code-bug
  artifact_id: qsl-semantics/src/model/variant_type.rs#bind
  related_ids:
  - qsl-semantics/src/model/intake.rs#read_nodes_partial
  path: qsl-semantics/src/model/variant_type.rs
  lines: 14-28
  excerpt: "    let Err(mut found) = read_variant_type(\n        context.package,\n        context.type_value,\n\
    \        context.node,\n        context.at,\n        context.construct_shape,\n        context.records,\n\
    \    ) else {\n        return Ok(());\n    };\n    // `found` is never empty; the binder reports one\
    \ refusal and the rest\n    // ride in `more_refusals`, in report order.\n    let first = found.remove(0);\n\
    \    context.more_refusals.append(&mut found);\n    Err(first)"
  finding: The sink has two invariants the types do not enforce. `read_nodes_partial` drains `more_refusals`
    only on the `Err` arm, so a binder that pushed to it and then returned `Ok` would have its refusals
    attached to the next refused node, or dropped. `variant_type::bind` calls `found.remove(0)`, which
    panics if `read_variant_type` ever returns `Err(vec![])`. Each holds today. A `debug_assert!(more_refusals.is_empty())`
    on the `Ok` arm and `split_first`/a non-empty type would make them checked.
```

+++
