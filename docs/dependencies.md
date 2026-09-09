# Dependency and generated-artifact inventory

LC04 adds qualification-only codegen `240fad84a9565ab723ba9844e18faea4e5d96f66`
and its IR `04eb6f849c03be23177d373549c6c272551f957d`, both MIT OR Apache-2.0.
The latter has a dev-only alias for the existing consumer's exact wire reader;
production IR remains `690bde7`. Generated Rust retains codegen's MIT/Apache
notices and compiles against its runtime `8a4d02b9ff4633cf6d02fd8bdf6ee1b11ad76354`
(MIT OR Apache-2.0). Syn 2.0.119 (MIT OR Apache-2.0), already resolved transitively,
is directly selected for Rust syntax inspection in the qualification driver.
New driver code is Rust under AGPL-3.0-only. No producer source is copied.

The LC04 lock selects 140 packages including this crate. Cargo metadata
supplies the complete [name/version/source/license snapshot](dependency-licenses.json).
The public IR dependency is pinned to 690bde7f2dc58662cf9ff0595c2c0e3b17107c6f;
ix-trace-rs remains pinned to 2ce4ebf47f726b9d76388220545cd0abda8a5cfb.
Serde 1.0.228 is selected consistently with IR's exact dependency.

FR-020 selects the reviewed `unbounded_depth` feature on the existing serde_json
pin. Only package intake disables the library recursion guard; its metered
traversal enforces the 128-container hard ceiling. Other decoders retain their
guards. No package version or grant changes.

FR-017 enables the existing pinned serde_json 1.0.151 `raw_value` feature in
the development dependency for source-aware Rust fixture decoding. Borrowed
values retain their original occurrences; Serde continues to own JSON grammar.
The feature adds no package or copied source and preserves serde_json's
`MIT OR Apache-2.0` grant. Cargo.lock is unchanged. The authored adapter and
regression fixtures retain AGPL-3.0-only.

The initial FR-019/021 package producer uses existing serde/serde_json Formatter
hooks and sha2 with unchanged versions, features and grants. Its production,
fixture composition, assertions and hard-limit controls are Rust under
AGPL-3.0-only. No new dependency or executable audit helper is introduced.
The follow-up qualification selects the already locked MIT jsonschema 0.17.1
as a direct development dependency with default features disabled and only
draft202012 enabled. Cargo metadata confirms those resolved features: no HTTP,
file or CLI resolver feature is enabled. The lock changes only the root
development dependency edge; the 138-package/version/grant inventory remains
unchanged. Tests compile the local Draft 2020-12 schema and exercise fixed and
real producer manifests plus structural adverse controls. Semantic readback
remains Task-017.

The Rust `author_native_package_vectors` maintenance example produces candidate
test data from the independently authored package recipe and upstream model
admission. Its fixture generation is domain-specific; it does not duplicate a
shared evidence engine or call the package encoder under test. It creates a
fresh output directory and requires deliberate review/promotion. Provenance,
fixed digests and the local command are recorded in
tests/fixtures/native-package/README.md. This adds no non-Rust executable path.

This is declared metadata, not a public distribution notice bundle or a
vulnerability scan. Existing grants remain intact, including ICU Unicode-3.0
and ar_archive_writer Apache-2.0 WITH LLVM-exception. The dependency stacker/psm
build uses existing third-party native build tools; no new first-party C,
Python, JavaScript or shell qualification helper is introduced.

