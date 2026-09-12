# Verification reference amendment

This private amendment addresses B's conditional review of specification PR8.
It is an internal implementation contract after its recorded specification
review; independent B adoption/qualification and public promotion remain separate.
The adopted state-core packet at e897f81 and its 1-draft schema remain unchanged.

## Selected versions and scope

The new envelope has exactly wireVersion, kind and value. wireVersion is
`ix.shared-reference/2-draft`; kind is artifact or semantic. This version covers
these two forms only. Existing source-map and migration envelopes keep their
1-draft selection; this amendment supplies no inferred migration, decoder
fallback or general extension bag.

An artifact value selects `ix.artifact-ref/3-draft`. Its seven mandatory fields
are refVersion, kind, authority, identity, revision, digest and wire. Revision
has exactly namespace and value (nonempty strings). Wire has exactly identity
and version (nonempty strings). Authority and identity are nonempty strings.
Digest is exactly `sha256:` plus 64 lowercase hexadecimal digits, hashing all
supplied artifact bytes with no normalization. Every record is closed.

The fourteen prior kinds remain: source, model-package, linked-package,
executable-projection, property, binding, snapshot, invocation,
generated-artifact, run-artifact, environment, model-manifest, model-lock and
dependency-closure. The six additional kinds have these exact roles:

| Kind | Artifact selected |
| --- | --- |
| trace | Recorded ordered trace supplied as verification input. |
| fault-model | Authored model of faults/mutations selected for a verification method. |
| review-procedure | Authored procedure governing a manual review. |
| oracle | Definition or executable artifact selected as the oracle; its role alone does not establish independence or correctness. |
| observation | Retained observation contributed to an assessment; its role alone does not establish truth or independence. |
| review-disposition | External disposition artifact about an exact reviewed core; separate from its procedure and observations. |

The consumer selects its expected role explicitly. A structurally valid nearby
kind is an identity-mismatch for that role. No generic-kind fallback is permitted.
Artifact identity is the exact tuple of kind, authority, identity, structured
revision, digest and wire under its selected refVersion. Equal bytes alone cannot
join different roles. Conflicting contents for one immutable identity/revision
remain identity-content-conflict. Changing versions requires a separately accepted
mapping; an amendment's existence is not that mapping.

## Semantic reference and JCS identity

A semantic value has exactly artifact, language, semanticProfile,
requiredFeatures and canonicalIdentity, preserving the prior field meanings.
All are required; only language, semanticProfile and canonicalIdentity may be
null. A null profile requires null language/canonicalIdentity and no required
features. A non-null profile has exactly identity, version and definitionDigest;
the digest is an exact byte digest. A non-null language has identity and edition.

requiredFeatures is a duplicate-free decoded set of nonempty Unicode scalar
strings. Producers order it by UTF-8 bytes; readers accept reordered unique sets
and reject duplicate decoded members before comparison. Unknown required features
refuse interpretation. This rule does not sort arrays inside the referenced
artifact's canonical preimage.

CanonicalIdentity has exactly domain, version, algorithm and digest, with these
closed alternatives:

| Domain | Version | Algorithm | Digest syntax |
| --- | --- | --- | --- |
| quire.contract.canonical-json | v1 | sha256 | 64 lowercase hex digits |
| quire.contract.bound-identity | v1 | sha256 | 64 lowercase hex digits |
| quire.verification.jcs | rfc8785-v1 | sha256 | sha256-jcs: followed by 64 lowercase hex digits |

The first two alternatives retain their existing algorithms, including their
different preimages. The new alternative preserves B's existing
`contracts::jcs_sha256`: SHA-256 over RFC 8785 canonical UTF-8 bytes of the
qualified structured value; prepend `sha256-jcs:` to the digest text afterward.
The prefix is not included in the hashed bytes. The semantic profile defines
which validated value is selected. A caller cannot strip arbitrary fields,
canonicalize arbitrary IR/source bytes or infer equivalence from a digest alone.

JCS preserves array order and string contents, sorts object keys by UTF-16, and
uses its specified binary64 number serialization. Input intake must reject
duplicate decoded keys, invalid Unicode and non-finite numbers. Exact-integer
contracts that cannot admit binary64 round trips need a different selected
representation. These rules follow [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html#section-3);
no RFC implementation or example corpus is copied here.

A raw byte digest cannot be converted by changing its prefix. Even if an artifact
already contains canonical bytes and the hexadecimal hash happens to agree,
byte identity and semantic identity still have different checked obligations.
Unknown domain/version/algorithm selections refuse with
unsupported-canonicalization before interpretation; malformed digest syntax is
invalid-wire, and a validly shaped incorrect digest is identity-mismatch.

## Non-circular automated-core correspondence

B's report profile selects the complete validated automated core before hashing.
Neither the external disposition nor the outer wrapper belongs to that preimage.
The disposition contains the exact resulting sha256-jcs core digest, and its
ArtifactRef uses review-disposition with a separate digest over its actual bytes.
Attaching a disposition cannot change the sealed core or its digest. A changed
core requires a new matching disposition. A foreign digest, byte-digest
substitution or nearby artifact kind refuses correspondence.

B owns all report-core fields, validation, reviewer/procedure/provenance checks,
missing-disposition incompleteness and final result policy. The fixture here uses
an explicitly synthetic fixture profile to demonstrate the projection and hash
rule without claiming to define B's report payload or record a real review.
B qualifies the same rule against its actual report schema under IT-003-SC-03.

## Qualification boundary

FR-017/018 and TC-001–004 define positive and adverse vectors; IT-003 records
producer structure, existing B canonicalizer parity and the later independent
strict reader as distinct observations. Structural schema validation is only
one layer. Readers retain the strict bounded intake, source/authority and
artifact checks from the adopted packet. Exhaustion yields incomplete without
partial acceptance. No network schema retrieval is authorized by schema IDs.

The candidate schema and vectors are first-party authored data. Existing document
and reusable-artifact publication terms remain unresolved; the packet stays
private. Any new executable qualification source is Rust under AGPL-3.0-only,
with original dependency grants retained. No hosted workflow is dispatched.
