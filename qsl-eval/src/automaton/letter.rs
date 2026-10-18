// SPDX-License-Identifier: AGPL-3.0-or-later
//! Letters and transition guards: the alphabet of a property automaton is
//! the truth assignments of its atoms.

use crate::automaton::error::AutomatonError;

/// The number of atoms one letter holds.
pub const MAX_ATOMS: usize = 64;

/// One atomic predicate of a property automaton, by index below
/// [`MAX_ATOMS`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Atom(u8);

impl Atom {
    /// The atom at `index`.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::AtomOutOfRange`] when `index` is not below
    /// [`MAX_ATOMS`].
    pub fn new(index: usize) -> Result<Self, AutomatonError> {
        u8::try_from(index)
            .ok()
            .filter(|small| usize::from(*small) < MAX_ATOMS)
            .map(Self)
            .ok_or(AutomatonError::AtomOutOfRange {
                index,
                max: MAX_ATOMS,
            })
    }

    /// The atom's index.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// The letter bit of this atom.
    fn mask(self) -> u64 {
        1u64 << u32::from(self.0)
    }
}

/// One truth assignment of the atoms: the letter read at one position.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Letter(u64);

impl Letter {
    /// The letter in which exactly the atoms of the set bits of `bits` hold.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// The letter's bits, atom `i` being bit `i`.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// This letter with `atom` made true.
    #[must_use]
    pub fn with(self, atom: Atom) -> Self {
        Self(self.0 | atom.mask())
    }

    /// Whether `atom` holds in this letter.
    pub fn holds(self, atom: Atom) -> bool {
        self.0 & atom.mask() != 0
    }
}

/// A conjunction of atom literals: the condition on the letter that enables
/// a transition. The default guard admits every letter.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Guard {
    required: u64,
    forbidden: u64,
}

impl Guard {
    /// The guard that admits every letter.
    pub const fn always() -> Self {
        Self {
            required: 0,
            forbidden: 0,
        }
    }

    /// This guard, also requiring `atom` to hold.
    #[must_use]
    pub fn requiring(self, atom: Atom) -> Self {
        Self {
            required: self.required | atom.mask(),
            ..self
        }
    }

    /// This guard, also requiring `atom` not to hold.
    #[must_use]
    pub fn forbidding(self, atom: Atom) -> Self {
        Self {
            forbidden: self.forbidden | atom.mask(),
            ..self
        }
    }

    /// Whether some letter satisfies the guard: no atom is both required and
    /// forbidden.
    pub const fn is_satisfiable(self) -> bool {
        self.required & self.forbidden == 0
    }

    /// Whether `letter` satisfies the guard.
    pub const fn admits(self, letter: Letter) -> bool {
        letter.0 & self.required == self.required && letter.0 & self.forbidden == 0
    }
}
