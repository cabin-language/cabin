use crate::Context;

pub trait Unzip3<T, U, V> {
	fn unzip(self) -> (T, U, V);
}

impl<T, U, V> Unzip3<Option<T>, Option<U>, Option<V>> for Option<(T, U, V)> {
	fn unzip(self) -> (Option<T>, Option<U>, Option<V>) {
		match self {
			Some((t, u, v)) => (Some(t), Some(u), Some(v)),
			None => (None, None, None),
		}
	}
}

pub trait ExitScope {
	fn or_exit_scope(self, context: &mut Context) -> Self;
}

impl<T, E> ExitScope for Result<T, E> {
	fn or_exit_scope(self, context: &mut Context) -> Self {
		self.inspect_err(|_| {
			context.scope.exit_scope(context.scope.unique_id()).unwrap();
		})
	}
}

pub trait Indent {
	fn indented(&self) -> String;
}

impl<S: AsRef<str>> Indent for S {
	fn indented(&self) -> String {
		self.as_ref().lines().map(|line| format!("\t{line}")).collect::<Vec<_>>().join("\n")
	}
}
