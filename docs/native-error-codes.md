# Native diagnostic codes

FR-010 exposes one native `Diagnostic` envelope with phase, code, exact caller
source labels, display path, original span and explanatory message. `Display`
renders `code: message`; `std::error::Error` has no underlying cause. The public
`source` field is provenance, so the standard traits are implemented directly:
the pinned thiserror derive would incorrectly treat that field as an error cause.
The fixture-audit target retains its separate [audit codes](audit-error-codes.md).

Native package APIs use the same Code vocabulary in a separate PackageError
envelope with a package stage, field/index path and measured pass usage. Its
typed cause retains the original JSON or native diagnostic. The producer is in
progress under Task-016; package-reader refusal qualification remains Task-017.

`Code::all()` enumerates this vocabulary, `as_str()` supplies its stable spelling,
and `from_code()` returns `None` for unknown spellings. Codes are not renamed or
reused; prose may change without changing classification.

| Code | Meaning |
| --- | --- |
| io-error | A selected local file could not be opened or read. |
| invalid-request | A closed command request has invalid JSON or fields. |
| invalid-digest | A selected command digest has an invalid spelling. |
| invalid-identifier | A selected command identifier fails its constructor. |
| output-failure | A typed command result could not be serialized. |
| unsupported_projection | Native expressions are outside the selected executable projection. |
| projection_binding | The existing IR rejected the derived executable projection. |
| invalid_projection_correspondence | Checked native correspondence could not be retained by the projection. |
| extraction-requires-run | Source-only compile/lower commands do not admit extracted programs. |
| extraction-package-conflict | Selected package bytes and extracted compilation were both requested. |
| extraction-clause-count | Extracted compilation requires exactly one authored binding; details retain the actual count. |
| invalid-quire-context | Pinned Quire rejected the semantic context; details identify its contract versions and original diagnostics. |
| invalid_source_identity | Required source identity, revision or display path is absent. |
| invalid_source_map | Correspondence, source binding or queried range is invalid. |
| source_digest_mismatch | Admitted source bytes differ from the supplied raw SHA-256 digest. |
| invalid_utf8 | Source bytes cannot be decoded as UTF-8. |
| invalid_syntax | Malformed source, including forbidden source characters. |
| unsupported_construct | Recognized construct is outside the admitted grammar/profile. |
| unknown_language | The selected language label is not admitted. |
| unknown_edition | The selected edition label is not admitted. |
| unknown_profile | The selected profile label is not admitted. |
| invalid_package | Native package encoding, closed structure or reconstructed claims are invalid. |
| unknown_wire | The native package or runtime-input format selector is unsupported. |
| unknown_required_feature | A package feature is unknown or outside the consumer's declared support. |
| missing_import | A selected formal package or native import alias is absent. |
| stale_dependency | A known package has no exact selected revision and declaration byte digest. |
| ambiguous_declaration | Multiple exact candidates or visible exports match; related formal loci are retained. |
| missing_declaration | A required value, field, type or variant cannot be resolved. |
| invalid_model_binding | A digest, explicit context binding or clause name is invalid. |
| invalid_runtime_input | Native runtime input structure is invalid, including a local root/child outside its admitted arena ordering. |
| wrong_snapshot | A native result/pre expression has the wrong observation, or a runtime selection's artifact role, observation or invocation context/name/anchor disagrees with its checked clause. |
| dangling_reference | A runtime object target is absent from its complete required population. |
| incomplete_population | A required runtime population is absent or explicitly incomplete; missing targets cannot be inferred as dangling. |
| unavailable_observation | A selected runtime artifact or required State-root counterpart is unavailable. |
| population_delta_mismatch | Recorded created/deleted identities repeat, intersect or disagree with complete pre/post population differences. |
| frame_violation | A surviving object field or State root changes without permission, or a created/deleted object type is outside the model frame. |
| cancelled | The caller's runtime cancellation poll stopped validation or evaluation with actual prior usage. |
| runtime_invariant | Reference evaluation encountered a violated established typing or validated-input invariant; no Boolean is produced. |
| ill_typed | Native contextual types, nominal identities, units or operator eligibility disagree, or a scalar context is ambiguous. |
| undefined_expression | The actual IR prover could not establish a potentially evaluated operation's definedness under its preceding guards. |
| resource_exhausted | A source, syntax, formatter, map, linking, checking/proof, package or runtime construction/validation ceiling prevented completion. |
| ambiguous_dispatch | FR-151 dispatch linking found no, or several undominated, applicable candidates for a closed subtype. |

Phase identifies the observing boundary: source, lex, parse, profile, format or
source_map, link, check or validate. Related formal declaration locations are structured fields;
an upstream IR canonicalization or proof failure is retained in the optional upstream
field. Runtime diagnostics additionally retain the exact actual/expected artifact,
authored clause, observation and typed value path in `runtime`. Earlier phases
leave that field empty. These fields do not change the
existing syntax CLI output. Resource exhaustion is incomplete, never false. The CLI exits 1 for
native refusal, 3 for incompleteness and 2 for usage/I/O failures; usage/I/O text
does not pretend to be a source diagnostic. A successful parse exits 0 and
reports parsed, without model-linking or evaluation claims.

Validation reports keep each diagnostic's classification. Incomplete population,
unavailable observation, cancellation and exhausted work are incomplete. A known
invalid defect makes the overall report Refused even when another defect is
incomplete or detail capacity is zero. The optional terminal stop reason is
separate from the detail vector. Neither a failed report nor a successful
ValidatedContext contains predicate truth.

The native model profile resolves operation parameters only in the selected
operation and results through the result keyword. Early binding refusals use
the link phase; resolving an actual result in a precondition leaves its
post-only availability for the checker. Inventory identity conflicts identify
the conflicting model's source at byte zero and retain both models' related
formal declarations. Existing formal-profile linking keeps its original codes.
