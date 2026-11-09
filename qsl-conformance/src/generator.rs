// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-295: the seeded parity program generator.
//!
//! Given a 64-bit seed and a count, the generator yields that many native
//! source units, each with its argument tuples; the same seed and count yield
//! byte-equal units and tuples. Only units that `check` admits count; the
//! generator draws again for a unit `check` refuses and reports how many it
//! discarded.
//!
//! This generator draws the `Boolean` and bounded `Int` subset of FR-295's
//! types and the `literal`, `operator`, `let`, `if` and `call` forms. About
//! half the units also carry a few functions over `Rational`, `Decimal`, `Text`
//! and record values, drawn from a second generator seeded by the unit's own
//! text, so adding them left the `Boolean` and `Int` units as they were. The
//! collection types and the `collection` form are not drawn yet.

use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use quire_exact::{
    admit_text, Decimal, Integer, Meter, Rational, ScalarLimits, TextPayload, TextProfile,
    TextType, Value,
};
use quire_semantic_value::checking::CheckingLimits;

/// The gate's default seed: the ASCII bytes `QSL-JIT1` read big-endian.
pub const DEFAULT_SEED: u64 = 0x5153_4C2D_4A49_5431;

const RANGE: i64 = 1000;

struct Rng(u64);

impl Rng {
    /// SplitMix64.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int,
    Boolean,
}

struct Signature {
    parameters: Vec<Kind>,
    result: Kind,
}

/// One generated unit.
#[derive(Clone, Debug)]
pub struct GeneratedUnit {
    /// The unit's index among the units drawn (admitted or not).
    pub index: u64,
    /// The native source text.
    pub source: String,
    /// Each function with its argument tuples.
    pub functions: Vec<(String, Vec<Vec<Value>>)>,
}

struct Body<'a> {
    rng: &'a mut Rng,
    earlier: &'a [Signature],
    budget: usize,
    locals: usize,
}

impl Body<'_> {
    fn leaf(&mut self, kind: Kind, scope: &[(String, Kind)]) -> String {
        let candidates: Vec<&String> = scope
            .iter()
            .filter(|(_, k)| *k == kind)
            .map(|(n, _)| n)
            .collect();
        if !candidates.is_empty() && self.rng.below(3) != 0 {
            let pick = self.rng.below(candidates.len() as u64) as usize;
            return candidates[pick].clone();
        }
        match kind {
            Kind::Int => self.rng.below(10).to_string(),
            Kind::Boolean => (self.rng.below(2) == 0).to_string(),
        }
    }

    fn expression(&mut self, kind: Kind, scope: &[(String, Kind)], depth: usize) -> String {
        if self.budget == 0 || depth == 0 {
            return self.leaf(kind, scope);
        }
        self.budget -= 1;
        match kind {
            Kind::Int => match self.rng.below(8) {
                0 | 1 => {
                    let operator = ["+", "-", "*"][self.rng.below(3) as usize];
                    let (a, b) = (
                        self.expression(Kind::Int, scope, depth - 1),
                        self.expression(Kind::Int, scope, depth - 1),
                    );
                    format!("({a} {operator} {b})")
                }
                2 => format!("(-{})", self.expression(Kind::Int, scope, depth - 1)),
                3 => {
                    let divisor = 1 + self.rng.below(9);
                    format!(
                        "({} mod {divisor})",
                        self.expression(Kind::Int, scope, depth - 1)
                    )
                }
                4 => self.conditional(Kind::Int, scope, depth),
                5 => self.binding(Kind::Int, scope, depth),
                6 => self.call(Kind::Int, scope, depth),
                _ => self.leaf(kind, scope),
            },
            Kind::Boolean => match self.rng.below(7) {
                0 | 1 => {
                    let operator = ["<", "<=", ">", ">=", "=", "!="][self.rng.below(6) as usize];
                    let (a, b) = (
                        self.expression(Kind::Int, scope, depth - 1),
                        self.expression(Kind::Int, scope, depth - 1),
                    );
                    format!("({a} {operator} {b})")
                }
                2 => {
                    let operator = ["and", "or", "implies"][self.rng.below(3) as usize];
                    let (a, b) = (
                        self.expression(Kind::Boolean, scope, depth - 1),
                        self.expression(Kind::Boolean, scope, depth - 1),
                    );
                    format!("({a} {operator} {b})")
                }
                3 => format!("(not {})", self.expression(Kind::Boolean, scope, depth - 1)),
                4 => self.conditional(Kind::Boolean, scope, depth),
                5 => self.call(Kind::Boolean, scope, depth),
                _ => self.leaf(kind, scope),
            },
        }
    }

    fn conditional(&mut self, kind: Kind, scope: &[(String, Kind)], depth: usize) -> String {
        let condition = self.expression(Kind::Boolean, scope, depth - 1);
        let (then, otherwise) = (
            self.expression(kind, scope, depth - 1),
            self.expression(kind, scope, depth - 1),
        );
        format!("(if {condition} then {then} else {otherwise})")
    }

    fn binding(&mut self, kind: Kind, scope: &[(String, Kind)], depth: usize) -> String {
        let name = format!("l{}", self.locals);
        self.locals += 1;
        let value = self.expression(Kind::Int, scope, depth - 1);
        let mut inner = scope.to_vec();
        inner.push((name.clone(), Kind::Int));
        let body = self.expression(kind, &inner, depth - 1);
        format!("(let {name} = {value} in {body})")
    }

    fn call(&mut self, kind: Kind, scope: &[(String, Kind)], depth: usize) -> String {
        let matching: Vec<usize> = self
            .earlier
            .iter()
            .enumerate()
            .filter(|(_, s)| s.result == kind)
            .map(|(i, _)| i)
            .collect();
        if matching.is_empty() {
            return self.leaf(kind, scope);
        }
        let index = matching[self.rng.below(matching.len() as u64) as usize];
        let parameters: Vec<Kind> = self.earlier[index].parameters.clone();
        let arguments: Vec<String> = parameters
            .iter()
            .map(|k| self.expression(*k, scope, depth.saturating_sub(2)))
            .collect();
        format!("f{index}({})", arguments.join(", "))
    }
}

