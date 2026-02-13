mod native;
mod renet;
mod shared;
mod wasm;

pub mod prelude {
  #[cfg(feature = "native")]
  pub use crate::renet::*;

  #[cfg(feature = "wasm")]
  pub use crate::wasm::*;

  pub use crate::shared::*;
}
