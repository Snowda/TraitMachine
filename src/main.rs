use rand;
use rand::prelude::IteratorRandom;

trait Print {
	fn debug(&self) -> &Self;
}

trait Convert {
	fn convert(&self) -> State;
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum State {
	Rain,
	Clear,
	Snow
}

impl State {
	pub fn iterator() -> impl Iterator<Item = State> {
        static WEATHERS: [State; 3] = [
            State::Rain,
            State::Clear,
            State::Snow,
        ];
        WEATHERS.iter().copied() // .copied() is used to get owned values if the enum implements Copy
    }
}

struct Machine {
	state: State
}

impl Default for Machine {
	fn default() -> Self {
		Machine { state: State::Clear }
	}
}

impl Machine {
	fn convert(&mut self) -> &Self {
		self.state = self.state.convert();
		self
	}

	fn debug(&self) -> &Self {
		self.state.debug();
		self
	}
}

const VARIANTS: &[State] = &[State::Clear, State::Rain, State::Snow];

fn random_state() -> State {
	let mut rng = rand::thread_rng();
	State::iterator().choose(&mut rng).unwrap()
}

impl Convert for State {
	fn convert(&self) -> State {
		match &self {
            State::Clear => random_state(),
            State::Rain => State::Clear,
            State::Snow => State::Rain,
		}
	}
}

impl Print for State {
	fn debug(&self) -> &Self {
		match self {
			State::Clear => println!("Clear"),
			State::Rain => println!("Rain"),
			State::Snow => println!("Snow"),
		}
		self
	}
}

#[tokio::main]
async fn main() {
	let mut machine: Machine = Machine::default();
	machine.debug();

	for _ in 1..10 {
		machine.convert();
		machine.debug();
	}
}