fn unit_source(rng: &mut Rng) -> (String, Vec<(String, Vec<Kind>)>) {
    let functions = 1 + rng.below(8) as usize;
    let mut signatures: Vec<Signature> = Vec::new();
    let mut text = String::new();
    let mut shapes = Vec::new();
    for index in 0..functions {
        let arity = rng.below(5) as usize;
        let parameters: Vec<Kind> = (0..arity)
            .map(|_| {
                if rng.below(3) == 0 {
                    Kind::Boolean
                } else {
                    Kind::Int
                }
            })
            .collect();
        let result = if rng.below(3) == 0 {
            Kind::Boolean
        } else {
            Kind::Int
        };
        let scope: Vec<(String, Kind)> = parameters
            .iter()
            .enumerate()
            .map(|(i, k)| (format!("p{i}"), *k))
            .collect();
        let mut body = Body {
            rng,
            earlier: &signatures,
            budget: 24,
            locals: 0,
        };
        let expression = body.expression(result, &scope, 5);
        let declared = |k: Kind| match k {
            Kind::Int => format!("Int[-{RANGE}, {RANGE}]"),
            Kind::Boolean => "Boolean".to_owned(),
        };
        let parameter_text: Vec<String> = scope
            .iter()
            .map(|(name, k)| format!("{name}: {}", declared(*k)))
            .collect();
        let result_text = match result {
            Kind::Int => "Integer".to_owned(),
            Kind::Boolean => "Boolean".to_owned(),
        };
        text.push_str(&format!(
            "function f{index} using v({}): {result_text} pure {{ {expression} }}\n",
            parameter_text.join(", ")
        ));
        shapes.push((format!("f{index}"), parameters.clone()));
        signatures.push(Signature { parameters, result });
    }
    (text, shapes)
}

/// The aliases the functions of [`extras`] use.
const EXTRA_TYPES: &str = "\
type GQ = Rational[-10000000000000000000000000, 10000000000000000000000000; 1, 100000];\n\
type GP = Rational[1, 10000000000000000000000000; 1, 100000];\n\
type GS = Rational[-10000000000000000000000000000000, 10000000000000000000000000000000; 1, 10000000000];\n\
type GM = Rational[-1000000000000000000000000000000000000000000000000000, 1000000000000000000000000000000000000000000000000000; 1, 10000000000];\n\
type GD = Rational[-10000000000000000000000000000000, 10000000000000000000000000000000; 1, 10000000000000000000000000000000];\n\
type GX = Decimal[0, 2000000; 0, 3; exact];\n\
type GT = Text[0, 16; nfd];\n\
record GR { x: Int[0, 9]; y: Boolean; }\n";

/// Which values a template's parameters take.
#[derive(Clone, Copy)]
enum Sample {
    Rational,
    Positive,
    Decimal,
    Text,
    Flag,
    Digit,
    Divisor,
}

