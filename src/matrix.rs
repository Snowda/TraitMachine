//! Transition matrix: `matrix[from][to]` is `P(next = to | current = from)` in percent, each row sums to 100.
//! [`save`] derives it from the `Resolve` thresholds; [`load`] gives a [`MatrixMachine`] that walks a row.
//! ```
//! use traitmachine::{save, load, StateKind, Probability};
//! let matrix = save();
//! let mut machine = load(&matrix, StateKind::Clear)?;
//! assert_eq!(machine.step(Probability::exact(80)), StateKind::Snow);
//! Ok::<(), traitmachine::MatrixError>(())
//! ```
//! The shape is part of the type, so a wrong-sized matrix does not compile:
//! ```compile_fail
//! let _ = traitmachine::load(&[[50u8; 2]; 2], traitmachine::StateKind::Rain);
//! ```

use crate::{AnyMachine, Clear, Machine, Probability, Rain, Snow};

/// A runtime, index-addressable tag for each state (the matrix needs an ordering).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StateKind {
	Rain,
	Clear,
	Snow,
}

impl StateKind {
	pub const ALL: [StateKind; 3] = [StateKind::Rain, StateKind::Clear, StateKind::Snow];

	#[must_use]
	pub fn index(self) -> usize {
		match self {
			StateKind::Rain => 0,
			StateKind::Clear => 1,
			StateKind::Snow => 2,
		}
	}

	#[must_use]
	pub fn name(self) -> &'static str {
		match self {
			StateKind::Rain => "Rain",
			StateKind::Clear => "Clear",
			StateKind::Snow => "Snow",
		}
	}
}

/// The number of states, and so each side of the matrix.
pub const STATES: usize = StateKind::ALL.len();

/// `matrix[from][to]` in percent; the type fixes the shape, so only row sums need a check.
pub type Matrix = [[u8; STATES]; STATES];

impl From<StateKind> for AnyMachine {
	fn from(kind: StateKind) -> Self {
		match kind {
			StateKind::Rain => AnyMachine::Rain(Machine::new(Rain)),
			StateKind::Clear => AnyMachine::Clear(Machine::new(Clear)),
			StateKind::Snow => AnyMachine::Snow(Machine::new(Snow)),
		}
	}
}

impl From<&AnyMachine> for StateKind {
	fn from(machine: &AnyMachine) -> Self {
		match machine {
			AnyMachine::Rain(_) => StateKind::Rain,
			AnyMachine::Clear(_) => StateKind::Clear,
			AnyMachine::Snow(_) => StateKind::Snow,
		}
	}
}

/// Why [`load`] refuses a matrix as a transition table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatrixError {
	RowSum { row: usize, sum: u16 },
}

impl core::fmt::Display for MatrixError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			MatrixError::RowSum { row, sum } => {
				write!(f, "row {row} probabilities sum to {sum}%, must be 100%")
			}
		}
	}
}

impl std::error::Error for MatrixError {}

/// Count each input `0..100` per state, so each row sums to 100 by construction.
#[must_use]
pub fn save() -> Matrix {
	let mut matrix: Matrix = [[0; STATES]; STATES];
	for from in StateKind::ALL {
		for percent in 0..100u8 {
			let next = AnyMachine::from(from).decide(Probability::exact(percent));
			matrix[from.index()][StateKind::from(&next).index()] += 1;
		}
	}
	matrix
}

/// Load a borrowed transition matrix into a runnable machine; every row must sum to 100.
pub fn load(matrix: &Matrix, start: StateKind) -> Result<MatrixMachine<'_>, MatrixError> {
	for (row, cells) in matrix.iter().enumerate() {
		let sum: u16 = cells.iter().map(|&p| u16::from(p)).sum();
		if sum != 100 {
			return Err(MatrixError::RowSum { row, sum });
		}
	}
	Ok(MatrixMachine { matrix, current: start })
}

/// A machine driven by a borrowed transition matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixMachine<'a> {
	matrix: &'a Matrix,
	current: StateKind,
}

impl<'a> MatrixMachine<'a> {
	#[must_use]
	pub fn state(&self) -> StateKind {
		self.current
	}

