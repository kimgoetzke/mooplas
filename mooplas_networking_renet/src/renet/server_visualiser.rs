use crate::renet::{RenetServerVisualiser, SHOW_VISUALISERS_BY_DEFAULT};
use bevy::app::Update;
use bevy::input::common_conditions::input_toggle_active;
use bevy::log::*;
use bevy::prelude::{App, IntoScheduleConfigs, KeyCode, On, Plugin, Res, ResMut, resource_exists};
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_renet::RenetServer;
use mooplas_networking::prelude::ServerEvent;

/// A Bevy plugin that adds a system to update and display the Renet server visualiser when toggled by the user.
pub struct ServerVisualiserPlugin;

impl Plugin for ServerVisualiserPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        update_server_visualiser_system
          .run_if(resource_exists::<RenetServerVisualiser>)
          .run_if(input_toggle_active(SHOW_VISUALISERS_BY_DEFAULT, KeyCode::F2)),
      )
      .add_observer(receive_server_events);
  }
}

/// System that updates and displays the Renet server visualiser when toggled by the user.
fn update_server_visualiser_system(
  mut egui_contexts: EguiContexts,
  mut visualiser: ResMut<RenetServerVisualiser>,
  server: Res<RenetServer>,
) {
  visualiser.update(&server);
  if let Ok(result) = egui_contexts.ctx_mut() {
    visualiser.show_window(result);
  } else {
    warn!("Failed to get Egui context for Renet server visualiser");
  }
}

fn receive_server_events(server_event: On<ServerEvent>, mut visualiser: Option<ResMut<RenetServerVisualiser>>) {
  match server_event.event() {
    ServerEvent::ClientConnected { client_id } => {
      info!("Adding client [{}] to visualiser", client_id);
      if let Some(visualiser) = visualiser.as_mut() {
        visualiser.add_client(client_id);
      }
    }
    ServerEvent::ClientDisconnected { client_id } => {
      info!("Removing client [{}] from visualiser", client_id);
      if let Some(visualiser) = visualiser.as_mut() {
        visualiser.remove_client(client_id);
      }
    }
    _ => { /* Ignored */ }
  }
}