/// Functions over `Rational`, `Decimal`, `Text` and a record: the
/// declaration and the kinds of its parameters.
const EXTRA_FUNCTIONS: [(&str, &[Sample]); 18] = [
    (
        "(p: GQ, q: GQ): GS pure { p + q }",
        &[Sample::Rational, Sample::Rational],
    ),
    (
        "(p: GQ, q: GQ): GS pure { p - q }",
        &[Sample::Rational, Sample::Rational],
    ),
    (
        "(p: GQ, q: GQ): GM pure { p * q }",
        &[Sample::Rational, Sample::Rational],
    ),
    (
        "(p: GQ, q: GP): GD pure { p / q }",
        &[Sample::Rational, Sample::Positive],
    ),
    ("(p: GQ): GQ pure { -p }", &[Sample::Rational]),
    (
        "(p: GQ, q: GQ): Boolean pure { p < q }",
        &[Sample::Rational, Sample::Rational],
    ),
    (
        "(p: GQ, q: GQ): Boolean pure { p = q }",
        &[Sample::Rational, Sample::Rational],
    ),
    (
        "(c: Boolean, p: GQ, q: GQ): GQ pure { if c then p else q }",
        &[Sample::Flag, Sample::Rational, Sample::Rational],
    ),
    (
        "(a: GX, b: GX): GX pure { a + b }",
        &[Sample::Decimal, Sample::Decimal],
    ),
    (
        "(a: GX, b: GX): Boolean pure { a <= b }",
        &[Sample::Decimal, Sample::Decimal],
    ),
    (
        "(a: GT, b: GT): Boolean pure { a < b }",
        &[Sample::Text, Sample::Text],
    ),
    (
        "(a: GT, b: GT): Boolean pure { a != b }",
        &[Sample::Text, Sample::Text],
    ),
    (
        "(c: Boolean, a: GT, b: GT): GT pure { if c then a else b }",
        &[Sample::Flag, Sample::Text, Sample::Text],
    ),
    (
        "(p: Int[0, 9], q: Int[1, 9]): Rational[0, 9; 1, 9] pure { p / q }",
        &[Sample::Digit, Sample::Divisor],
    ),
    (
        "(a: Int[0, 9], b: Boolean): GR pure { GR { x: a, y: b } }",
        &[Sample::Digit, Sample::Flag],
    ),
    (
        "(a: Int[0, 9], b: Boolean): Boolean pure { (GR { x: a, y: b }).y }",
        &[Sample::Digit, Sample::Flag],
    ),
    (
        "(a: Int[0, 9], b: Boolean): Boolean pure { GR { x: a, y: b } = GR { x: a, y: not b } }",
        &[Sample::Digit, Sample::Flag],
    ),
    (
        "(a: Int[0, 9], b: Boolean): Int[0, 9] pure { let r = GR { x: a, y: b } in if r.y then r.x else a }",
        &[Sample::Digit, Sample::Flag],
    ),
];

fn rational_value(numerator: i64, denominator: i64) -> Value {
    match Rational::new(Integer::from(numerator), Integer::from(denominator)) {
        Ok(value) => Value::Rational(value),
        Err(_) => Value::Rational(Rational::from_integer(Integer::from(numerator))),
    }
}

fn sample_value(rng: &mut Rng, sample: Sample) -> Option<Value> {
    let digit = |rng: &mut Rng| Value::Integer(Integer::from(rng.below(10) as i64));
    let small = |rng: &mut Rng| rng.below(2001) as i64 - 1000;
    Some(match sample {
        Sample::Flag => Value::Boolean(rng.below(2) == 0),
        Sample::Digit => digit(rng),
        Sample::Divisor => Value::Integer(Integer::from(1 + rng.below(9) as i64)),
        Sample::Rational => rational_value(small(rng), 1 + rng.below(9) as i64),
        Sample::Positive => rational_value(1 + rng.below(1000) as i64, 1 + rng.below(9) as i64),
        Sample::Decimal => {
            Value::Decimal(Decimal::new(Integer::from(rng.below(2_000_001) as i64), 3))
        }
        Sample::Text => {
            let payload = ["", "a", "b", "ab", "\u{e9}", "e\u{301}"][rng.below(6) as usize];
            let text = admit_text(
                &TextPayload::from_utf8(payload.as_bytes()).ok()?,
                &TextType::new(0, 16, TextProfile::Nfd).ok()?,
                &mut Meter::new(ScalarLimits {
                    integer_bits: u64::MAX,
                    decimal_digits: u64::MAX,
                    scale_expansion: u64::MAX,
                    text_input_bytes: u64::MAX,
                    text_scalars: u64::MAX,
                    normalized_scalars: u64::MAX,
                    unit_edges: u64::MAX,
                    value_occurrences: u64::MAX,
                    work_units: u64::MAX,
                    result_units: u64::MAX,
                }),
            )
            .completed()?;
            Value::Text(text)
        }
    })
}

