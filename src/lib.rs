//! Trait-driven state machine: every transition goes through `decide(Probability)`.
//! States are zero-sized types; each state's `Resolve` impl fixes its permitted
//! next states (`Outcome`) at compile time while the input picks among them.
//! ```
//! use traitmachine::{Machine, Clear, Probability, ClearNext};
//! assert!(matches!(Machine::new(Clear).decide(Probability::exact(10)), ClearNext::Rain(_)));
//! assert!(matches!(Machine::new(Clear).decide(Probability::exact(50)), ClearNext::Clear(_)));
//! assert!(matches!(Machine::new(Clear).decide(Probability::exact(90)), ClearNext::Snow(_)));
//! ```

use rand::RngExt;

pub mod matrix;
pub use matrix::{load, save, MatrixError, MatrixMachine, StateKind};

pub trait Named {
	const NAME: &'static str;
}

/// An input percent in `0..100`, compared against a state's internal thresholds.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Probability(u8);

impl Probability {
	/// A user-supplied, deterministic input.
	#[must_use]
	pub fn exact(percent: u8) -> Self {
		Probability(percent)
	}

	/// A statistical input sampled uniformly in `0..100`.
	#[must_use]
	pub fn roll() -> Self {
		Probability(rand::rng().random_range(0u8..100))
	}

	#[must_use]
	pub fn percent(self) -> u8 {
		self.0
	}
}

