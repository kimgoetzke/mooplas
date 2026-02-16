mod client;
mod client_visualiser;
mod messages;
mod resources;
mod server;
mod server_visualiser;
mod utils;

use crate::prelude::ChannelType;
use bevy_renet::renet::DefaultChannel;
pub use client::*;
pub use client_visualiser::ClientVisualiserPlugin;
pub use messages::*;
pub use resources::*;
pub use server::*;
pub use server_visualiser::ServerVisualiserPlugin;
pub use utils::*;

pub(crate) const PROTOCOL_ID: u64 = 1000;

pub type RawClientId = bevy_renet::renet::ClientId;

impl From<DefaultChannel> for ChannelType {
  fn from(value: DefaultChannel) -> Self {
    match value {
      DefaultChannel::Unreliable => ChannelType::Unreliable,
      DefaultChannel::ReliableOrdered => ChannelType::ReliableOrdered,
      DefaultChannel::ReliableUnordered => ChannelType::ReliableUnordered,
    }
  }
}

impl From<ChannelType> for DefaultChannel {
  fn from(value: ChannelType) -> Self {
    match value {
      ChannelType::Unreliable => DefaultChannel::Unreliable,
      ChannelType::ReliableOrdered => DefaultChannel::ReliableOrdered,
      ChannelType::ReliableUnordered => DefaultChannel::ReliableUnordered,
    }
  }
}
