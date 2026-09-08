# Synthetic typed-model producer fixture

Newly authored source and manifest under AGPL-3.0-only. This uses the existing
Filament TypeSpec frontend to produce real bounded declaration/package bytes;
it is not the spec-bundle extraction frontend or the contract-IR typed-model
adapter. Native source obligations remain separately authored and are not copied
into model clause text. No operation frame is inferred from an empty clause list.

The fixture is an unqualified model input until the shared adapter and source/
operation binding are reviewed. It must not replace the older intentionally
unresolved native import as though linkage were already established.

Generation uses the existing Filament compiler and its pinned TypeSpec 1.15.0
(MIT) build toolchain; no TypeSpec dependency is added to the Rust runtime.
Exact producer revision, commands, output digests and remaining qualification
limits are recorded with the generated artifact checkpoint.