/// FNV-1a over `text`: the seed of a unit's second generator.
fn text_seed(text: &str) -> u64 {
    text.bytes().fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
    })
}

/// A function's name and its argument tuples.
type Functions = (String, Vec<Vec<Value>>);

/// The functions over `Rational`, `Decimal` and `Text` a unit with source
/// `source` carries (none for about half the units), as source text and
/// argument tuples. The draw reads only `source`, never the unit stream.
fn extras(source: &str) -> (String, Vec<Functions>) {
    let mut rng = Rng(text_seed(source));
    if rng.below(2) == 0 {
        return (String::new(), Vec::new());
    }
    let mut text = EXTRA_TYPES.to_owned();
    let mut functions = Vec::new();
    for index in 0..1 + rng.below(3) as usize {
        let (declaration, samples) =
            EXTRA_FUNCTIONS[rng.below(EXTRA_FUNCTIONS.len() as u64) as usize];
        let name = format!("g{index}");
        text.push_str(&format!("function {name} using v{declaration}\n"));
        let tuples = (0..4)
            .filter_map(|_| {
                let mut tuple = Vec::new();
                for sample in samples {
                    tuple.push(sample_value(&mut rng, *sample)?);
                }
                Some(tuple)
            })
            .collect();
        functions.push((name, tuples));
    }
    (text, functions)
}

fn tuples(rng: &mut Rng, parameters: &[Kind]) -> Vec<Vec<Value>> {
    let integer = |v: i64| Value::Integer(Integer::from(v));
    let pick = |rng: &mut Rng, k: Kind, mode: u8| match (k, mode) {
        (Kind::Int, 0) => integer(-RANGE),
        (Kind::Int, 1) => integer(RANGE),
        (Kind::Int, _) => integer(rng.below(2 * RANGE as u64 + 1) as i64 - RANGE),
        (Kind::Boolean, 0) => Value::Boolean(false),
        (Kind::Boolean, 1) => Value::Boolean(true),
        (Kind::Boolean, _) => Value::Boolean(rng.below(2) == 0),
    };
    (0..4_u8)
        .map(|mode| parameters.iter().map(|k| pick(rng, *k, mode)).collect())
        .collect()
}

/// `source` compiled as one complete-V1 unit, or `None` when `check` or an
/// earlier stage refuses it.
#[must_use]
pub fn compile(source: &str) -> Option<CheckedPackage> {
    compile_whole(&format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{source}"
    ))
}

/// The names of the functions of `source` that take no parameters.
#[must_use]
pub fn zero_parameter_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("function ")?;
            let (name, rest) = rest.split_once(' ')?;
            let (_, after) = rest.strip_prefix("using ")?.split_once('(')?;
            after.starts_with(')').then(|| name.to_owned())
        })
        .collect()
}

/// A complete unit (language and profile declarations included) compiled
/// into a checked package, or `None` when a stage refuses it.
#[must_use]
pub fn compile_whole(text: &str) -> Option<CheckedPackage> {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default()
            .with_tokens(1 << 24)
            .with_nodes(1 << 24)
            .with_source_bytes(1 << 26)
            .with_work_units(1 << 40),
    )
    .ok()?;
    if !parsed.is_admissible() {
        return None;
    }
    let unit = qsl_forms::build_unit(&parsed).ok()?;
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .ok()?;
    let checked = declarations
        .check(
            CheckingLimits::default()
                .with_nodes(1 << 30)
                .with_input_bytes(1 << 30)
                .with_work_budget(1 << 40),
        )
        .ok()?;
    Some(CheckedPackage::link(checked))
}

/// The units the generator yields for `seed` and `count`, each with its
/// compiled package, and how many drawn units `check` refused.
#[must_use]
pub fn generate(seed: u64, count: u64) -> (Vec<(GeneratedUnit, CheckedPackage)>, u64) {
    let mut rng = Rng(seed);
    let mut admitted = Vec::new();
    let mut discarded = 0;
    let mut index = 0;
    while (admitted.len() as u64) < count {
        let (mut source, shapes) = unit_source(&mut rng);
        let mut functions: Vec<Functions> = shapes
            .iter()
            .map(|(name, parameters)| (name.clone(), tuples(&mut rng, parameters)))
            .collect();
        let (extra_source, extra_functions) = extras(&source);
        source.push_str(&extra_source);
        functions.extend(extra_functions);
        match compile(&source) {
            Some(package) => admitted.push((
                GeneratedUnit {
                    index,
                    source,
                    functions,
                },
                package,
            )),
            None => discarded += 1,
        }
        index += 1;
    }
    (admitted, discarded)
}