	/// The matrix behind this machine (save it back out).
	#[must_use]
	pub fn matrix(&self) -> &'a Matrix {
		self.matrix
	}

	/// Transition on `input`; the percent selects the next state by cumulative row probability.
	pub fn step(&mut self, input: Probability) -> StateKind {
		let roll = u16::from(input.percent());
		let mut acc: u16 = 0;
		for (kind, &prob) in StateKind::ALL.into_iter().zip(&self.matrix[self.current.index()]) {
			acc += u16::from(prob);
			if roll < acc {
				self.current = kind;
				return kind;
			}
		}
		// Unreachable for a validated row (sum 100, roll < 100); stay put if not.
		self.current
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use core::fmt::Write;

	/// A `fmt::Write` sink that consumes `self.0` as output arrives; no heap buffer.
	struct Expect<'a>(&'a str);

	impl Write for Expect<'_> {
		fn write_str(&mut self, s: &str) -> core::fmt::Result {
			self.0 = self.0.strip_prefix(s).ok_or(core::fmt::Error)?;
			Ok(())
		}
	}

	fn renders(value: &impl core::fmt::Display, expected: &str) -> bool {
		let mut expect = Expect(expected);
		write!(expect, "{value}").is_ok() && expect.0.is_empty()
	}

	#[test]
	fn every_saved_row_covers_all_probabilities() {
		for (row, cells) in save().iter().enumerate() {
			let sum: u16 = cells.iter().map(|&p| u16::from(p)).sum();
			assert_eq!(sum, 100, "row {row} must account for all 100%");
		}
	}

	#[test]
	fn saved_matrix_matches_the_literal_thresholds() {
		assert_eq!(save(), [[0, 70, 30], [33, 33, 34], [60, 0, 40]], "Resolve thresholds changed");
	}

	#[test]
	fn load_matches_trait_thresholds() -> Result<(), MatrixError> {
		let matrix = save();
		for start in StateKind::ALL {
			for percent in 0..100u8 {
				let input = Probability::exact(percent);
				let via_traits = StateKind::from(&AnyMachine::from(start).decide(input));
				let mut via_matrix: MatrixMachine = load(&matrix, start)?;
				assert_eq!(via_matrix.step(input), via_traits, "start {start:?}, p {percent}");
			}
		}
		return Ok(());
	}

	#[test]
	fn load_rejects_row_that_misses_probabilities() {
		let matrix: Matrix = [[90, 0, 0], [0, 100, 0], [0, 0, 100]];
		assert_eq!(load(&matrix, StateKind::Rain), Err(MatrixError::RowSum { row: 0, sum: 90 }));
	}

	#[test]
	fn round_trip_matrix_out_and_back() -> Result<(), MatrixError> {
		let original = save();
		let machine: MatrixMachine = load(&original, StateKind::Clear)?;
		assert_eq!(machine.matrix(), &original);
		return Ok(());
	}

	#[test]
	fn state_kind_name_covers_all_variants() {
		assert_eq!(StateKind::Rain.name(), "Rain");
		assert_eq!(StateKind::Clear.name(), "Clear");
		assert_eq!(StateKind::Snow.name(), "Snow");
	}

	#[test]
	fn machine_state_reports_current_row() -> Result<(), MatrixError> {
		let matrix = save();
		let machine: MatrixMachine = load(&matrix, StateKind::Snow)?;
		assert_eq!(machine.state(), StateKind::Snow);
		return Ok(());
	}

	#[test]
	fn display_renders_row_sum() {
		let err = MatrixError::RowSum { row: 0, sum: 90 };
		assert!(renders(&err, "row 0 probabilities sum to 90%, must be 100%"), "full message");
		assert!(!renders(&err, "row 0 probabilities sum to 90%"), "a prefix alone must not match");
		assert!(!renders(&err, "row 1 probabilities sum to 90%, must be 100%"), "a wrong row must not match");
	}

	#[test]
	fn matrix_error_is_std_error() {
		let err: &dyn std::error::Error = &MatrixError::RowSum { row: 1, sum: 50 };
		assert!(err.source().is_none());
	}

	#[test]
	fn step_input_at_or_above_100_stays_put() -> Result<(), MatrixError> {
		let matrix = save();
		let mut machine: MatrixMachine = load(&matrix, StateKind::Clear)?;
		assert_eq!(machine.step(Probability::exact(100)), StateKind::Clear);
		assert_eq!(machine.state(), StateKind::Clear);
		return Ok(());
	}
}
