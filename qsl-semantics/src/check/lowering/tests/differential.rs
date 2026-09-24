// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-221: lowering keys each distinct node content once, and the keys and
//! preimage bytes of every node are the ones full keying gives.
//!
//! The generated corpus mixes the shapes whose nodes repeat (every function's
//! parameter, type and literal nodes), call chains, self and mutual recursion
//! (drafts and recursion groups), and a recursive record compared in several
//! functions (a group member built again after its group is keyed).

use sha2::{Digest, Sha256};

use super::*;

/// How many packages the corpus holds.
const PACKAGES: u64 = 192;

/// SHA-256 over every generated package's nodes, in package order and, in
/// each package, ascending by key: the key's 32 bytes, the preimage's length
/// as a big-endian `u64`, then the preimage. Recorded from the lowering
/// before QSL-221, which keyed every node in full on every build (at
/// 0f7db2d6, and again at 4ba5c0ce after QSL-233's source owners; the
/// revisions are informational). A change that means to change
/// a key records the new digest with the vectors it changes.
const CORPUS_DIGEST: &str = "3ea3f638522493bb559bb1194bffb03eface9d9b74c9de7795f3b78d42b4fa9a";

/// A deterministic xorshift64 stream: the corpus is the same on every run.
struct Stream(u64);

impl Stream {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A value below `bound`.
    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn index(&mut self, bound: usize) -> usize {
        usize::try_from(self.below(u64::try_from(bound).expect("small bound")))
            .expect("below a usize bound")
    }
}

/// A Boolean expression over `x: Int[0, 9]` and `b: Boolean`, nested at most
/// `depth` deep. A call to `callees[i]` is guarded by `x > 0` and passes
/// `x - 1`, so every call decreases the measure `x`.
fn boolean_expression(stream: &mut Stream, depth: u32, callees: &[String]) -> Expression {
    let leaf = depth == 0 || stream.below(4) == 0;
    if leaf {
        return match stream.below(4) {
            0 => Expression::Boolean(stream.below(2) == 0),
            1 => name_expr("b"),
            2 => binary(
                BinaryOperator::Greater,
                name_expr("x"),
                integer_expr(i64::try_from(stream.below(9)).expect("small")),
            ),
            _ => binary(
                BinaryOperator::Equal,
                name_expr("x"),
                integer_expr(i64::try_from(stream.below(9)).expect("small")),
            ),
        };
    }
    match stream.below(5) {
        0 => binary(
            BinaryOperator::And,
            boolean_expression(stream, depth - 1, callees),
            boolean_expression(stream, depth - 1, callees),
        ),
        1 => binary(
            BinaryOperator::Or,
            boolean_expression(stream, depth - 1, callees),
            boolean_expression(stream, depth - 1, callees),
        ),
        // A nested `let` is shallower, so no name shadows another.
        2 => Expression::Let {
            name: format!("y{depth}"),
            value: Box::new(name_expr("b")),
            body: Box::new(binary(
                BinaryOperator::And,
                name_expr(&format!("y{depth}")),
                boolean_expression(stream, depth - 1, callees),
            )),
        },
        3 if !callees.is_empty() => {
            let callee = &callees[stream.index(callees.len())];
            Expression::If {
                condition: Box::new(binary(
                    BinaryOperator::Greater,
                    name_expr("x"),
                    integer_expr(0),
                )),
                then: Box::new(Expression::Call {
                    name: callee.clone(),
                    arguments: vec![
                        binary(BinaryOperator::Subtract, name_expr("x"), integer_expr(1)),
                        boolean_expression(stream, depth - 1, &[]),
                    ],
                }),
                otherwise: Box::new(boolean_expression(stream, depth - 1, &[])),
            }
        }
        _ => Expression::If {
            condition: Box::new(boolean_expression(stream, depth - 1, callees)),
            then: Box::new(boolean_expression(stream, depth - 1, callees)),
            otherwise: Box::new(boolean_expression(stream, depth - 1, callees)),
        },
    }
}

/// Package `seed`: 1 to 8 functions `g0 … gN(x: Int[0, 9], b: Boolean):
/// Boolean decreases(x)`. Each may call any function, itself included, so
/// the package has chains, self recursion and mutual recursion. Every third
/// package also declares `record List { next?: List; }` and compares two
/// `List`s in two functions.
fn package(seed: u64) -> PackageDeclarations {
    let mut stream = Stream(0x9e37_79b9_7f4a_7c15 ^ (seed + 1).wrapping_mul(0x2545_f491_4f6c_dd1d));
    let count = 1 + stream.index(8);
    let names: Vec<String> = (0..count).map(|at| format!("g{at}")).collect();
    let mut functions: Vec<FunctionDeclaration> = names
        .iter()
        .map(|name| {
            let body = boolean_expression(&mut stream, 3, &names);
            function(
                name,
                &[("x", int_form(0, 9)), ("b", boolean())],
                boolean(),
                Some(name_expr("x")),
                body,
            )
        })
        .collect();
    let mut declarations = PackageDeclarations::new(fixture_source());
    if seed.is_multiple_of(3) {
        let list = || TypeForm::name("List", SPAN);
        for name in ["eq_a", "eq_b"] {
            functions.push(function(
                name,
                &[("a", list()), ("c", list())],
                boolean(),
                None,
                binary(BinaryOperator::Equal, name_expr("a"), name_expr("c")),
            ));
        }
        declarations.types = list_types();
    }
    PackageDeclarations {
        functions,
        ..declarations
    }
}

