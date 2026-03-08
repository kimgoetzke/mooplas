mod online;

#[cfg(feature = "online")]
mod utils;

#[cfg(feature = "online")]
mod structs;

#[cfg(feature = "online")]
mod server;

#[cfg(feature = "online")]
mod client;

#[cfg(feature = "online_renet")]
mod renet;

#[cfg(feature = "online_matchbox")]
mod matchbox;

pub use online::OnlinePlugin;
