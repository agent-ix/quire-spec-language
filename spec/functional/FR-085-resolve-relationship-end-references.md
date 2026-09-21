---
id: FR-085
title: "Resolve relationship-end references against the closed declaration set"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: references
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
---
# FR-085: Resolve relationship-end references against the closed declaration set

## Description

When binding a declared relationship's source and target ends, the model
binder SHALL resolve each end's declared type reference against the admitted
domain package's closed declared object types, SHALL retain the end's
declared role and multiplicity unchanged, and SHALL refuse a relationship
whose end names a type absent from the domain package.

Static navigation over a resolved relationship end — qualified-name
resolution, `r.name` runtime traversal and the typed
`Option<Reference<U>>`/`Reference<U>`/`Set<Reference<U>>`/`Bag<Reference<U>>`
result quire-specification [FR-152](ix://agent-ix/quire-specification/FR-152)'s
"Navigation" section defines — is out of this requirement's scope; it is
tracked by [#147](https://github.com/agent-ix/quire-spec-language/issues/147),
which names FR-152-AC-1/3/5/7 as its unbacked criteria. This requirement
resolves and binds a relationship end's declared type, role and multiplicity
only.

## Inputs

- Each relationship record's declared source end and target end, each
  carrying a declared type reference, an optional role and a typed
  multiplicity.
- The declared direction of the relationship (source-to-target,
  target-to-source, bidirectional or undirected).
- The set of object types the admitted domain package declares.

## Outputs

A resolved relationship binding retaining both ends' resolved type, role,
multiplicity and the relationship's direction, or a typed dangling-reference
refusal naming the missing end.

## Behavior

### Each end resolves against the closed declared type set, never a display name

For each relationship record, the model binder SHALL resolve the declared
type reference of the source end and of the target end against the object
types the admitted domain package declares. It SHALL match an end reference
by the declared type's key, never by a display name shared with an
unrelated declaration.

### A dangling end refuses the whole relationship record

If either end's declared type reference names a type absent from the
admitted domain package, the model binder SHALL refuse that relationship
record with a `missing_declaration`/`missing-name` cause naming the missing
type and the end (`source` or `target`), and SHALL admit no binding for that
relationship record. This is the same cause quire-specification
[FR-154](ix://agent-ix/quire-specification/FR-154) assigns a relationship
end naming no node of the package, which [FR-056](FR-056-admit-domain-package-model-declarations.md)-AC-5
already states locally; this requirement does not introduce a second cause
for the same condition.

### Role, multiplicity and direction are retained, never defaulted

The model binder SHALL retain each end's declared role and multiplicity
unchanged in the resolved binding. It SHALL NOT substitute a default or
inferred multiplicity for an end that declares none, and it SHALL retain the
relationship's declared direction unchanged.

### A navigation relationship carries no systems kind

A relationship whose both ends resolve to declared object types (as opposed
to declared endpoints) SHALL resolve as a plain relationship binding with no
systems-model kind. Resolving such a relationship's ends is this
requirement's scope; classifying and admitting a Port-to-Port relationship
as a Connection is
[FR-086](FR-086-bind-systems-model-references.md)'s scope.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-085-AC-1 | Given a relationship whose target end names a type absent from the admitted domain package, resolution refuses that relationship record with a `missing_declaration`/`missing-name` cause naming the missing type and the `target` end, and admits no binding for it; a relationship whose both ends resolve is admitted. | Test (TC-230) |
| FR-085-AC-2 | Given a relationship whose source end declares a role and a bounded multiplicity, the resolved binding retains that exact role and multiplicity; no default or inferred multiplicity appears for an end that declares none. | Test (TC-231) |
| FR-085-AC-3 | Given a relationship whose two ends both resolve to declared object types, and separately a relationship whose two ends both resolve to declared endpoints, resolving each under this requirement carries no systems-model kind for either — this requirement does not itself decide Connection classification; that determination is FR-086's separate, later step. | Test (TC-232) |

## Dependencies

- **Upstream:** [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  supplies the closed declared type set this requirement resolves against;
  [FR-056](FR-056-admit-domain-package-model-declarations.md) builds the
  relationship export and its declared ends from the domain package.
  AD-006's model-view decision keeps relationships in the checked model
  view. `src/model/systems.rs`'s relationship-classification loop (lines
  275-317) is the only place a plain object-to-object relationship's ends
  are resolved today, and it does not yet implement this requirement's
  `missing_declaration`/`missing-name` cause: for a relationship where one
  end resolves to a declared object type and the other end resolves to
  neither a declared object type nor a declared endpoint, it reports both
  ends `UnknownEndpoint`/`dangling_reference`, including the legitimately
  resolving end, rather than refusing only the missing end with this
  requirement's cause. FR-085-AC-1 does not ship today. Remaining work:
  #120.
- **Downstream:** relationship end resolution feeds the object-reference
  fields the evaluator later dereferences; this requirement does not itself
  perform reachability or dereference evaluation, which is
  [FR-047](FR-047-evaluate-finite-object-reference-graphs.md)'s scope over
  an already-admitted value graph.
- Relates to [FR-086](FR-086-bind-systems-model-references.md), which
  classifies and admits Port-to-Port relationships as Connections; this
  requirement's scope is bounded to plain object-to-object relationship-end
  resolution.
