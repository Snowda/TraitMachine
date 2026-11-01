//! Matrix-algebra view of the machine: a row-stochastic transition matrix where
//! `matrix[from][to]` is `P(next = to | current = from)` as an integer percent, so
//! every row sums to 100. [`save`] derives it from the `Resolve` thresholds; [`load`]
//! turns a matrix back into a [`MatrixMachine`] that steps by a cumulative row walk.
//! ```
//! use traitmachine::{save, load, StateKind, Probability};
//! let matrix = save();
//! assert_eq!(matrix.shape(), &[3, 3]);
//! let mut machine = match load(matrix, StateKind::Clear) {
//!     Ok(machine) => machine,
//!     Err(e) => return Err(format!("load matrix: {e}")),
//! };
//! assert_eq!(machine.step(Probability::exact(80)), StateKind::Snow);
//! Ok::<(), String>(())
//! ```

use ndarray::Array2;

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

/// Why a 2D matrix can't be loaded as a transition table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatrixError {
	NotSquare { rows: usize, cols: usize },
	WrongSize { expected: usize, found: usize },
	RowSum { row: usize, sum: u16 },
}

impl core::fmt::Display for MatrixError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			MatrixError::NotSquare { rows, cols } => {
				write!(f, "transition matrix must be square, got {rows}x{cols}")
			}
			MatrixError::WrongSize { expected, found } => {
				write!(f, "expected a {expected}x{expected} matrix, got {found}x{found}")
			}
			MatrixError::RowSum { row, sum } => {
				write!(f, "row {row} probabilities sum to {sum}%, must be 100%")
			}
		}
	}
}

impl std::error::Error for MatrixError {}

/// Derive the transition matrix by tallying every input `0..100` per state, so each
/// row is the exact percent distribution and sums to 100 by construction.
#[must_use]
pub fn save() -> Array2<u8> {
	let n = StateKind::ALL.len();
	let mut matrix = Array2::<u8>::zeros((n, n));
	for from in StateKind::ALL {
		for percent in 0..100u8 {
			let next = AnyMachine::from(from).decide(Probability::exact(percent));
			let to = StateKind::from(&next);
			matrix[[from.index(), to.index()]] += 1;
		}
	}
	matrix
}

/// Load a 2D transition matrix into a runnable machine, validating it is N×N and every row sums to 100.
pub fn load(matrix: Array2<u8>, start: StateKind) -> Result<MatrixMachine, MatrixError> {
	let (rows, cols) = matrix.dim();
	if rows != cols {
		return Err(MatrixError::NotSquare { rows, cols });
	}
	let n = StateKind::ALL.len();
	if rows != n {
		return Err(MatrixError::WrongSize { expected: n, found: rows });
	}
	for row in 0..rows {
		let sum: u16 = matrix.row(row).iter().map(|&p| u16::from(p)).sum();
		if sum != 100 {
			return Err(MatrixError::RowSum { row, sum });
		}
	}
	Ok(MatrixMachine { matrix, current: start.index() })
}

/// A machine driven by a loaded transition matrix; the current state is a row index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixMachine {
	matrix: Array2<u8>,
	current: usize,
}

impl MatrixMachine {
	#[must_use]
	pub fn state(&self) -> StateKind {
		StateKind::ALL[self.current]
	}

	/// The 2D matrix backing this machine (save it back out).
	#[must_use]
	pub fn matrix(&self) -> &Array2<u8> {
		&self.matrix
	}

