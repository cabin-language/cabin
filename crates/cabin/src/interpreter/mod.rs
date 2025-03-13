use crate::{
	io::{IoReader, IoWriter},
	Context,
};

pub trait Runtime {
	type Output;
	fn evaluate_at_runtime<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output;
}
