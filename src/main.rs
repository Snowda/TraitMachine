use std::io::{self, Write};

use traitmachine::{AnyMachine, Clear, Machine, Probability};

fn main() -> io::Result<()> {
	let mut out = io::stdout().lock();
	let mut machine = AnyMachine::Clear(Machine::new(Clear));
	writeln!(out, "start: {}", machine.name())?;

	// exact() is deterministic user input (Clear's 66..100 band => Snow); roll() samples.
	machine = machine.decide(Probability::exact(80));
	writeln!(out, "{}", machine.name())?;

	for _ in 0..8 {
		machine = machine.decide(Probability::roll());
		writeln!(out, "{}", machine.name())?;
	}
	return Ok(());
}
