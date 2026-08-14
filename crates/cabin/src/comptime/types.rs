use crate::comptime::storage::{Evaluated, Pointer};

pub struct Type {
	editable: bool,
	inner: Pointer<Evaluated>,
}