/// QSL-221: over the generated corpus, every package checks; every node
/// outside a recursion group has the key and preimage bytes full keying of
/// its content gives; and the corpus's keys and preimages are byte for byte
/// the ones the lowering gave before it keyed each content once.
#[trace("FR-092-AC-6", "FR-092-AC-11", "TC-414")]
#[test]
fn keying_each_content_once_keeps_every_key_and_preimage() {
    let mut corpus = Sha256::new();
    let mut nodes = 0_usize;
    let mut grouped = 0_usize;
    for seed in 0..PACKAGES {
        let checked = package(seed)
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("package {seed} checks: {refusals:?}"));
        let graph = checked.semantic_graph();
        for node in graph.nodes() {
            nodes += 1;
            if node.recursion().is_some() {
                grouped += 1;
            } else {
                let keyed = node_key(&node.content.input()).expect("a lowered node keys");
                assert_eq!(keyed.key, node.key(), "package {seed}: key");
                assert_eq!(keyed.preimage, node.preimage(), "package {seed}: preimage");
            }
            corpus.update(node.key().as_bytes());
            let length = u64::try_from(node.preimage().len()).expect("a preimage length");
            corpus.update(length.to_be_bytes());
            corpus.update(node.preimage());
        }
    }
    // The corpus reaches both paths: nodes keyed alone and recursion groups.
    assert!(nodes > 10_000, "{nodes} nodes");
    assert!(grouped > 500, "{grouped} grouped nodes");
    assert_eq!(lower_hex(&corpus.finalize()), CORPUS_DIGEST);
}

/// Every content in one hash bucket.
fn one_bucket(_: &NodeContent) -> u64 {
    0
}

/// `value_types` built in turn by one lowering hashing contents by `hash`:
/// each type's key, and every node's key and preimage.
fn built_types(
    scope: &Scope,
    value_types: &[ValueType],
    hash: fn(&NodeContent) -> u64,
) -> (Vec<NodeKey>, Vec<(NodeKey, Vec<u8>)>) {
    let owner = SourceOwner::from(&fixture_source());
    let lock = LockEvidence::default();
    let mut occurrences = OccurrenceMap::default();
    let location = generated_location();
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut lowering = Lowering::new(
        scope,
        &owner,
        &[],
        scope.types().units().clone(),
        &lock,
        crate::check::MAX_CHECKING_DEPTH,
        0,
        &mut occurrences,
        &mut meter,
    )
    .with_content_hash(hash);
    let keys = value_types
        .iter()
        .map(|value_type| {
            lowering
                .type_node(value_type, &location)
                .expect("the type keys")
        })
        .collect();
    let nodes = lowering
        .finish(&location)
        .graph
        .nodes()
        .map(|node| (node.key(), node.preimage().to_vec()))
        .collect();
    (keys, nodes)
}

/// QSL-221: a content takes a key only when a node or draft holds that
/// same content, never because its hash matches. With every content in
/// one hash bucket, the keys and preimages are the ones the default hash
/// gives, across scalars, bounded, option, collection and declared types,
/// a recursive record (drafts and a recursion group) and repeated builds.
#[trace("FR-092-AC-1", "FR-092-AC-11", "TC-413")]
#[test]
fn a_hash_collision_never_takes_another_contents_key() {
    let pair = NodeKey::from_digest([1; 32]);
    let list = NodeKey::from_digest([5; 32]);
    let scope = scope_with(
        TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    pair,
                    "Pair",
                    CompositeShape::Tuple(vec![int(0, 9), ValueType::Boolean]),
                ),
                CompositeDeclaration::new(
                    list,
                    "List",
                    CompositeShape::Record(vec![
                        FieldDeclaration::new("head", int(0, 9), Presence::Required),
                        FieldDeclaration::new(
                            "next",
                            ValueType::Composite(list),
                            Presence::Optional,
                        ),
                    ]),
                ),
            ],
            [],
        )
        .expect("the types admit"),
        Vec::new(),
    );
    let once = [
        ValueType::Boolean,
        ValueType::Integer,
        int(0, 9),
        int(1, 9),
        ValueType::option(int(0, 9)),
        sequence(int(0, 9), Some((0, 5))),
        sequence(ValueType::Boolean, None),
        rational_9(),
        ValueType::Composite(pair),
        ValueType::Composite(list),
        ValueType::option(ValueType::Composite(list)),
    ];
    // Each type twice, so the second build of every content is a lookup.
    let value_types: Vec<ValueType> = once.iter().chain(&once).cloned().collect();
    let expected = built_types(&scope, &value_types, content_hash);
    let collided = built_types(&scope, &value_types, one_bucket);
    assert!(expected.1.len() > 10, "{} nodes", expected.1.len());
    assert_eq!(collided.0, expected.0, "type keys");
    assert_eq!(collided.1, expected.1, "every node's key and preimage");
}
