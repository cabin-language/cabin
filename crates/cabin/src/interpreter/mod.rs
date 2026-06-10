use crate::Context;

pub trait Runtime {
	type Output;
	fn evaluate_at_runtime(self, context: &mut Context) -> Self::Output;
}
