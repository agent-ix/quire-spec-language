// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064-AC-9: source links that leave the base or loop.
#![cfg(unix)]

use std::os::unix::fs::symlink;

use ix_trace_rs::trace;
use qsl_string_edge::{scan, Error};

use crate::{request, Tree, CLEAN, VIOLATION};

/// A file link and a directory link whose targets lie outside the base are
/// errors naming the link, not silently followed.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn a_link_escaping_the_base_is_a_typed_error() {
    let outside = Tree::new();
    outside.write("elsewhere.rs", VIOLATION);
    outside.write("dir/lib.rs", VIOLATION);

    let file_link = Tree::new();
    file_link.write("src/lib.rs", CLEAN);
    let link = file_link.path().join("src/escape.rs");
    symlink(outside.path().join("elsewhere.rs"), &link).expect("symlink");
    let error = scan(&request(file_link.path(), &["src"])).expect_err("escaping file link");
    assert!(
        matches!(&error, Error::LinkEscapesBase { path } if *path == link),
        "{error:?}"
    );

    let dir_link = Tree::new();
    dir_link.write("src/lib.rs", CLEAN);
    let link = dir_link.path().join("src/escape");
    symlink(outside.path().join("dir"), &link).expect("symlink");
    let error = scan(&request(dir_link.path(), &["src"])).expect_err("escaping dir link");
    assert!(
        matches!(&error, Error::LinkEscapesBase { path } if *path == link),
        "{error:?}"
    );
}

/// A directory link back into one of its own ancestors is a cycle error; a
/// link to a sibling directory inside the base is followed and its source
/// is scanned under the link's own path.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn a_directory_link_cycle_is_an_error_and_an_inside_link_is_followed() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    let link = tree.path().join("src/again");
    symlink(tree.path().join("src"), &link).expect("symlink");
    let error = scan(&request(tree.path(), &["src"])).expect_err("cycle");
    assert!(
        matches!(&error, Error::LinkCycle { path } if *path == link),
        "{error:?}"
    );

    // A link to a directory outside every root but inside the base is followed
    // and reported under the link's own path.
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    tree.write("shared/lib.rs", VIOLATION);
    symlink(tree.path().join("shared"), tree.path().join("src/shared")).expect("symlink");
    let report = scan(&request(tree.path(), &["src"])).expect("scan");
    let files: Vec<_> = report.occurrences.iter().map(|o| o.file.as_str()).collect();
    assert_eq!(files, ["src/shared/lib.rs"]);
}

/// A link into another selected root's tree (or its own) would scan and
/// report the same source twice, so it is refused.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn a_link_into_a_selected_root_is_refused() {
    let tree = Tree::new();
    tree.write("a/src/lib.rs", CLEAN);
    tree.write("b/src/lib.rs", VIOLATION);
    let dir_link = tree.path().join("a/src/linked");
    symlink(tree.path().join("b/src"), &dir_link).expect("symlink");
    let error = scan(&request(tree.path(), &["a/src", "b/src"])).expect_err("dir link into root");
    assert!(
        matches!(&error, Error::LinkIntoRoot { path } if *path == dir_link),
        "{error:?}"
    );

    let tree = Tree::new();
    tree.write("a/src/lib.rs", CLEAN);
    tree.write("b/src/lib.rs", VIOLATION);
    let file_link = tree.path().join("a/src/twin.rs");
    symlink(tree.path().join("b/src/lib.rs"), &file_link).expect("symlink");
    let error = scan(&request(tree.path(), &["a/src", "b/src"])).expect_err("file link into root");
    assert!(
        matches!(&error, Error::LinkIntoRoot { path } if *path == file_link),
        "{error:?}"
    );

    // A link to a directory that contains another selected root.
    let tree = Tree::new();
    tree.write("a/src/lib.rs", CLEAN);
    tree.write("b/src/lib.rs", VIOLATION);
    let holder = tree.path().join("a/src/linkb");
    symlink(tree.path().join("b"), &holder).expect("symlink");
    let error = scan(&request(tree.path(), &["a/src", "b/src"])).expect_err("link to a holder");
    assert!(
        matches!(&error, Error::LinkIntoRoot { path } if *path == holder),
        "{error:?}"
    );

    let tree = Tree::new();
    tree.write("src/one/lib.rs", VIOLATION);
    tree.write("src/two/lib.rs", CLEAN);
    let sibling = tree.path().join("src/two/again");
    symlink(tree.path().join("src/one"), &sibling).expect("symlink");
    let error = scan(&request(tree.path(), &["src"])).expect_err("sibling link");
    assert!(
        matches!(&error, Error::LinkIntoRoot { path } if *path == sibling),
        "{error:?}"
    );
}

/// A dangling link named like a Rust file cannot be read, so the scan fails
/// rather than skipping it.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn a_dangling_rust_link_is_a_read_error() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    let link = tree.path().join("src/gone.rs");
    symlink(tree.path().join("absent.rs"), &link).expect("symlink");
    let error = scan(&request(tree.path(), &["src"])).expect_err("dangling");
    assert!(
        matches!(&error, Error::Read { path, .. } if *path == link),
        "{error:?}"
    );
}
