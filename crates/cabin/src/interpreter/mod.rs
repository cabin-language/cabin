use crate::{io::Io, Context};

pub trait Runtime {
	type Output;
	fn evaluate_at_runtime<System: Io>(self, context: &mut Context<System>) -> Self::Output;
}
