use bevy::prelude::Commands;
use bevy_matchbox::MatchboxSocket;
use mooplas_networking::prelude::{ClientNetworkingActive, ServerNetworkingActive};

/// Cleans up all networking resources for native platforms.
pub fn remove_all_matchbox_resources(commands: &mut Commands) {
  commands.remove_resource::<ClientNetworkingActive>();
  commands.remove_resource::<ServerNetworkingActive>();
  commands.remove_resource::<MatchboxSocket>();
}
