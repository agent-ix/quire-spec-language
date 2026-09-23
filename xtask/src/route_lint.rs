// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-46 (FR-080-AC-3): `cargo xtask route-lint` -- scans the `#185`
//! registry module (`qsl-route/src/lib.rs`) for a `static`, `OnceLock`,
//! `thread_local!` or `lazy_static!` item and fails when it finds one.
//!
//! ADR-012 §5.3's registry evidence requires "a lint gate that finds no
//! `static`, `OnceLock` or `thread_local!` in the registry module". The
//! registry is specified to be an ordinary value, built by the
//! orchestrating driver and passed as an argument to every consumer
//! (FR-075 "The registry is an ordinary value, not ambient state"); any of
//! those constructs -- or `lazy_static!`, which this scan also catches
//! (PR #305 review, finding 11): its `static ref NAME: TYPE = EXPR;`
//! syntax is not valid Rust outside the macro invocation, so it never
//! reaches the plain `static` check and needs its own -- would let a
//! consumer read registrations through hidden global state instead, which
//! FR-075-CON-1 forbids. This scan catches that regression at the one
//! module FR-075 actually implements it in -- not the whole crate, so it
//! stays targeted at the seam FR-080 names, and not skipped, so ambient
//! state introduced in the registry module itself is actually caught
//! (TC-206's own two failure modes).

use std::fs;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::error::{Error, Result};

/// The registry module this gate scans, relative to the workspace root.
/// FR-080-AC-3's own scope: "the registry module (the module implementing
/// FR-075)".
pub const REGISTRY_MODULE_PATH: &str = "qsl-route/src/lib.rs";

/// One finding: a `static`, `OnceLock`-typed `static`, `thread_local!` or
/// `lazy_static!` item, named and located.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// What was found (`"static"`, `"OnceLock"`, `"thread_local!"` or
    /// `"lazy_static!"`).
    pub kind: &'static str,
    /// The item's own name, when it has one (a `static`'s identifier, or
    /// `"<thread_local! block>"`/`"<lazy_static! block>"` for a macro
    /// invocation, which syntactically may bind more than one name).
    pub name: String,
    /// 1-based source line the item starts on.
    pub line: u32,
}

struct AmbientStateVisitor {
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for AmbientStateVisitor {
    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        let type_mentions_once_lock = type_mentions_ident(&node.ty, "OnceLock");
        self.findings.push(Finding {
            kind: if type_mentions_once_lock {
                "OnceLock"
            } else {
                "static"
            },
            name: node.ident.to_string(),
            line: line_of(node),
        });
        syn::visit::visit_item_static(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Some(segment) = node.path.segments.last() {
            let (kind, name): (&'static str, &'static str) = if segment.ident == "thread_local" {
                ("thread_local!", "<thread_local! block>")
            } else if segment.ident == "lazy_static" {
                // `lazy_static!`'s `static ref NAME: TYPE = EXPR;` syntax
                // is not valid Rust `static` syntax on its own -- it
                // parses only inside the macro invocation, so it never
                // reaches `visit_item_static` above and needs its own
                // check here, the same way `thread_local!` does.
                ("lazy_static!", "<lazy_static! block>")
            } else {
                return syn::visit::visit_macro(self, node);
            };
            self.findings.push(Finding {
                kind,
                name: name.to_owned(),
                line: line_of(node),
            });
        }
        syn::visit::visit_macro(self, node);
    }
}

fn line_of<T: Spanned>(node: &T) -> u32 {
    u32::try_from(node.span().start().line).unwrap_or(u32::MAX)
}

/// Whether `ty` mentions `ident` anywhere in its type path (e.g. `OnceLock<T>`,
/// `std::sync::OnceLock<T>`, or a `OnceCell`-style alias someone renamed to
/// contain the substring is *not* detected here -- this is a syntactic
/// check over the literal written identifier).
fn type_mentions_ident(ty: &syn::Type, ident: &str) -> bool {
    struct IdentSeeker<'a> {
        target: &'a str,
        found: bool,
    }
    impl<'ast> Visit<'ast> for IdentSeeker<'_> {
        fn visit_ident(&mut self, node: &'ast proc_macro2::Ident) {
            if node == self.target {
                self.found = true;
            }
        }
    }
    let mut seeker = IdentSeeker {
        target: ident,
        found: false,
    };
    seeker.visit_type(ty);
    seeker.found
}

