# Native diagnostic codes

FR-010 exposes one native `Diagnostic` envelope with phase, code, exact caller
source labels, display path, original span and explanatory message. `Display`
renders `code: message`; `std::error::Error` has no underlying cause. The public
`source` field is provenance, so the standard traits are implemented directly:
the pinned thiserror derive would incorrectly treat that field as an error cause.
The fixture-audit target retains its separate [audit codes](audit-error-codes.md).

`Code::all()` enumerates this vocabulary, `as_str()` supplies its stable spelling,
and `from_code()` returns `None` for unknown spellings. Codes are not renamed or
reused; prose may change without changing classification.

| Code | Meaning |
| --- | --- |
| invalid_source_identity | Required source identity, revision or display path is absent. |
| invalid_source_map | Correspondence, source binding or queried range is invalid. |
| source_digest_mismatch | Admitted source bytes differ from the supplied raw SHA-256 digest. |
| invalid_utf8 | Source bytes cannot be decoded as UTF-8. |
| invalid_syntax | Malformed source, including forbidden source characters. |
| unsupported_construct | Recognized construct is outside the admitted grammar/profile. |
| unknown_language | The selected language label is not admitted. |
| unknown_edition | The selected edition label is not admitted. |
| unknown_profile | The selected profile label is not admitted. |
| missing_import | A selected formal package or native import alias is absent. |
| stale_dependency | A known package has no exact selected revision and declaration byte digest. |
| ambiguous_declaration | Multiple exact candidates or visible exports match; related formal loci are retained. |
| missing_declaration | A required value, field, type or variant cannot be resolved. |
| invalid_model_binding | A digest, explicit context binding or clause name is invalid. |
| invalid_runtime_input | Native runtime input structure is invalid, including a local root/child outside its admitted arena ordering. |
| wrong_snapshot | A native invocation result lacks a selected operation binding, is accessed by its hidden IR name, or a checked result/pre expression is unavailable at its observation. |
| ill_typed | Native contextual types, nominal identities, units or operator eligibility disagree, or a scalar context is ambiguous. |
| undefined_expression | The actual IR prover could not establish a potentially evaluated operation's definedness under its preceding guards. |
| resource_exhausted | A source, syntax, formatter, map, linking, checking/proof or runtime-construction ceiling prevented completion. |

Phase identifies the observing boundary: source, lex, parse, profile, format or
source_map, link or check. Related formal declaration locations are structured fields;
an upstream IR canonicalization or proof failure is retained in the optional upstream
field. Legacy diagnostics leave both empty. These fields do not change the
existing syntax CLI output. Resource exhaustion is incomplete, never false. The CLI exits 1 for
native refusal, 3 for incompleteness and 2 for usage/I/O failures; usage/I/O text
does not pretend to be a source diagnostic. A successful parse exits 0 and
reports parsed, without model-linking or evaluation claims.

The native model profile resolves operation parameters only in the selected
operation and results through the result keyword. Early binding refusals use
the link phase; resolving an actual result in a precondition leaves its
post-only availability for the checker. Inventory identity conflicts identify
the conflicting model's source at byte zero and retain both models' related
formal declarations. Existing formal-profile linking keeps its original codes.
