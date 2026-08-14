use try_as::traits::{TryAsMut, TryAsRef};

pub trait TryAs {
	/// Attempts to convert this enum variant into the given type. This is a generic wrapper
	/// around `try_as_ref`.
	///
	/// # Errors
	///
	/// If this enum is of the wrong variant.
	fn try_as<T>(&self) -> anyhow::Result<&T>
	where
		Self: TryAsRef<T>,
	{
		self.try_as_ref().ok_or_else(|| anyhow::anyhow!("Incorrect variant"))
	}

	fn try_mut<T>(&mut self) -> anyhow::Result<&mut T>
	where
		Self: TryAsMut<T>,
	{
		self.try_as_mut().ok_or_else(|| anyhow::anyhow!("Incorrect variant"))
	}

	fn is<T>(&self) -> bool
	where
		Self: TryAsRef<T>,
	{
		self.try_as::<T>().is_ok()
	}
}

impl<T> TryAs for T {}
