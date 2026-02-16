#[cfg(all(feature = "renet", feature = "matchbox"))]
compile_error!("Features `renet` and `matchbox` are mutually exclusive.");

#[cfg(not(any(feature = "renet", feature = "matchbox")))]
compile_error!("You must enable either `renet` or `matchbox`.");

mod shared;

#[cfg(feature = "matchbox")]
mod matchbox;

#[cfg(feature = "renet")]
mod renet;

mod backend {
  #[cfg(feature = "renet")]
  pub use super::renet::*;

  #[cfg(feature = "matchbox")]
  pub use super::matchbox::*;
}

pub mod prelude {
  pub use super::backend::*;
  pub use super::shared::*;
}