| Package | Version | Declared grant |
| --- | --- | --- |
| ahash | 0.8.12 | `MIT OR Apache-2.0` |
| aho-corasick | 1.1.5 | `Unlicense OR MIT` |
| anyhow | 1.0.104 | `MIT OR Apache-2.0` |
| ar_archive_writer | 0.5.3 | `Apache-2.0 WITH LLVM-exception` |
| autocfg | 1.5.1 | `Apache-2.0 OR MIT` |
| base64 | 0.21.7 | `MIT OR Apache-2.0` |
| bit-set | 0.5.3 | `MIT/Apache-2.0` |
| bit-vec | 0.6.3 | `MIT/Apache-2.0` |
| bitflags | 2.13.1 | `MIT OR Apache-2.0` |
| block-buffer | 0.10.4 | `MIT OR Apache-2.0` |
| bumpalo | 3.20.3 | `MIT OR Apache-2.0` |
| bytecount | 0.6.9 | `Apache-2.0/MIT` |
| cc | 1.4.5 | `MIT OR Apache-2.0` |
| cfg-if | 1.0.4 | `MIT OR Apache-2.0` |
| cpufeatures | 0.2.17 | `MIT OR Apache-2.0` |
| crypto-common | 0.1.7 | `MIT OR Apache-2.0` |
| deranged | 0.5.8 | `MIT OR Apache-2.0` |
| digest | 0.10.7 | `MIT OR Apache-2.0` |
| displaydoc | 0.2.7 | `MIT OR Apache-2.0` |
| errno | 0.3.14 | `MIT OR Apache-2.0` |
| fancy-regex | 0.11.0 | `MIT` |
| fastrand | 2.5.0 | `Apache-2.0 OR MIT` |
| find-msvc-tools | 0.1.12 | `MIT OR Apache-2.0` |
| fnv | 1.0.7 | `Apache-2.0 / MIT` |
| form_urlencoded | 1.2.2 | `MIT OR Apache-2.0` |
| fraction | 0.13.1 | `MIT/Apache-2.0` |
| futures-core | 0.3.34 | `MIT OR Apache-2.0` |
| futures-task | 0.3.34 | `MIT OR Apache-2.0` |
| futures-util | 0.3.34 | `MIT OR Apache-2.0` |
| generic-array | 0.14.7 | `MIT` |
| getrandom | 0.2.17 | `MIT OR Apache-2.0` |
| getrandom | 0.3.4 | `MIT OR Apache-2.0` |
| getrandom | 0.4.3 | `MIT OR Apache-2.0` |
| icu_collections | 2.3.0 | `Unicode-3.0` |
| icu_locale_core | 2.3.0 | `Unicode-3.0` |
| icu_normalizer | 2.3.0 | `Unicode-3.0` |
| icu_normalizer_data | 2.3.0 | `Unicode-3.0` |
| icu_properties | 2.3.0 | `Unicode-3.0` |
| icu_properties_data | 2.3.0 | `Unicode-3.0` |
| icu_provider | 2.3.1 | `Unicode-3.0` |
| idna | 1.1.0 | `MIT OR Apache-2.0` |
| idna_adapter | 1.2.2 | `Apache-2.0 OR MIT` |
| iso8601 | 0.6.5 | `MIT` |
| itoa | 1.0.18 | `MIT OR Apache-2.0` |
| ix-trace-rs | 0.1.0 | `AGPL-3.0-or-later` |
| js-sys | 0.3.105 | `MIT OR Apache-2.0` |
| jsonschema | 0.17.1 | `MIT` |
| lazy_static | 1.5.0 | `MIT OR Apache-2.0` |
| libc | 0.2.189 | `MIT OR Apache-2.0` |
| linux-raw-sys | 0.12.1 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` |
| litemap | 0.8.3 | `Unicode-3.0` |
| lock_api | 0.4.14 | `MIT OR Apache-2.0` |
| logos | 0.16.1 | `MIT OR Apache-2.0` |
| logos-codegen | 0.16.1 | `MIT OR Apache-2.0` |
| logos-derive | 0.16.1 | `MIT OR Apache-2.0` |
| memchr | 2.8.3 | `Unlicense OR MIT` |
| nom | 8.0.0 | `MIT` |
| num | 0.4.3 | `MIT OR Apache-2.0` |
| num-bigint | 0.4.8 | `MIT OR Apache-2.0` |
| num-cmp | 0.1.0 | `MIT/Apache-2.0` |
| num-complex | 0.4.6 | `MIT OR Apache-2.0` |
| num-conv | 0.2.2 | `MIT OR Apache-2.0` |
| num-integer | 0.1.47 | `MIT OR Apache-2.0` |
| num-iter | 0.1.46 | `MIT OR Apache-2.0` |
| num-rational | 0.4.2 | `MIT OR Apache-2.0` |
| num-traits | 0.2.19 | `MIT OR Apache-2.0` |
| object | 0.39.1 | `Apache-2.0 OR MIT` |
| once_cell | 1.21.4 | `MIT OR Apache-2.0` |
| parking_lot | 0.12.5 | `MIT OR Apache-2.0` |
| parking_lot_core | 0.9.12 | `MIT OR Apache-2.0` |
| percent-encoding | 2.3.2 | `MIT OR Apache-2.0` |
| pin-project-lite | 0.2.17 | `Apache-2.0 OR MIT` |
| potential_utf | 0.1.6 | `Unicode-3.0` |
| powerfmt | 0.2.0 | `MIT OR Apache-2.0` |
| proc-macro2 | 1.0.107 | `MIT OR Apache-2.0` |
| psm | 0.1.32 | `MIT OR Apache-2.0` |
| quire-contract-ir | 0.1.0 | `MIT OR Apache-2.0` |
| quire-contract-codegen | 0.1.0 | `MIT OR Apache-2.0` |
| quire-spec-language | 0.1.0 | `AGPL-3.0-only` |
| quote | 1.0.47 | `MIT OR Apache-2.0` |
| r-efi | 5.3.0 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` |
| r-efi | 6.0.0 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` |
| redox_syscall | 0.5.18 | `MIT` |
| regex | 1.13.1 | `MIT OR Apache-2.0` |
| regex-automata | 0.4.18 | `MIT OR Apache-2.0` |
| regex-syntax | 0.8.11 | `MIT OR Apache-2.0` |
| rustix | 1.1.4 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` |
| rustversion | 1.0.23 | `MIT OR Apache-2.0` |
| scopeguard | 1.2.0 | `MIT OR Apache-2.0` |
| serde | 1.0.228 | `MIT OR Apache-2.0` |
| serde_core | 1.0.228 | `MIT OR Apache-2.0` |
| serde_derive | 1.0.228 | `MIT OR Apache-2.0` |
| serde_json | 1.0.151 | `MIT OR Apache-2.0` |
| serde_stacker | 0.1.11 | `MIT OR Apache-2.0` |
| sha2 | 0.10.9 | `MIT OR Apache-2.0` |
| shlex | 2.0.1 | `MIT OR Apache-2.0` |
| slab | 0.4.12 | `MIT` |
| smallvec | 1.16.0 | `MIT OR Apache-2.0` |
| stable_deref_trait | 1.2.1 | `MIT OR Apache-2.0` |
| stacker | 0.1.15 | `MIT OR Apache-2.0` |
| syn | 2.0.119 | `MIT OR Apache-2.0` |
| syn | 3.0.5 | `MIT OR Apache-2.0` |
| synstructure | 0.13.2 | `MIT` |
| tempfile | 3.27.0 | `MIT OR Apache-2.0` |
| thiserror | 2.0.20 | `MIT OR Apache-2.0` |
| thiserror-impl | 2.0.20 | `MIT OR Apache-2.0` |
| time | 0.3.55 | `MIT OR Apache-2.0` |
| time-core | 0.1.9 | `MIT OR Apache-2.0` |
| time-macros | 0.2.32 | `MIT OR Apache-2.0` |
| tinystr | 0.8.4 | `Unicode-3.0` |
| typenum | 1.20.1 | `MIT OR Apache-2.0` |
| unicode-ident | 1.0.24 | `(MIT OR Apache-2.0) AND Unicode-3.0` |
| url | 2.5.8 | `MIT OR Apache-2.0` |
| utf8_iter | 1.0.4 | `Apache-2.0 OR MIT` |
| uuid | 1.26.0 | `Apache-2.0 OR MIT` |
| version_check | 0.9.5 | `MIT/Apache-2.0` |
| wasi | 0.11.1+wasi-snapshot-preview1 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` |
| wasip2 | 1.0.4+wasi-0.2.12 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` |
| wasm-bindgen | 0.2.128 | `MIT OR Apache-2.0` |
| wasm-bindgen-macro | 0.2.128 | `MIT OR Apache-2.0` |
| wasm-bindgen-macro-support | 0.2.128 | `MIT OR Apache-2.0` |
| wasm-bindgen-shared | 0.2.128 | `MIT OR Apache-2.0` |
| winapi | 0.3.9 | `MIT/Apache-2.0` |
| winapi-i686-pc-windows-gnu | 0.4.0 | `MIT/Apache-2.0` |
| winapi-x86_64-pc-windows-gnu | 0.4.0 | `MIT/Apache-2.0` |
| windows-link | 0.2.1 | `MIT OR Apache-2.0` |
| windows-sys | 0.61.2 | `MIT OR Apache-2.0` |
| wit-bindgen | 0.57.1 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` |
| writeable | 0.6.4 | `Unicode-3.0` |
| yoke | 0.8.3 | `Unicode-3.0` |
| yoke-derive | 0.8.2 | `Unicode-3.0` |
| zerocopy | 0.8.57 | `BSD-2-Clause OR Apache-2.0 OR MIT` |
| zerocopy-derive | 0.8.57 | `BSD-2-Clause OR Apache-2.0 OR MIT` |
| zerofrom | 0.1.8 | `Unicode-3.0` |
| zerofrom-derive | 0.1.7 | `Unicode-3.0` |
| zerotrie | 0.2.5 | `Unicode-3.0` |
| zerovec | 0.11.8 | `Unicode-3.0` |
| zerovec-derive | 0.11.6 | `Unicode-3.0` |
| zmij | 1.0.23 | `MIT` |

New native source, tests and declarative token definitions are AGPL-3.0-only.
The imported IR remains MIT OR Apache-2.0, and ix-trace-rs AGPL-3.0-or-later.
Logos and thiserror derive generate Rust at compile time; generated build files
are not checked in. Original dependency notices remain in their packages.
Target-specific packages in the inventory do not imply those targets were run.
No external parser corpus or model implementation was copied.

LC03's input constructors explicitly enable serde's existing derive feature on
the direct pinned serde 1.0.228 dependency. The pinned IR already enabled that
feature, so Cargo.lock and the resolved package/grant inventory are unchanged.
The authored runtime modules and Rust tests are AGPL-3.0-only; serde derives
generate only build output, and no external input reader was copied or added.

## Optional model producer toolchain

The original `tests/fixtures/model-source` TypeSpec source/manifest and the
generated `model-output` records are implementation fixtures under AGPL-3.0-only.
They contain this fixture's declarations and producer metadata; no external
model corpus or generated TypeSpec implementation is copied. Provenance pins
the existing Filament producer to `3b75e01c652ba00bb07c352ff5467419401e792b`,
TypeSpec compiler 1.15.0 (MIT), and the observed Node v22.15.0 toolchain. The
existing Filament checkout and its dependency notices remain under their
existing grants. Fresh use of its installed toolchain awaits owner disposition;
this change does not import its JavaScript dependency graph into the Rust crate.

All four Python helpers have been replaced by the Rust fixture-audit target.
Local checks verify stored artifact bytes without Python, Node, TypeSpec or a
private sibling repository; the explicitly selected private packet lane adds
real review/roles/syntax checks. Hosted CI is manual-dispatch only. Model-producer
refuses fresh production under the campaign language gate. Historical provenance
keeps its original command strings and exact fixture copies retain their
implementation-fixture grant.
