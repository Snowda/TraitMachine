use std::io::{self, Write};

use traitmachine::{AnyMachine, Clear, Machine, Probability};

fn main() -> io::Result<()> {
	let mut out: io::StdoutLock<'_> = io::stdout().lock();
	let mut machine: AnyMachine = AnyMachine::Clear(Machine::new(Clear));
	writeln!(out, "start: {}", machine.name())?;

	// exact() is deterministic user input (Clear's 66..100 band => Snow). roll() samples at random.
	machine = machine.decide(Probability::exact(80));
	writeln!(out, "{}", machine.name())?;

	(0..8).try_fold(machine, |current: AnyMachine, _| -> io::Result<AnyMachine> {
		let next: AnyMachine = current.decide(Probability::roll());
		writeln!(out, "{}", next.name())?;
		return Ok(next);
	})?;
	return Ok(());
}
