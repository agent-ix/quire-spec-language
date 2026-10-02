---
id: FR-355
title: "Check the quire fences of spec artifacts against the objects they declare"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-036
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-004
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
---
# FR-355: Check the quire fences of spec artifacts against the objects they declare

## Description

QSL SHALL provide the Check-stage library operation `check_fences` (ADR-029
LC-1, OP-1), one of the lifecycle operations of FR-275 with its call shape,
its limits (FR-277) and its cancellation (FR-276). The driver's `check`
verb calls it when its arguments name a spec artifact inventory and its
module manifests (ADR-029 CB-5). Given a spec artifact
inventory and the module manifests that type it, it checks the expression
in every `quire` fence of every artifact as a state invariant of the object
type that artifact declares, and reports each refusal at the artifact's path
and the line and column inside the fence where it arises.

The operation reuses three existing paths: FR-056's bundle entry point
builds the domain package from the same inventory, quire-rs extraction
locates the fences and gives each body's source map (FR-030), and FR-104's
state-invariant check types each body. The grammar and typing rules are
the ones S1 to S3 already apply to an invariant; this requirement applies
them to fence bodies and adds none.

## Inputs

- `inventory`: the artifact paths and bytes, each with its FR-001 source
  identity, as FR-056's bundle entry point takes them.
- `manifests`: the module manifests that type the artifacts, selected by
  module identity.
- `alias`: the model alias fence bodies name the domain package's
  declarations by, as `alias::<artifact id>` (FR-056).
- `limits`: FR-056's `ModelNormalizationLimitsV1` and the S1 to S3 stage
  limits, each caller-configurable with its published default (ADR-029
  LC-4).
- `cancel`: the operation's `Cancel` handle (ADR-029 LC-3).

## Outputs

`Result<Staged<FenceReport>, StageFailure<FenceCheckCause>>` (ADR-029
LC-2). A `FenceReport` holds one `FenceResult` per `quire` fence, in
inventory order and then document order. Each `FenceResult` carries the
artifact path, the clause id of the heading that owns the fence, the fence's
first body line, and its diagnostics: empty when the body checks, otherwise
the refusals FR-104's check returns, each located by its artifact path, line
and column. `StageFailure` carries a refusal of the inventory as a whole:
FR-056's intake refusals, an extraction that is not available, a reached
limit, and `StageFailure::Cancelled` when the run is cancelled.

## Behavior

- `check_fences` SHALL build the domain package from `inventory` and
  `manifests` through FR-056's bundle entry point, and SHALL take each
  artifact's declaration from that package by the artifact's id.
- `check_fences` SHALL locate every fence through quire-rs extraction over
  each artifact's original bytes, with no second Markdown parser, and SHALL
  verify each body's source map with `SourceMap::verify` before it
  translates any offset (FR-004, FR-030).
- For each fence whose language is `quire`, `check_fences` SHALL check the
  body as the body of a state invariant on the artifact's declared object
  type, `self` the receiver and the domain package the model selected
  under `alias`, through S1 to S3 (FR-104), and SHALL report the check's refusals with their locations
  translated through the body's source map into the artifact's path, line
  and column.
- A body that does not parse, a name that does not resolve against the
  declared object type's fields or the package's declarations, and a body
  whose type checking refuses SHALL each give at least one diagnostic in
  that fence's `FenceResult`.
- One fence's refusal SHALL NOT stop the check of the other fences; every
  `quire` fence of the inventory SHALL get a `FenceResult`.
- When an artifact holds a `quire` fence and declares no object type the
  package admits, `check_fences` SHALL report the fence with
  `missing_declaration`/`missing-name` naming the artifact's id.
- A fence whose language is not `quire` SHALL get no `FenceResult`.
- When `cancel` is set, `check_fences` SHALL stop within one charge and
  return `StageFailure::Cancelled` with no `FenceReport` (FR-276).
- `check_fences` SHALL read no path, environment variable, clock or search
  location beyond its inputs, and SHALL give the same result for the same
  input.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-355-AC-1 | An entity artifact `account` whose Properties table declares `account_id: Integer 1..1` (identity), `verified: Boolean 1..1`, `suspended: Boolean 1..1`, and `first_order: Integer 1..1` and `suspended_at: Integer 1..1`, each with presence `optional`, with the clauses `present(self.first_order) implies self.verified` and `self.suspended = present(self.suspended_at)`, gives a `FenceReport` of two `FenceResult`s, each with no diagnostic. | Test (TC-901) |
| FR-355-AC-2 | The AC-1 artifact with four more clauses, `present(self.verified)`, `self.account_id`, `self.nickname` and `self.suspended and`, gives six `FenceResult`s: the first two have no diagnostic; `present(self.verified)` has `ill_typed`/`type-mismatch`, since `verified` is not optional; `self.account_id` has `ill_typed`/`non-boolean-root`, since an `Integer` stands where the clause needs a `Boolean` (FR-104-AC-3); `self.nickname` has `missing_declaration`/`missing-name`; `self.suspended and` has `invalid_syntax`/`unexpected-end`. Each refusal names the artifact's path and the line inside its fence where the error is. | Test (TC-901) |
| FR-355-AC-3 | The AC-1 inventory with an `enumeration` artifact `tier` declaring the values `Standard` and `Gold`, and a clause of `account` that names it, `self.verified implies Accounts::tier::Gold = Accounts::tier::Gold` under the alias `Accounts`, gives that clause's `FenceResult` `unsupported_construct`/`declaration-form` at the name `Accounts::tier`, the refusal FR-056 gives a clause naming a variant type, and leaves AC-1's two `FenceResult`s with no diagnostic. | Test (TC-901) |
| FR-355-AC-4 | A fence body written on its second line, after an indented first line, reports its refusal at the artifact line and column of the offending token, computed through the fence's source map, for an artifact with CRLF line endings and for the same artifact with LF line endings. | Test (TC-901) |
| FR-355-AC-5 | An artifact whose only fence is labelled `text` gives no `FenceResult`; an artifact with a `quire` fence whose frontmatter names an object type no supplied manifest declares gives one `FenceResult` with `missing_declaration`/`missing-name` naming the artifact's id. | Test (TC-901) |
| FR-355-AC-6 | The AC-2 inventory checked twice gives equal reports; checked with a cancelled `Cancel` it returns `StageFailure::Cancelled` and no report. | Test (TC-901) |

## Dependencies

- [FR-056](FR-056-admit-domain-package-model-declarations.md): the bundle
  entry point that builds the domain package from the artifacts.
- [FR-030](FR-030-consume-quire-extraction.md): quire-rs extraction and the
  fence body's source map.
- [FR-004](FR-004-verify-source-maps.md): source-map verification.
- [FR-104](FR-104-check-state-clauses.md): the state-invariant check each
  body goes through.
- ADR-029 LC-1 to LC-4: the Check stage, request and outcome shape,
  cancellation and limits.

## References

- Linear QSL-62.
