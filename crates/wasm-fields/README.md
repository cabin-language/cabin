# `wasm-fields`

A small procedural macro for marking Rust struts that contain non-Copy fields with `wasm_bindgen`.

## Usage

## The Problem

When marking a struct with `#[wasm_bindgen]` that has, for example, a `String` field:

```rust
	#[wasm_bindgen]
	pub struct Example {
		pub text: String,
	}
```

An error is emitted:

```
error[E0277]: the trait bound `String: std::marker::Copy` is not satisfied
 --> tests/test.rs:8:13
  |
6 |     #[wasm_bindgen]
  |     --------------- in this procedural macro expansion
7 |     pub struct Example {
8 |         pub text: String,
  |                   ^^^^^^ the trait `std::marker::Copy` is not implemented for `String`
  |
note: required by a bound in `assert_copy`
 --> tests/test.rs:6:2
  |
6 |     #[wasm_bindgen]
  |     ^^^^^^^^^^^^^^^ required by this bound in `assert_copy`
  = note: this error originates in the derive macro `wasm_bindgen::__rt::BindgenedStruct` (in Nightly builds, run with -Z macro-backtrace for more info)
```

This is because `#[wasm_bindgen]` requires the types of all struct fields to `impl Copy`.

## The Solution

The solution is to add an `impl` for the struct marked `#[wasm_bindgen]` that has getter and setter methods (marked with `#[wasm_bindgen(setter)]` and `#[wasm_bindgen(getter)]`), and make the field private:

```rust
#[wasm_bindgen]
pub struct Example {
	text: String,
}

#[wasm_bindgen]
impl Example {
	#[wasm_bindgen(setter)]
	pub fn set_text(&mut self, text: String) {
		self.text = text;
	}

	#[wasm_bindgen(getter)]
	pub fn text(&self) -> String {
		self.text.clone()
	}
}
```

This generates the following types:

```typescript
export class Example {
  private constructor();
  free(): void;
  text: string
}
```

These can quickly become tiresome to write, so `#[wasm_fields]` generates them for you; The above `impl` is equivalent to simply writing:

```rust
#[wasm_bindgen]
#[wasm_fields]
pub struct Example {
	text: String,
}
```
