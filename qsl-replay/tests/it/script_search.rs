// SPDX-License-Identifier: AGPL-3.0-or-later
//! A solver-free reading of a script: satisfiability by search over small
//! domains, so a test can tell an unsat query from a sat one.

use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

pub fn parse(text: &str) -> Vec<Sexp> {
    let spaced = text.replace('(', " ( ").replace(')', " ) ");
    let mut stack: Vec<Vec<Sexp>> = vec![Vec::new()];
    for token in spaced.split_whitespace() {
        match token {
            "(" => stack.push(Vec::new()),
            ")" => {
                let done = stack.pop().expect("balanced");
                stack.last_mut().expect("balanced").push(Sexp::List(done));
            }
            atom => stack
                .last_mut()
                .expect("balanced")
                .push(Sexp::Atom(atom.to_owned())),
        }
    }
    stack.pop().expect("balanced")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Val {
    Bool(bool),
    Int(i64),
}

fn constants(term: &Sexp, names: &BTreeMap<String, usize>, found: &mut Vec<usize>) {
    match term {
        Sexp::Atom(name) => {
            if let Some(index) = names.get(name) {
                found.push(*index);
            }
        }
        Sexp::List(items) => items.iter().for_each(|item| constants(item, names, found)),
    }
}

fn eval(term: &Sexp, names: &BTreeMap<String, usize>, env: &[Val]) -> Val {
    let int = |value: Val| match value {
        Val::Int(value) => value,
        Val::Bool(_) => panic!("an integer was expected"),
    };
    let boolean = |value: Val| match value {
        Val::Bool(value) => value,
        Val::Int(_) => panic!("a Boolean was expected"),
    };
    match term {
        Sexp::Atom(atom) => match atom.as_str() {
            "true" => Val::Bool(true),
            "false" => Val::Bool(false),
            _ => match names.get(atom) {
                Some(index) => env[*index],
                None => Val::Int(atom.parse().expect("a numeral")),
            },
        },
        Sexp::List(items) => {
            let Some(Sexp::Atom(head)) = items.first() else {
                panic!("an application")
            };
            let operands: Vec<Val> = items[1..]
                .iter()
                .map(|item| eval(item, names, env))
                .collect();
            match (head.as_str(), operands.as_slice()) {
                ("not", [only]) => Val::Bool(!boolean(*only)),
                ("and", all) => Val::Bool(all.iter().all(|value| boolean(*value))),
                ("or", all) => Val::Bool(all.iter().any(|value| boolean(*value))),
                ("=>", [left, right]) => Val::Bool(!boolean(*left) || boolean(*right)),
                ("=", [left, right]) => Val::Bool(left == right),
                ("<=", [left, right]) => Val::Bool(int(*left) <= int(*right)),
                ("<", [left, right]) => Val::Bool(int(*left) < int(*right)),
                (">=", [left, right]) => Val::Bool(int(*left) >= int(*right)),
                (">", [left, right]) => Val::Bool(int(*left) > int(*right)),
                ("+", all) => Val::Int(all.iter().map(|value| int(*value)).sum()),
                ("-", [only]) => Val::Int(-int(*only)),
                ("-", [left, right]) => Val::Int(int(*left) - int(*right)),
                ("*", [left, right]) => Val::Int(int(*left) * int(*right)),
                ("mod", [left, right]) => Val::Int(int(*left).rem_euclid(int(*right))),
                ("ite", [test, then, other]) => {
                    if boolean(*test) {
                        *then
                    } else {
                        *other
                    }
                }
                other => panic!("an unknown operator {other:?}"),
            }
        }
    }
}

/// The values an Integer constant ranges over in the search below. Every
/// ranged constant of the fixtures lies inside it (`Int[0, 3]`), so an
/// unsatisfiable answer is exact for a script whose Integers are all ranged
/// ([`assert_unsat`] checks that); a satisfiable answer is a witness for any.
pub const SEARCH_VALUES: std::ops::Range<i64> = 0..8;

/// Whether some assignment of the declared constants satisfies the script,
/// Integers ranging over [`SEARCH_VALUES`].
pub fn satisfiable(script: &str) -> bool {
    let mut names = BTreeMap::new();
    let mut kinds = Vec::new();
    let mut assertions = Vec::new();
    for command in parse(script) {
        let Sexp::List(items) = command else { continue };
        match items.as_slice() {
            [Sexp::Atom(head), Sexp::Atom(name), Sexp::Atom(sort)] if head == "declare-const" => {
                names.insert(name.clone(), kinds.len());
                kinds.push(sort == "Bool");
            }
            [Sexp::Atom(head), term] if head == "assert" => assertions.push(term.clone()),
            _ => {}
        }
    }
    // Check each assertion as soon as its last constant is assigned.
    let mut by_last: Vec<Vec<&Sexp>> = vec![Vec::new(); kinds.len() + 1];
    for assertion in &assertions {
        let mut found = Vec::new();
        constants(assertion, &names, &mut found);
        by_last[found.into_iter().max().map_or(0, |last| last + 1)].push(assertion);
    }
    fn search(
        at: usize,
        kinds: &[bool],
        names: &BTreeMap<String, usize>,
        by_last: &[Vec<&Sexp>],
        env: &mut Vec<Val>,
    ) -> bool {
        if !by_last[at]
            .iter()
            .all(|term| eval(term, names, env) == Val::Bool(true))
        {
            return false;
        }
        if at == kinds.len() {
            return true;
        }
        let domain: Vec<Val> = if kinds[at] {
            vec![Val::Bool(false), Val::Bool(true)]
        } else {
            SEARCH_VALUES.map(Val::Int).collect()
        };
        for value in domain {
            env.push(value);
            let found = search(at + 1, kinds, names, by_last, env);
            env.pop();
            if found {
                return true;
            }
        }
        false
    }
    search(0, &kinds, &names, &by_last, &mut Vec::new())
}
/// The assertion `term` printed as the canonical printer prints it.
pub fn render(term: &Sexp) -> String {
    match term {
        Sexp::Atom(atom) => atom.clone(),
        Sexp::List(items) => format!(
            "({})",
            items.iter().map(render).collect::<Vec<_>>().join(" ")
        ),
    }
}

/// The assertions of `script`, each printed, without `(assert ...)`.
pub fn assertions(script: &str) -> Vec<String> {
    parse(script)
        .into_iter()
        .filter_map(|command| match command {
            Sexp::List(items) => match items.as_slice() {
                [Sexp::Atom(head), term] if head == "assert" => Some(render(term)),
                _ => None,
            },
            Sexp::Atom(_) => None,
        })
        .collect()
}

/// The operands of the application printed as `term`.
pub fn operands(term: &str) -> Vec<String> {
    match parse(term).into_iter().next() {
        Some(Sexp::List(items)) => items.iter().skip(1).map(render).collect(),
        _ => Vec::new(),
    }
}

/// The script is unsatisfiable, and the search is exact for it: every
/// Integer constant it declares carries a range inside the searched values.
pub fn assert_unsat(script: &str) {
    let max = SEARCH_VALUES.end - 1;
    for line in script.lines() {
        if let Some(rest) = line.strip_prefix("(declare-const ") {
            if let Some(name) = rest.strip_suffix(" Int)") {
                let ranged = (0..=max).any(|upper| {
                    script.lines().any(|candidate| {
                        candidate == format!("(assert (and (<= 0 {name}) (<= {name} {upper})))")
                    })
                });
                assert!(ranged, "`{name}` is searched over 0..{max} but not ranged");
            }
        }
    }
    assert!(!satisfiable(script));
}
