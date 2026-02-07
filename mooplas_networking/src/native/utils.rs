use crate::native::{RenetClientVisualiser, RenetServerVisualiser};
use bevy::prelude::Commands;
use bevy_renet::netcode::{NetcodeClientTransport, NetcodeServerTransport};
use bevy_renet::{RenetClient, RenetServer};

pub fn remove_all_resources(commands: &mut Commands) {
  commands.remove_resource::<RenetServer>();
  commands.remove_resource::<NetcodeServerTransport>();
  commands.remove_resource::<RenetClient>();
  commands.remove_resource::<NetcodeClientTransport>();
  commands.remove_resource::<RenetServerVisualiser>();
  commands.remove_resource::<RenetClientVisualiser>();
}
