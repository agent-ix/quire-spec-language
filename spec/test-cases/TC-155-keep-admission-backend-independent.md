---
id: TC-155
title: "Keep admission backend-independent and route only supported items"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-155: Keep admission backend-independent and route only supported items

## Description

Verify that the QSL composed linker admits requested pairs without backend
input, that registration admits advertised (kind, mode) pairs under the same
rules, that candidate sets follow the FR-290 rule independently of registration
order, and that routing consumes settled dispositions as data. Negotiation
itself is quire-contract-codegen's and is not run here; its settlements are
supplied as fixture records shaped like `negotiate_*` output. Scope:
FR-057-AC-5, FR-057-AC-6, FR-057-AC-8 and FR-057-AC-9.

## Test Procedure

1. Link one protocol declaration and one state declaration with two required
   requested pairs: `global-conformance` on the protocol declaration and
   `operation-contract` on the state declaration. Add a temporal declaration
   with no requested pairs. Check that the admission
   entry point takes no registry or backend parameter.
2. Admit the pairs under three registries: empty, one backend advertising only
   (`global-conformance`, `bounded`), and two backends advertising different
   kinds. Compare the admitted pairs, static components and family-checker
   handoff.
3. Register a backend advertising `GlobalConformance`, then one advertising no
   kind, then one advertising (`value-validity`, `finite`), then one repeating
   an already registered backend identity.
4. Register two backends that both advertise (`operation-contract`,
   `bounded`) and compute the `operation-contract` item's candidates with no
   named backend, then naming one of them, then naming a registered backend
   that advertises only `global-conformance`, then naming an unregistered one.
5. Supply settled records: `global-conformance` `supported`,
   `operation-contract` `unsupported` with its empty-candidate warning. Route
   them.
6. Repeat steps 4 and 5 with the same backends registered in reverse order.
7. Supply, as fixture records, a run result in which the `supported` item
   timed out, then FR-331 `unsupported`/`tool-unavailable` results for a
   missing tool, a different tool identity, a probe error and a probe past its
   limit, then an FR-331 `failed`/`tool-unavailable` result for a tool changed
   after a passing probe. Route them.

## Expected Results

- Step 1: the entry point has no registry or backend parameter.
- Step 2: admitted pairs, static components and handoff are identical in all
  three runs. All three declarations, including the temporal declaration with
  no requested pairs, reach their family checker with no capability
  request selecting them.
- Step 3: each registration refuses with `invalid_capability`
  (`unknown-kind`, `absent-kind`, `unknown-mode`, `duplicate-backend`), keyed
  by backend identity; each refused registration contributes nothing, and the
  registration already held under the repeated identity stands.
- Step 4: the unnamed request carries both backends as (identity, manifest
  digest) candidates ordered by identity then digest, and no chosen one; the named request carries exactly the named
  backend; the backend lacking the kind yields an empty candidate set; the
  unregistered name carries the unknown-backend mark.
- Step 5: only the `global-conformance` item is routed. The
  `operation-contract` item gets no target and no artifact, is not a refusal,
  and nothing is pending after routing returns.
- Step 6: candidate sets and the route are the same as in steps 4 and 5.
- Step 7: the timeout is carried as a run result, not as `unsupported`, a
  refusal or a hold. For each tool record, `unsupported` or `failed`, routing keeps the item's
  `supported` disposition, reports neither a refusal nor a hold, and re-routes
  it to no other candidate or mode.
- Assertions compare typed codes, causes and identities, never message text.
