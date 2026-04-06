use crate::app_state::AppState;
use crate::prelude::constants::{NORMAL_FONT, SMALL_FONT};
use crate::prelude::{
  AvailableControlSchemes, ContinueMessage, CustomInteraction, ExitLobbyMessage, RegisteredPlayers, Settings,
  TouchControlsToggledMessage,
};
use crate::ui::in_game_ui::in_game_ui;
use crate::ui::shared::spawn_button;
use bevy::app::{App, Plugin, Update};
use bevy::asset::AssetServer;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::log::info;
use bevy::prelude::{
  AlignItems, Changed, ChildOf, Commands, Component, Entity, IntoScheduleConfigs, JustifyContent, MessageReader,
  MessageWriter, MonitorSelection, Node, PositionType, Query, Res, ResMut, Single, Window, With, default, in_state, px,
};
use mooplas_networking::prelude::NetworkRole;

/// A plugin that manages all in-game buttons and their related systems, such as toggling touch controls and fullscreen
/// mode, exiting the lobby, and continuing after game over.
pub struct InGameButtonsPlugin;

impl Plugin for InGameButtonsPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        (
          handle_touch_controls_toggled_message,
          toggle_touch_controls_button_system,
          toggle_fullscreen_button_system,
          exit_button_system,
        )
          .run_if(in_state(AppState::Registering)),
      )
      .add_systems(
        Update,
        continue_button_system
          .run_if(in_state(AppState::Registering))
          .run_if(|network_role: Res<NetworkRole>| !network_role.is_client()),
      )
      .add_systems(Update, continue_button_system.run_if(in_state(AppState::GameOver)));
  }
}

/// Marker component for the touch controls toggle button.
#[derive(Component)]
struct ToggleTouchControlsButton;

/// Marker component for the fullscreen toggle button.
#[derive(Component)]
struct ToggleFullscreenButton;

/// Marker component for the exit button.
#[derive(Component)]
struct ExitButton;

/// Marker component for the touch continue button.
#[derive(Component)]
struct ContinueButton;

/// A system that handles messages toggling touch controls to update the lobby UI's prompts accordingly. Makes sure that
/// the prompt doesn't ask for a key press when touch controls are enabled and vice versa.
fn handle_touch_controls_toggled_message(
  mut commands: Commands,
  mut messages: MessageReader<TouchControlsToggledMessage>,
  lobby_ui_root_query: Query<Entity, With<in_game_ui::LobbyUiRoot>>,
  settings: Res<Settings>,
  asset_server: Res<AssetServer>,
  available_control_schemes: Res<AvailableControlSchemes>,
  registered_players: Res<RegisteredPlayers>,
  network_role: Res<NetworkRole>,
) {
  for _ in messages.read() {
    for entity in &lobby_ui_root_query {
      commands.entity(entity).despawn();
    }
    in_game_ui::spawn_lobby_ui(
      &mut commands,
      &settings,
      &asset_server,
      &available_control_schemes,
      &registered_players,
      &network_role,
    );
  }
}

/// A system that toggles touch controls when the corresponding button is pressed.
fn toggle_touch_controls_button_system(
  mut query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<ToggleTouchControlsButton>)>,
  mut touch_controls_toggled_message: MessageWriter<TouchControlsToggledMessage>,
  mut settings: ResMut<Settings>,
) {
  for interaction in &mut query {
    if *interaction == CustomInteraction::Released {
      settings.general.enable_touch_controls = !settings.general.enable_touch_controls;
      touch_controls_toggled_message.write(TouchControlsToggledMessage::new(settings.general.enable_touch_controls));
      info!(
        "[Button] Set touch controls to [{:?}]",
        settings.general.enable_touch_controls
      );
    }
  }
}

/// A system that toggles the window mode when the corresponding button is pressed.
fn toggle_fullscreen_button_system(
  mut query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<ToggleFullscreenButton>)>,
  mut window: Single<&mut Window>,
) {
  for interaction in &mut query {
    if *interaction == CustomInteraction::Released {
      window.mode = match window.mode {
        bevy::window::WindowMode::Windowed => bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        _ => bevy::window::WindowMode::Windowed,
      };
      info!("[Button] Set window mode to [{:?}]", window.mode);
    }
  }
}

/// A system that exists the current game when the exit button is pressed.
fn exit_button_system(
  mut query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<ExitButton>)>,
  mut exit_lobby_message: MessageWriter<ExitLobbyMessage>,
) {
  for interaction in &mut query {
    if *interaction == CustomInteraction::Released {
      exit_lobby_message.write(ExitLobbyMessage::default());
      info!("[Button] Pressed exit button");
    }
  }
}

/// A system that handles the continue button press by sending [`ContinueMessage`].
fn continue_button_system(
  mut query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<ContinueButton>)>,
  mut continue_message: MessageWriter<ContinueMessage>,
) {
  for interaction in &mut query {
    if *interaction == CustomInteraction::Released {
      continue_message.write(ContinueMessage);
      info!("[Button] Pressed continue button");
    }
  }
}

pub(crate) fn spawn_in_game_buttons(asset_server: &Res<AssetServer>, parent: &mut RelatedSpawnerCommands<ChildOf>) {
  parent
    .spawn(Node {
      width: px(180),
      height: px(100),
      position_type: PositionType::Relative,
      align_items: AlignItems::Center,
      justify_content: JustifyContent::Center,
      ..default()
    })
    .with_children(|parent| {
      spawn_button(
        parent,
        asset_server,
        ToggleTouchControlsButton,
        "Touch Controls",
        170,
        SMALL_FONT,
      );
    });

  parent
    .spawn((Node {
      width: px(160),
      height: px(100),
      position_type: PositionType::Relative,
      align_items: AlignItems::Center,
      justify_content: JustifyContent::Center,
      ..default()
    },))
    .with_children(|parent| {
      spawn_button(
        parent,
        asset_server,
        ToggleFullscreenButton,
        "Fullscreen",
        150,
        SMALL_FONT,
      );
    });

  parent
    .spawn(Node {
      width: px(110),
      height: px(100),
      position_type: PositionType::Relative,
      align_items: AlignItems::Center,
      justify_content: JustifyContent::Center,
      ..default()
    })
    .with_children(|parent| {
      spawn_button(parent, asset_server, ExitButton, "Exit", 100, SMALL_FONT);
    });
}

pub(crate) fn spawn_continue_button(asset_server: &Res<AssetServer>, parent: &mut RelatedSpawnerCommands<ChildOf>) {
  spawn_button(parent, asset_server, ContinueButton, "HERE", 170, NORMAL_FONT);
}
