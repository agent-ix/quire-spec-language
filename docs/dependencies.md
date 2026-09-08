# Dependency and generated-artifact inventory

LC01 uses pinned Logos 0.16.1 for token recognition and serde_json 1.0.151
for JSON strings/CLI output. Source correspondence adds sha2 0.10.9 for exact
byte integrity. Cargo.lock fixes the transitive resolution below.
This records package metadata, not a public distribution notice bundle.
Existing dependency grants are preserved. Release remains a separate review.

| Package | Version | Declared grant |
|---|---|---|
| aho-corasick | 1.1.5 | `Unlicense OR MIT` |
| bitflags | 2.13.1 | `MIT OR Apache-2.0` |
| block-buffer | 0.10.4 | `MIT OR Apache-2.0` |
| cfg-if | 1.0.4 | `MIT OR Apache-2.0` |
| cpufeatures | 0.2.17 | `MIT OR Apache-2.0` |
| crypto-common | 0.1.7 | `MIT OR Apache-2.0` |
| digest | 0.10.7 | `MIT OR Apache-2.0` |
| errno | 0.3.14 | `MIT OR Apache-2.0` |
| fastrand | 2.5.0 | `Apache-2.0 OR MIT` |
| fnv | 1.0.7 | `Apache-2.0 / MIT` |
| generic-array | 0.14.7 | `MIT` |
| getrandom | 0.4.3 | `MIT OR Apache-2.0` |
| itoa | 1.0.18 | `MIT OR Apache-2.0` |
| ix-trace-rs | 0.1.0, Git 2ce4ebf47f726b9d76388220545cd0abda8a5cfb | `AGPL-3.0-or-later` |
| libc | 0.2.189 | `MIT OR Apache-2.0` |
| linux-raw-sys | 0.12.1 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` |
| logos | 0.16.1 | `MIT OR Apache-2.0` |
| logos-codegen | 0.16.1 | `MIT OR Apache-2.0` |
| logos-derive | 0.16.1 | `MIT OR Apache-2.0` |
| memchr | 2.8.3 | `Unlicense OR MIT` |
| once_cell | 1.21.4 | `MIT OR Apache-2.0` |
| proc-macro2 | 1.0.107 | `MIT OR Apache-2.0` |
| quote | 1.0.47 | `MIT OR Apache-2.0` |
| r-efi | 6.0.0 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` |
| regex-automata | 0.4.18 | `MIT OR Apache-2.0` |
| regex-syntax | 0.8.11 | `MIT OR Apache-2.0` |
| rustix | 1.1.4 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` |
| serde | 1.0.229 | `MIT OR Apache-2.0` |
| serde_core | 1.0.229 | `MIT OR Apache-2.0` |
| serde_derive | 1.0.229 | `MIT OR Apache-2.0` |
| serde_json | 1.0.151 | `MIT OR Apache-2.0` |
| sha2 | 0.10.9 | `MIT OR Apache-2.0` |
| syn | 2.0.119 | `MIT OR Apache-2.0` |
| syn | 3.0.5 | `MIT OR Apache-2.0` |
| tempfile | 3.27.0 | `MIT OR Apache-2.0` |
| thiserror | 2.0.20 | `MIT OR Apache-2.0` |
| thiserror-impl | 2.0.20 | `MIT OR Apache-2.0` |
| typenum | 1.20.1 | `MIT OR Apache-2.0` |
| unicode-ident | 1.0.24 | `(MIT OR Apache-2.0) AND Unicode-3.0` |
| version_check | 0.9.5 | `MIT/Apache-2.0` |
| windows-link | 0.2.1 | `MIT OR Apache-2.0` |
| windows-sys | 0.61.2 | `MIT OR Apache-2.0` |
| zmij | 1.0.23 | `MIT` |

Registry packages offer MIT, with unicode-ident additionally requiring its
Unicode-3.0 grant. The shared Git dependency ix-trace-rs retains AGPL-3.0-or-later.
The legacy `Apache-2.0 / MIT` fnv spelling is retained above
as declared by its manifest; version_check likewise uses legacy `MIT/Apache-2.0`. Original notice/license files stay in dependency
packages. No existing project code or external parser corpus is copied.
New test fixtures and the declarative token specification are authored here
under AGPL-3.0-only. The Logos derive macro generates recognizer code at build
time; generated build files are not checked in. A shipped binary/package needs
its own included-artifact and notices inventory before publication.

The LR02 audit uses serde 1.0.229 and thiserror 2.0.20 directly; the latter's
derive implementation is compile-time code. tempfile 3.27.0 and ix-trace-rs are
dev-dependencies. The shared marker has no runtime dependencies and expands
its item unchanged after validating arguments. Its repository tag is v0.1.1
while the pinned Cargo package declares version 0.1.0. Cargo metadata from the
locked offline resolution supplies all 43 dependency rows above, including
target-specific packages; this inventory does not imply those targets ran.

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
