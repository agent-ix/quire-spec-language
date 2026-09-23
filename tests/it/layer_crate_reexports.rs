// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §7.2 (QSL-181 AC-3, QSL-182 AC-3): the root crate re-exports no
//! item of an extracted layer crate. Callers name each item at its layer
//! crate's own path (`qsl_package::CheckedPackage`,
//! `qsl_semantics::check::CheckedExpression`), so this crate's public
//! surface never becomes a second path to a moved item.
//!
//! A `syn` scan of every `.rs` file under `src/`: a `pub use` item whose
//! path is rooted at a workspace layer crate fails it, whatever the shape of
//! its use tree. A private or `pub(crate)`/`pub(super)` `use` of a layer
//! crate is an import inside this crate, not part of its public surface, and
//! passes.
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

/// The extracted ADR-011 §6.1 layer crates, by Rust crate name.
const LAYER_CRATES: [&str; 7] = [
    "quire_exact",
    "qsl_foundation",
    "qsl_cst",
    "qsl_forms",
    "qsl_semantics",
    "qsl_package",
    "qsl_replay",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("the source tree lists") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// The first path segment of each leaf of `tree`: a `use a::{b, c::d}`
/// yields `a` once per leaf, and a leading `::` is ignored.
fn roots(tree: &syn::UseTree, root: Option<&str>, out: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            let first = root.map_or_else(|| path.ident.to_string(), str::to_owned);
            roots(&path.tree, Some(&first), out);
        }
        syn::UseTree::Name(name) => {
            out.push(root.map_or_else(|| name.ident.to_string(), str::to_owned))
        }
        syn::UseTree::Rename(rename) => {
            out.push(root.map_or_else(|| rename.ident.to_string(), str::to_owned));
        }
        syn::UseTree::Glob(_) => out.extend(root.map(str::to_owned)),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                roots(item, root, out);
            }
        }
    }
}

/// Every `pub use` item in `items`, recursing into inline modules.
fn reexports(items: &[syn::Item], file: &str, out: &mut Vec<(String, String)>) {
    for item in items {
        match item {
            syn::Item::Use(item_use) if matches!(item_use.vis, syn::Visibility::Public(_)) => {
                let mut found = Vec::new();
                roots(&item_use.tree, None, &mut found);
                for root in found {
                    out.push((file.to_owned(), root));
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, nested)) = &module.content {
                    reexports(nested, file, out);
                }
            }
            _ => {}
        }
    }
}

fn layer_crate_reexports(src: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    rust_files(src, &mut files);
    files.sort();
    let mut found = Vec::new();
    for path in files {
        let source = std::fs::read_to_string(&path).expect("a source file reads");
        let parsed =
            syn::parse_file(&source).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let relative = path
            .strip_prefix(src)
            .expect("under src")
            .to_string_lossy()
            .replace('\\', "/");
        reexports(&parsed.items, &relative, &mut found);
    }
    found
        .into_iter()
        .filter(|(_, root)| LAYER_CRATES.contains(&root.as_str()))
        .collect()
}

/// TC-256 step 3 and TC-172 step 6 as amended by QSL-182 and QSL-181:
/// `value::expression` re-exports neither `CheckedPackage` nor any `check`
/// item, and no other root-crate module re-exports a layer crate's item.
/// The scanner itself is checked against a planted re-export of each shape.
#[trace("FR-087-AC-9", "TC-256", "TC-172")]
#[test]
fn the_root_crate_reexports_no_layer_crate_item() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let found = layer_crate_reexports(&src);
    assert!(
        found.is_empty(),
        "root-crate re-exports of layer-crate items: {found:?}"
    );

    let planted = syn::parse_file(
        "use qsl_package::CheckedPackage;\n\
         pub use qsl_package::CheckedPackage;\n\
         pub use ::qsl_semantics::{check::CheckedExpression, library};\n\
         pub(crate) use qsl_package::EmittedPackage;\n\
         mod inner { pub use qsl_package::*; }\n\
         pub use serde_json::Value;\n",
    )
    .expect("the fixture parses");
    let mut planted_found = Vec::new();
    reexports(&planted.items, "fixture.rs", &mut planted_found);
    let roots: Vec<&str> = planted_found
        .iter()
        .map(|(_, root)| root.as_str())
        .filter(|root| LAYER_CRATES.contains(root))
        .collect();
    assert_eq!(
        roots,
        [
            "qsl_package",
            "qsl_semantics",
            "qsl_semantics",
            "qsl_package"
        ]
    );
}
