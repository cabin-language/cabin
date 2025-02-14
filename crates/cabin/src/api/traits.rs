use try_as::traits::TryAsRef;

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
}

impl<T> TryAs for T {}
