use crate::{comptime::memory::VirtualPointer, Context};

pub(crate) trait Typed {
	fn get_type(&self, context: &mut Context) -> VirtualPointer;
}
