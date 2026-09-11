// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: bounded Boolean partition proof over source-authorized observations.

use crate::protocol_artifact::{work::Work, Dimension, Error, Invalid, Unsupported};

#[derive(Clone, Copy, Debug)]
pub(super) enum Op {
    Constant(bool),
    Atom(usize),
    Not(usize),
    And(usize, usize),
    Or(usize, usize),
    Implies(usize, usize),
    Equal(usize, usize),
    NotEqual(usize, usize),
    If {
        condition: usize,
        then_value: usize,
        else_value: usize,
    },
    Let {
        initializer: usize,
        body: usize,
    },
}

impl Op {
    fn inputs(self) -> [Option<usize>; 3] {
        match self {
            Self::Constant(_) | Self::Atom(_) => [None, None, None],
            Self::Not(value) => [Some(value), None, None],
            Self::And(left, right)
            | Self::Or(left, right)
            | Self::Implies(left, right)
            | Self::Equal(left, right)
            | Self::NotEqual(left, right) => [Some(left), Some(right), None],
            Self::If {
                condition,
                then_value,
                else_value,
            } => [Some(condition), Some(then_value), Some(else_value)],
            Self::Let { initializer, body } => [Some(initializer), Some(body), None],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Basis {
    Constant,
    Atom(usize),
    Composite,
}

struct Node {
    op: Op,
    // Derived from every original child, never from evaluated Boolean results.
    basis: Basis,
}

pub(super) struct Arena {
    nodes: Vec<Node>,
}

impl Arena {
    pub(super) fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub(super) fn push(&mut self, op: Op, work: &mut Work) -> Result<usize, Error> {
        work.visit()?;
        let mut bases = [None; 3];
        let mut closed = true;
        for (slot, input) in op.inputs().into_iter().enumerate() {
            if let Some(input) = input {
                work.visit()?;
                let child = self
                    .nodes
                    .get(input)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
                bases[slot] = Some(child.basis);
                closed &= child.basis == Basis::Constant;
            }
        }
        let basis = match op {
            Op::Constant(_) => Basis::Constant,
            Op::Atom(atom) => Basis::Atom(atom),
            Op::Let { .. } => match (bases[0], bases[1]) {
                (Some(Basis::Constant), Some(Basis::Constant)) => Basis::Constant,
                (Some(Basis::Atom(left)), Some(Basis::Atom(right))) if left == right => {
                    Basis::Atom(left)
                }
                _ => Basis::Composite,
            },
            Op::Not(_)
            | Op::And(_, _)
            | Op::Or(_, _)
            | Op::Implies(_, _)
            | Op::Equal(_, _)
            | Op::NotEqual(_, _)
            | Op::If { .. } => {
                if closed {
                    Basis::Constant
                } else {
                    Basis::Composite
                }
            }
        };
        work.charge(Dimension::Entries, 1)?;
        self.nodes.try_reserve(1).map_err(|_| Error::Allocation)?;
        let index = self.nodes.len();
        self.nodes.push(Node { op, basis });
        Ok(index)
    }

    pub(super) fn partition(
        &self,
        guards: &[usize],
        visible: &[usize],
        atom_count: usize,
        work: &mut Work,
    ) -> Result<Vec<bool>, Error> {
        work.visit()?;
        if guards.is_empty() {
            return Err(unproved());
        }
        let mut horizon = 0;
        for &root in guards.iter().chain(visible) {
            work.visit()?;
            self.nodes
                .get(root)
                .ok_or(Error::Invalid(Invalid::Reference))?;
            // A valid vector index is strictly below its length, so +1 fits.
            horizon = horizon.max(root + 1);
        }

        // First close each advertised expression and collect exactly the
        // observation atoms on which its result is authorized to depend.
        let mut visible_atoms = bits(atom_count, work)?;
        let mut visible_reached = bits(horizon, work)?;
        for &root in visible {
            work.visit()?;
            visible_reached[root] = true;
        }
        self.close(&mut visible_reached, work)?;
        for (index, &included) in visible_reached.iter().enumerate() {
            work.visit()?;
            if !included {
                continue;
            }
            if let Op::Atom(atom) = self
                .nodes
                .get(index)
                .ok_or(Error::Invalid(Invalid::Reference))?
                .op
            {
                work.visit()?;
                *visible_atoms
                    .get_mut(atom)
                    .ok_or(Error::Invalid(Invalid::Reference))? = true;
            }
        }

        // A guard may only depend on atoms that occur in that original visible
        // closure. This is distinct from proving that its selected case is
        // determined by the advertised Boolean results below.
        let mut reached = bits(horizon, work)?;
        for &root in guards {
            work.visit()?;
            reached[root] = true;
        }
        self.close(&mut reached, work)?;
        for (index, &included) in reached.iter().enumerate() {
            work.visit()?;
            if !included {
                continue;
            }
            if let Op::Atom(atom) = self
                .nodes
                .get(index)
                .ok_or(Error::Invalid(Invalid::Reference))?
                .op
            {
                work.visit()?;
                if !visible_atoms
                    .get(atom)
                    .copied()
                    .ok_or(Error::Invalid(Invalid::Reference))?
                {
                    return Err(unproved());
                }
            }
        }

        // Formula roots themselves are evaluated with their guards, preserving
        // every original child (including a syntactically unused let initializer).
        for &root in visible {
            work.visit()?;
            reached[root] = true;
        }
        self.close(&mut reached, work)?;

        let mut assignment = bits(atom_count, work)?;
        let mut slots = Vec::new();
        let mut visible_atom_count = 0;
        for &included in &visible_atoms {
            work.visit()?;
            if included {
                visible_atom_count += 1;
            }
        }
        work.charge(Dimension::Entries, visible_atom_count)?;
        slots
            .try_reserve_exact(visible_atom_count)
            .map_err(|_| Error::Allocation)?;
        for (atom, &included) in visible_atoms.iter().enumerate() {
            work.visit()?;
            if included {
                slots.push(atom);
            }
        }
        let mut ordered = Vec::new();
        for (index, &included) in reached.iter().enumerate() {
            work.visit()?;
            if included {
                work.charge(Dimension::Entries, 1)?;
                ordered.try_reserve(1).map_err(|_| Error::Allocation)?;
                ordered.push(index);
            }
        }
        let mut values = bits(horizon, work)?;
        let mut feasible = bits(guards.len(), work)?;
        let mut direct = true;
        for &root in visible {
            work.visit()?;
            direct &= matches!(
                self.nodes
                    .get(root)
                    .ok_or(Error::Invalid(Invalid::Reference))?
                    .basis,
                Basis::Constant | Basis::Atom(_)
            );
        }
        let mut signatures: Vec<(Vec<bool>, usize)> = Vec::new();
        loop {
            work.visit()?;
            for &index in &ordered {
                work.visit()?;
                values[index] = evaluate(self.nodes[index].op, &values, &assignment, work)?;
            }
            let mut selected = None;
            for (case, &root) in guards.iter().enumerate() {
                work.visit()?;
                if value(&values, root, work)? && selected.replace(case).is_some() {
                    return Err(unproved());
                }
            }
            let selected = selected.ok_or_else(unproved)?;
            feasible[selected] = true;

            if !direct {
                let mut signature = Vec::new();
                work.charge(Dimension::Entries, visible.len())?;
                signature
                    .try_reserve_exact(visible.len())
                    .map_err(|_| Error::Allocation)?;
                for &root in visible {
                    work.visit()?;
                    signature.push(value(&values, root, work)?);
                }
                let mut prior = None;
                for (known, case) in &signatures {
                    work.visit()?;
                    if known.len() != signature.len() {
                        return Err(Error::Invalid(Invalid::Reference));
                    }
                    let mut equal = true;
                    for (left, right) in known.iter().zip(&signature) {
                        work.visit()?;
                        if left != right {
                            equal = false;
                            break;
                        }
                    }
                    if equal {
                        prior = Some(*case);
                        break;
                    }
                }
                if let Some(case) = prior {
                    if case != selected {
                        return Err(unproved());
                    }
                } else {
                    work.charge(Dimension::Entries, 1)?;
                    signatures.try_reserve(1).map_err(|_| Error::Allocation)?;
                    signatures.push((signature, selected));
                }
            }

            // Enumerate the entire selected visible basis, even atoms unused by
            // guards. A charged carry has no fixed-width mask or atom ceiling.
            let mut advanced = false;
            for &atom in &slots {
                work.visit()?;
                assignment[atom] = !assignment[atom];
                if assignment[atom] {
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                return Ok(feasible);
            }
        }
    }

    fn close(&self, reached: &mut [bool], work: &mut Work) -> Result<(), Error> {
        // push admits only prior child handles. Reverse order therefore closes
        // every original dependency without recursion or an expanding stack.
        for index in (0..reached.len()).rev() {
            work.visit()?;
            if !reached[index] {
                continue;
            }
            let op = self
                .nodes
                .get(index)
                .ok_or(Error::Invalid(Invalid::Reference))?
                .op;
            for child in op.inputs().into_iter().flatten() {
                work.visit()?;
                *reached
                    .get_mut(child)
                    .ok_or(Error::Invalid(Invalid::Reference))? = true;
            }
        }
        Ok(())
    }
}

fn bits(len: usize, work: &mut Work) -> Result<Vec<bool>, Error> {
    work.charge(Dimension::Entries, len)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| Error::Allocation)?;
    values.resize(len, false);
    Ok(values)
}

fn value(values: &[bool], index: usize, work: &mut Work) -> Result<bool, Error> {
    work.visit()?;
    values
        .get(index)
        .copied()
        .ok_or(Error::Invalid(Invalid::Reference))
}

fn evaluate(op: Op, values: &[bool], atoms: &[bool], work: &mut Work) -> Result<bool, Error> {
    Ok(match op {
        Op::Constant(value) => value,
        Op::Atom(atom) => value(atoms, atom, work)?,
        Op::Not(inner) => !value(values, inner, work)?,
        Op::And(left, right) => value(values, left, work)? & value(values, right, work)?,
        Op::Or(left, right) => value(values, left, work)? | value(values, right, work)?,
        Op::Implies(left, right) => !value(values, left, work)? | value(values, right, work)?,
        Op::Equal(left, right) => value(values, left, work)? == value(values, right, work)?,
        Op::NotEqual(left, right) => value(values, left, work)? != value(values, right, work)?,
        Op::If {
            condition,
            then_value,
            else_value,
        } => {
            let condition = value(values, condition, work)?;
            let then_value = value(values, then_value, work)?;
            let else_value = value(values, else_value, work)?;
            if condition {
                then_value
            } else {
                else_value
            }
        }
        Op::Let { initializer, body } => {
            value(values, initializer, work)?;
            value(values, body, work)?
        }
    })
}

fn unproved() -> Error {
    Error::Unsupported(Unsupported::FamilyProof)
}
