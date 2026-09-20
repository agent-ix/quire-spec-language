---
id: US-007
title: "Trust a single S2 forms producer between the CST and family checking"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-007: Trust a single S2 forms producer between the CST and family checking

## Story

**As a** QSL contributor implementing or reviewing a semantic family
**I want** exactly one stage-boundary producer of parsed, span-only semantic
input between the lossless CST and family checking, with no other code path
able to hand checking a form built from source text
**So that** I never have to ask whether some other module — a leftover
structural lowering, a second parser — could also be feeding checked input,
and a parsed form I inspect never carries a semantic identity that checking
has not minted yet.

## Context

Before this ticket, `complete::package::lower_source_graph` produces a
`LoweredSourceGraph`: a structural lowering established by #117, before
semantic checking existed as a distinct stage. ADR-010 §2.6 X4 found it has
no consumer anywhere in `src` outside the module that defines it — a dead
end (SEAM-5). At the same time, the dedicated S2 stage ADR-011 assigns the
CST-to-parsed-form edge (E2) to — a `forms` module, at the layer-2 position
ADR-011 §6.1 names — does not exist yet. Two things are wrong about this at
once: a dead path stays in the tree wired to nothing, and the boundary meant
to replace it has not been built either. Leaving the dead end in place, even
unused, is a standing risk that a future change wires it back into a real
path once something resembling S2 exists, which would reopen exactly the
"more than one path can derive meaning from source" gap ADR-011 §3 (FB-01)
closes.

## Acceptance Examples (Illustrative)

### US-007-EX-1: A parsed form carries only position, no identity

- **Given** a source construct newly parsed at the S2 boundary.
- **When** I inspect the resulting parsed form.
- **Then** it carries the construct's span and nothing that could serve as a
  semantic identity; identity is minted later, only at check (S3).

### US-007-EX-2: A CST with a recovery node produces no form

- **Given** a CST that still carries an error or recovery node.
- **When** it reaches the S2 boundary.
- **Then** no parsed form is produced; the boundary returns a refusal
  instead of building a form from the incomplete tree.

### US-007-EX-3: The old dead end is gone, not dormant

- **Given** the S2 forms stage has landed.
- **When** I search the compiled crate for `LoweredSourceGraph`,
  `LoweredDeclaration`, or `lower_source_graph`.
- **Then** none of the three symbols exist anywhere in the crate, and no
  disabled or feature-gated copy of the pre-migration test that exercised
  them remains either.

## Priority and Risk (Informative)

Priority: High. ADR-011 §3's FB-01 guarantee — that nothing after S2 reads
source text or the CST to recover meaning — depends on there being no second
path back into source text. A dead end left in place, even one with no
current consumer, is exactly the kind of leftover a later change can
accidentally wire into the checking or packaging path once a real S2
boundary exists, which would quietly reopen the guarantee this ticket exists
to close.

## Traceability (Informative)

- [FR-067](../functional/FR-067-add-s2-forms-and-retire-seam-5.md)