/// Maps a `Probability` to one of the state's `Outcome` variants (its permitted next states).
pub trait Resolve {
	type Outcome;
	fn resolve(self, input: Probability) -> Self::Outcome;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rain;
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Clear;
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Snow;

impl Named for Rain {
	const NAME: &'static str = "Rain";
}
impl Named for Clear {
	const NAME: &'static str = "Clear";
}
impl Named for Snow {
	const NAME: &'static str = "Snow";
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RainNext {
	Clear(Machine<Clear>),
	Snow(Machine<Snow>),
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ClearNext {
	Rain(Machine<Rain>),
	Clear(Machine<Clear>),
	Snow(Machine<Snow>),
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SnowNext {
	Rain(Machine<Rain>),
	Snow(Machine<Snow>),
}

impl Resolve for Rain {
	type Outcome = RainNext;
	fn resolve(self, input: Probability) -> RainNext {
		match input.percent() {
			0..70 => RainNext::Clear(Machine::new(Clear)),
			_ => RainNext::Snow(Machine::new(Snow)),
		}
	}
}

impl Resolve for Clear {
	type Outcome = ClearNext;
	fn resolve(self, input: Probability) -> ClearNext {
		match input.percent() {
			0..33 => ClearNext::Rain(Machine::new(Rain)),
			33..66 => ClearNext::Clear(Machine::new(Clear)),
			_ => ClearNext::Snow(Machine::new(Snow)),
		}
	}
}

impl Resolve for Snow {
	type Outcome = SnowNext;
	fn resolve(self, input: Probability) -> SnowNext {
		match input.percent() {
			0..60 => SnowNext::Rain(Machine::new(Rain)),
			_ => SnowNext::Snow(Machine::new(Snow)),
		}
	}
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Machine<S> {
	state: S,
}

impl<S: Named> Machine<S> {
	#[must_use]
	pub fn new(state: S) -> Self {
		Self { state }
	}

	#[must_use]
	pub fn name(&self) -> &'static str {
		S::NAME
	}
}

impl<S: Resolve> Machine<S> {
	/// The unified transition; the input selects which `Outcome` variant results.
	#[must_use]
	pub fn decide(self, input: Probability) -> S::Outcome {
		self.state.resolve(input)
	}
}

/// Runtime witness of a machine in any state, carried across an open-ended loop.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AnyMachine {
	Rain(Machine<Rain>),
	Clear(Machine<Clear>),
	Snow(Machine<Snow>),
}

impl AnyMachine {
	#[must_use]
	pub fn name(&self) -> &'static str {
		match self {
			AnyMachine::Rain(m) => m.name(),
			AnyMachine::Clear(m) => m.name(),
			AnyMachine::Snow(m) => m.name(),
		}
	}

	/// Step once, mapping each typed outcome back into the runtime witness.
	#[must_use]
	pub fn decide(self, input: Probability) -> AnyMachine {
		match self {
			AnyMachine::Rain(m) => match m.decide(input) {
				RainNext::Clear(n) => AnyMachine::Clear(n),
				RainNext::Snow(n) => AnyMachine::Snow(n),
			},
			AnyMachine::Clear(m) => match m.decide(input) {
				ClearNext::Rain(n) => AnyMachine::Rain(n),
				ClearNext::Clear(n) => AnyMachine::Clear(n),
				ClearNext::Snow(n) => AnyMachine::Snow(n),
			},
			AnyMachine::Snow(m) => match m.decide(input) {
				SnowNext::Rain(n) => AnyMachine::Rain(n),
				SnowNext::Snow(n) => AnyMachine::Snow(n),
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// --- Probability ---------------------------------------------------------

	#[test]
	fn exact_round_trips_through_percent() {
		assert_eq!(Probability::exact(0).percent(), 0);
		assert_eq!(Probability::exact(42).percent(), 42);
		assert_eq!(Probability::exact(99).percent(), 99);
	}

	#[test]
	fn roll_stays_in_0_to_99() {
		// roll() samples 0..100; the upper bound is exclusive, so 99 is the max.
		for _ in 0..1_000 {
			let p = Probability::roll().percent();
			assert!(p < 100, "roll produced {p}, out of 0..100");
		}
	}

	// --- Named / Machine::name ----------------------------------------------

	#[test]
	fn machine_name_reports_state_const() {
		assert_eq!(Machine::new(Rain).name(), "Rain");
		assert_eq!(Machine::new(Clear).name(), "Clear");
		assert_eq!(Machine::new(Snow).name(), "Snow");
	}

	// --- Rain::resolve: 0..70 => Clear, else Snow ---------------------------

	#[test]
	fn rain_decides_clear_below_70() {
		assert!(matches!(Machine::new(Rain).decide(Probability::exact(0)), RainNext::Clear(_)));
		assert!(matches!(Machine::new(Rain).decide(Probability::exact(69)), RainNext::Clear(_)));
	}

	#[test]
	fn rain_decides_snow_at_and_above_70() {
		assert!(matches!(Machine::new(Rain).decide(Probability::exact(70)), RainNext::Snow(_)));
		assert!(matches!(Machine::new(Rain).decide(Probability::exact(99)), RainNext::Snow(_)));
	}

	// --- Clear::resolve: 0..33 Rain, 33..66 Clear, else Snow ----------------

	#[test]
	fn clear_decides_rain_below_33() {
		assert!(matches!(Machine::new(Clear).decide(Probability::exact(0)), ClearNext::Rain(_)));
		assert!(matches!(Machine::new(Clear).decide(Probability::exact(32)), ClearNext::Rain(_)));
	}

	#[test]
	fn clear_decides_clear_in_33_to_66() {
		assert!(matches!(Machine::new(Clear).decide(Probability::exact(33)), ClearNext::Clear(_)));
		assert!(matches!(Machine::new(Clear).decide(Probability::exact(65)), ClearNext::Clear(_)));
	}

	#[test]
	fn clear_decides_snow_at_and_above_66() {
		assert!(matches!(Machine::new(Clear).decide(Probability::exact(66)), ClearNext::Snow(_)));
		assert!(matches!(Machine::new(Clear).decide(Probability::exact(99)), ClearNext::Snow(_)));
	}

	// --- Snow::resolve: 0..60 => Rain, else Snow ----------------------------

	#[test]
	fn snow_decides_rain_below_60() {
		assert!(matches!(Machine::new(Snow).decide(Probability::exact(0)), SnowNext::Rain(_)));
		assert!(matches!(Machine::new(Snow).decide(Probability::exact(59)), SnowNext::Rain(_)));
	}

	#[test]
	fn snow_decides_snow_at_and_above_60() {
		assert!(matches!(Machine::new(Snow).decide(Probability::exact(60)), SnowNext::Snow(_)));
		assert!(matches!(Machine::new(Snow).decide(Probability::exact(99)), SnowNext::Snow(_)));
	}

	// --- AnyMachine::name ----------------------------------------------------

	#[test]
	fn any_machine_name_matches_wrapped_state() {
		assert_eq!(AnyMachine::Rain(Machine::new(Rain)).name(), "Rain");
		assert_eq!(AnyMachine::Clear(Machine::new(Clear)).name(), "Clear");
		assert_eq!(AnyMachine::Snow(Machine::new(Snow)).name(), "Snow");
	}

	// --- AnyMachine::decide: every arm of the runtime witness ---------------

	#[test]
	fn any_rain_steps_to_clear_then_snow() {
		let rain = AnyMachine::Rain(Machine::new(Rain));
		assert!(matches!(rain.decide(Probability::exact(0)), AnyMachine::Clear(_)));
		assert!(matches!(rain.decide(Probability::exact(70)), AnyMachine::Snow(_)));
	}

	#[test]
	fn any_clear_steps_to_all_three() {
		let clear = AnyMachine::Clear(Machine::new(Clear));
		assert!(matches!(clear.decide(Probability::exact(0)), AnyMachine::Rain(_)));
		assert!(matches!(clear.decide(Probability::exact(33)), AnyMachine::Clear(_)));
		assert!(matches!(clear.decide(Probability::exact(66)), AnyMachine::Snow(_)));
	}

	#[test]
	fn any_snow_steps_to_rain_then_snow() {
		let snow = AnyMachine::Snow(Machine::new(Snow));
		assert!(matches!(snow.decide(Probability::exact(0)), AnyMachine::Rain(_)));
		assert!(matches!(snow.decide(Probability::exact(60)), AnyMachine::Snow(_)));
	}
}
