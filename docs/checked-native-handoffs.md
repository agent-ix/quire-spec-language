# Checked native owner handoffs

FR-051 publishes two static, immutable handoffs from an already admitted
`quire.compiled-protocol/2` package:

- `quire.checked-predicate/v1` identifies a complete checked Boolean clause
  leaf, a checked `holds(...)` leaf reachable from a temporal root, or that
  temporal declaration's exact optional activation guard. The guard does not
  become a formula leaf.
- `quire.checked-temporal-subject/v1` identifies one checked temporal
  declaration, its clock/profile selection, activation and history boundary,
  temporal graph, and reachable checked predicate leaves.

The public Rust modules are `protocol_artifact::checked_predicate` and
`protocol_artifact::temporal_subject`. Each exposes `SCHEMA_BYTES`,
`SCHEMA_SHA256`, `Limits`, `derive`, and `read`. Derivation accepts only a
constructor-private strict-v2 `AdmittedPackage` and a typed table selection.
Reading accepts the same owner authority and succeeds only when the supplied
bytes equal the independently re-derived canonical document. A decoded JSON
object, a schema-valid object, or a caller assertion is never authority.

Both documents use the same closed top-level order: `contract`, `identity`,
`package`, `subject`, `source`, `clause`, `expression`, `bindings`, `type`,
`profiles`, and `limits`. Their identity is the `quire-canonical` SHA-256,
under the contract label as digest domain, of the RFC 8785 encoding of the
document with the `identity` member omitted (ADR-013 §2).
The separate `Document::digest` is the raw SHA-256 of the complete bytes.

`Limits::bounded` clamps caller ceilings to the owner maxima. Derivation and
reading bound input and output bytes, JSON depth, individual string bytes,
collection populations, expression depth, and visited fields. Exhaustion
returns `resource_incomplete` with no partial document or validated view; a
retry starts with fresh accounting.

These handoffs carry static compiler authority only. They deliberately contain
no runtime truth, observation, availability, progress, closure, completeness,
or result state. They neither accept nor invoke source parsers, evaluators,
callbacks, plugins, trust/total flags, Contract-IR vocabulary, TL proposition
values, or Boolean coercion.

The production dependency named `quire-contract-ir` in this crate selects the
cycle-free `quire-contract-model` package at its exact reviewed revision. The
legacy compatibility package remains a separately named development-only input
for historical code-generation fixtures and is not reachable from QSL's normal
dependency graph.
