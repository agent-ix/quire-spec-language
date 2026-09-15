# Composed linked-package reference definition

Artifact interpretation: `quire.package.composed/v1`; revision: `1-draft.2`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**
This defines a semantic reference to a fully linked package. It is not a source
profile and cannot be selected by a clause's `using` alias.

This is a working definition in one coherent draft. Required definitions and
rule links resolve within this checkout; acceptance freezes their exact selections.
Earlier draft text remains in Git history. Runtime references still require the
selected immutable identities, revisions and digest domains.

## Required definition

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `ix:native` / `1-draft` | `1-draft.2` | [Composed edition](edition.md) |

## Package and clause selections

The package's outer `SemanticRef.semanticProfile` selects this definition's
identity, revision and exact byte digest. Its artifact kind is `linked-package`;
its language selects the composed edition. The inner package retains every
declaration's exact state, predicate, temporal or protocol profile selection,
typed dependency closure, source correspondence and required binding roles.
The outer selection describes that container contract; it does not replace,
merge or choose between the inner clause meanings.

A state/temporal/protocol profile cannot stand in for this outer package
interpretation, even for a package containing only that family. Conversely,
this outer interpretation cannot supply a missing clause profile or admit a
construct disallowed by it. Changing one inner selection changes the static
package subject; the outer definition identity alone cannot identify that subject.
The exact artifact content binding and inner manifest remain required.

For this definition, `requiredFeatures` is exactly the duplicate-free set below
derived from the package's declarations, including unused checked predicates.
It is not a claim about executed paths, backend support or release conformance.

| Feature | Included exactly when |
| --- | --- |
| `declaration.predicate` | At least one named predicate is declared. |
| `family.state` | At least one invariant, precondition or postcondition is declared. |
| `family.temporal` | At least one temporal clause is declared. |
| `family.protocol` | At least one protocol clause is declared. |

Missing, extra or unknown feature entries refuse this interpretation. Unique
feature order is nonsemantic under the selected shared-reference rules. A reader
still validates every inner definition and required feature; recognizing these
four outer entries is not permission to ignore an unknown inner operator/profile.
A later declaration family needs an explicitly revised definition and feature
contract, not silent omission from this catalog.

Partial compilation and diagnostics cannot claim a fully linked package through
this interpretation. Independently checked, dependency-closed declarations may
still have their own scoped references and results under FR-030/031. Successful
execution of one such declaration cannot certify the refused whole package.

## Wire and canonical identity

This candidate composes the existing `ix.shared-reference/2-draft` envelope and
`ix.artifact-ref/3-draft` fields. It adds no field or artifact-kind discriminant.
The referenced package payload still needs its own exact supported wire contract;
this semantic definition neither supplies a serialization layout nor makes an
unknown payload wire decodable. Acceptance of the envelope is insufficient.

For this candidate's whole-package reference, `canonicalIdentity` is null: no
whole-package semantic canonicalizer is selected. A raw artifact digest, an IR
expression digest or the verification JCS domain cannot substitute for one.
Build provenance, runtime inputs and static components retain the separations
in the required edition's package contract. Historical references keep their
original profiles and wire versions; no in-place migration is inferred.

## Selected reference rules

The following current reference rules supply the declared envelope contract.

| Rule file |
| --- |
| [Reference schema](../../shared-reference-2-draft/schema.json) |
| [Reference meanings](../../shared-reference-2-draft/README.md) |

TC-030 and IT-010 plan the whole-package/inner-profile controls. Existing reference
schema and canonicalizer tests do not qualify this new artifact interpretation
or supply the pending strict linked-package reader.
