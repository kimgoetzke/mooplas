use crate::native::RenetClientVisualiser;
use crate::prelude::SHOW_VISUALISERS_BY_DEFAULT;
use bevy::app::{Plugin, Update};
use bevy::input::common_conditions::input_toggle_active;
use bevy::log::warn;
use bevy::prelude::{IntoScheduleConfigs, KeyCode, Res, ResMut, resource_exists};
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_renet::RenetClient;

/// A Bevy plugin that adds a system to update and display the Renet client visualiser when toggled by the user.
pub struct ClientVisualiserPlugin;

impl Plugin for ClientVisualiserPlugin {
  fn build(&self, app: &mut bevy::prelude::App) {
    app.add_systems(
      Update,
      update_client_visualiser_system
        .run_if(resource_exists::<RenetClientVisualiser>)
        .run_if(input_toggle_active(SHOW_VISUALISERS_BY_DEFAULT, KeyCode::F2)),
    );
  }
}

/// System that updates and displays the Renet client visualiser when toggled by the user.
fn update_client_visualiser_system(
  mut egui_contexts: EguiContexts,
  mut visualiser: ResMut<RenetClientVisualiser>,
  client: Res<RenetClient>,
) {
  visualiser.add_network_info(client.network_info());
  if let Ok(ctx) = egui_contexts.ctx_mut() {
    visualiser.show_window(ctx);
  } else {
    warn!("Failed to get Egui context for Renet client visualiser");
  }
}