	/// Transition on `input`; the percent selects the next state by cumulative row probability.
	pub fn step(&mut self, input: Probability) -> StateKind {
		let roll = u16::from(input.percent());
		let mut acc: u16 = 0;
		let row = self.matrix.row(self.current);
		for (col, &prob) in row.iter().enumerate() {
			acc += u16::from(prob);
			if roll < acc {
				self.current = col;
				return StateKind::ALL[col];
			}
		}
		// Unreachable for a validated row (sum 100, roll < 100); stay put if not.
		self.state()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn saved_matrix_is_square_and_sized() {
		assert_eq!(save().shape(), &[3, 3]);
	}

	#[test]
	fn every_saved_row_covers_all_probabilities() {
		let matrix = save();
		for row in 0..3 {
			let sum: u16 = matrix.row(row).iter().map(|&p| u16::from(p)).sum();
			assert_eq!(sum, 100, "row {row} must account for all 100%");
		}
	}

	#[test]
	fn load_matches_trait_thresholds() -> Result<(), String> {
		let matrix = save();
		for start in StateKind::ALL {
			for percent in 0..100u8 {
				let input = Probability::exact(percent);
				let via_traits = StateKind::from(&AnyMachine::from(start).decide(input));
				let mut via_matrix: MatrixMachine = match load(matrix.clone(), start) {
					Ok(machine) => machine,
					Err(e) => return Err(format!("valid matrix: {e}")),
				};
				assert_eq!(via_matrix.step(input), via_traits, "start {start:?}, p {percent}");
			}
		}
		return Ok(());
	}

	#[test]
	fn load_rejects_non_square() {
		let matrix = Array2::<u8>::zeros((2, 3));
		assert_eq!(load(matrix, StateKind::Rain), Err(MatrixError::NotSquare { rows: 2, cols: 3 }));
	}

	#[test]
	fn load_rejects_wrong_size() {
		let mut matrix = Array2::<u8>::zeros((2, 2));
		matrix.row_mut(0)[0] = 100;
		matrix.row_mut(1)[1] = 100;
		assert_eq!(
			load(matrix, StateKind::Rain),
			Err(MatrixError::WrongSize { expected: 3, found: 2 })
		);
	}

	#[test]
	fn load_rejects_row_that_misses_probabilities() {
		// Row 0 sums to 90% — 10% of outcomes unaccounted for.
		let mut matrix = Array2::<u8>::zeros((3, 3));
		matrix.row_mut(0)[0] = 90;
		for row in 1..3 {
			matrix.row_mut(row)[row] = 100;
		}
		assert_eq!(load(matrix, StateKind::Rain), Err(MatrixError::RowSum { row: 0, sum: 90 }));
	}

	#[test]
	fn round_trip_matrix_out_and_back() -> Result<(), String> {
		let original = save();
		let machine: MatrixMachine = match load(original.clone(), StateKind::Clear) {
			Ok(machine) => machine,
			Err(e) => return Err(format!("valid matrix: {e}")),
		};
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
	fn machine_state_reports_current_row() -> Result<(), String> {
		let machine: MatrixMachine = match load(save(), StateKind::Snow) {
			Ok(machine) => machine,
			Err(e) => return Err(format!("valid matrix: {e}")),
		};
		assert_eq!(machine.state(), StateKind::Snow);
		return Ok(());
	}

	#[test]
	fn display_renders_not_square() {
		let msg = MatrixError::NotSquare { rows: 2, cols: 3 }.to_string();
		assert_eq!(msg, "transition matrix must be square, got 2x3");
	}

	#[test]
	fn display_renders_wrong_size() {
		let msg = MatrixError::WrongSize { expected: 3, found: 2 }.to_string();
		assert_eq!(msg, "expected a 3x3 matrix, got 2x2");
	}

	#[test]
	fn display_renders_row_sum() {
		let msg = MatrixError::RowSum { row: 0, sum: 90 }.to_string();
		assert_eq!(msg, "row 0 probabilities sum to 90%, must be 100%");
	}

	#[test]
	fn matrix_error_is_std_error() {
		// Exercises the `impl std::error::Error`: usable as a trait object, no source.
		let err: Box<dyn std::error::Error> = Box::new(MatrixError::RowSum { row: 1, sum: 50 });
		assert!(err.source().is_none());
	}

	#[test]
	fn step_input_at_or_above_100_stays_put() -> Result<(), String> {
		// roll never reaches acc (max 100), so the loop falls through to the
		// "stay put" fallback and the current state is unchanged.
		let mut machine: MatrixMachine = match load(save(), StateKind::Clear) {
			Ok(machine) => machine,
			Err(e) => return Err(format!("valid matrix: {e}")),
		};
		assert_eq!(machine.step(Probability::exact(100)), StateKind::Clear);
		assert_eq!(machine.state(), StateKind::Clear);
		return Ok(());
	}
}
