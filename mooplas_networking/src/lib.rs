mod native;
mod shared;
mod wasm;

pub mod prelude {
  pub use crate::shared::*;
}

#[cfg(feature = "wasm")]
pub use wasm::*;

// #[cfg(feature = "native")]
// pub use native::*;