/// Scan `source` (already-read Rust source text) for `static`, `OnceLock`,
/// `thread_local!` and `lazy_static!` items. Pure function, used both by
/// the real gate (over `qsl-route/src/lib.rs` on disk) and by this module's own
/// tests (over literal injected source strings).
pub fn scan_source(source: &str) -> std::result::Result<Vec<Finding>, syn::Error> {
    let parsed = syn::parse_file(source)?;
    let mut visitor = AmbientStateVisitor {
        findings: Vec::new(),
    };
    visitor.visit_file(&parsed);
    Ok(visitor.findings)
}

/// `cargo xtask route-lint`: scan [`REGISTRY_MODULE_PATH`] and fail,
/// naming every finding, if it contains a `static`, `OnceLock`,
/// `thread_local!` or `lazy_static!` item.
pub fn run(workspace_root: &Path) -> Result<String> {
    let path: PathBuf = workspace_root.join(REGISTRY_MODULE_PATH);
    let source = fs::read_to_string(&path).map_err(|source| Error::io(path.clone(), source))?;
    let findings = scan_source(&source).map_err(|source| Error::RouteLintParse { path, source })?;
    if !findings.is_empty() {
        let summary = findings
            .iter()
            .map(|finding| {
                format!(
                    "{}:{} {} `{}`",
                    REGISTRY_MODULE_PATH, finding.line, finding.kind, finding.name
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Err(Error::RouteLintFound { summary });
    }
    Ok(format!(
        "route-lint: {REGISTRY_MODULE_PATH} has no static, OnceLock, thread_local! or \
         lazy_static! item.\n"
    ))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// TC-206 step 1: the unmodified module passes.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn the_real_registry_module_has_no_ambient_state() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask is one level under the workspace root");
        let outcome = run(workspace_root).expect("the real registry module must pass this gate");
        assert!(outcome.contains("no static, OnceLock, thread_local! or lazy_static!"));
    }

    /// TC-206 step 2: an injected plain `static` is found and named.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_an_injected_plain_static() {
        let findings = scan_source(
            r#"
            pub struct Registry;
            static REGISTRY: Registry = Registry;
            "#,
        )
        .unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "static");
        assert_eq!(findings[0].name, "REGISTRY");
    }

    /// TC-206 step 3: an injected `OnceLock`-typed `static` is found and
    /// distinguished from a plain `static`.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_an_injected_once_lock_static() {
        let findings = scan_source(
            r#"
            static REGISTRY: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
            "#,
        )
        .unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "OnceLock");
        assert_eq!(findings[0].name, "REGISTRY");
    }

    /// TC-206 step 4: an injected `thread_local!` block is found.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_an_injected_thread_local_block() {
        let findings = scan_source(
            r#"
            thread_local! {
                static REGISTRY: u8 = 0;
            }
            "#,
        )
        .unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "thread_local!");
    }

    /// PR #305 review, finding 11: `lazy_static!`'s `static ref NAME: TYPE
    /// = EXPR;` syntax parses only inside the macro invocation, so it never
    /// reaches `visit_item_static` and needs its own detection, the same
    /// way `thread_local!` does.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_an_injected_lazy_static_block() {
        let findings = scan_source(
            r#"
            lazy_static::lazy_static! {
                static ref REGISTRY: u8 = 0;
            }
            "#,
        )
        .unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "lazy_static!");
    }

    /// A lint rule that only caught `static` would pass an implementation
    /// using any of the others (TC-206's own stated failure mode); confirm
    /// all four are independently detected in one scan.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_all_four_kinds_in_one_module() {
        let findings = scan_source(
            r#"
            static PLAIN: u8 = 0;
            static LOCKED: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
            thread_local! {
                static LOCAL: u8 = 0;
            }
            lazy_static::lazy_static! {
                static ref LAZY: u8 = 0;
            }
            "#,
        )
        .unwrap();
        let kinds: Vec<&str> = findings.iter().map(|finding| finding.kind).collect();
        assert_eq!(
            kinds,
            ["static", "OnceLock", "thread_local!", "lazy_static!"]
        );
    }
}
