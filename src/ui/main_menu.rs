use crate::app_state::AppState;
use crate::prelude::constants::NORMAL_FONT;
use crate::prelude::{MenuName, ToggleMenuMessage};
use crate::shared::CustomInteraction;
use crate::ui::shared::{despawn_menu, menu_base_node, spawn_background, spawn_button, spawn_logo};
use bevy::log::*;
use bevy::prelude::{
  AlignItems, App, AssetServer, Changed, Commands, Component, Entity, FlexDirection, IntoScheduleConfigs,
  JustifyContent, MessageReader, MessageWriter, NextState, Node, OnEnter, OnExit, Plugin, Query, Res, ResMut, Update,
  With, default, in_state, px,
};

/// Plugin that provides and manages the main menu UI.
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(OnEnter(AppState::Preparing), spawn_main_menu_system)
      .add_systems(
        Update,
        (handle_button_interactions_system, handle_toggle_menu_message).run_if(in_state(AppState::Preparing)),
      )
      .add_systems(OnExit(AppState::Preparing), despawn_main_menu_system);
  }
}

/// Marker component for the root of the main menu. Used for despawning.
#[derive(Component)]
struct MainMenuRoot;

/// Marker component for the Play Online button in the main menu.
#[derive(Component)]
struct PlayOnlineButton;

/// Marker component for the Play Local button in the main menu.
#[derive(Component)]
struct PlayLocalButton;

/// Marker component for the Exit button in the main menu.
#[derive(Component)]
struct ExitButton;

/// System to spawn the main menu UI including the background image.
fn spawn_main_menu_system(mut commands: Commands, asset_server: Res<AssetServer>) {
  spawn_main_menu(&mut commands, &asset_server);
}

fn spawn_main_menu(commands: &mut Commands, asset_server: &AssetServer) {
  let background_image = asset_server.load("images/background_menu_main.png");
  let logo_image = asset_server.load("images/logo.png");

  // Background & logo
  spawn_background(commands, MainMenuRoot, background_image);
  spawn_logo(commands, MainMenuRoot, logo_image);

  // Main Menu UI
  commands
    .spawn(menu_base_node(MainMenuRoot, "Main Menu".to_string()))
    .with_children(|parent| {
      parent
        .spawn(Node {
          flex_direction: FlexDirection::Column,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          row_gap: px(20.),
          ..default()
        })
        .with_children(|parent| {
          #[cfg(feature = "online")]
          spawn_button(parent, &asset_server, PlayOnlineButton, "Play Online", 300, NORMAL_FONT);
          spawn_button(parent, &asset_server, PlayLocalButton, "Play Local", 300, NORMAL_FONT);
          spawn_button(parent, &asset_server, ExitButton, "Exit", 300, NORMAL_FONT);
        });
    });
}

/// System to handle all main menu button interactions.
fn handle_button_interactions_system(
  mut commands: Commands,
  mut exit_button_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<ExitButton>)>,
  mut play_local_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<PlayLocalButton>)>,
  mut play_online_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<PlayOnlineButton>)>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
  menu_root_query: Query<Entity, With<MainMenuRoot>>,
  mut next_state: ResMut<NextState<AppState>>,
) {
  for interaction in &mut exit_button_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Exit\"");
      std::process::exit(0);
    }
  }

  for interaction in &mut play_local_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Play Local\"");
      next_state.set(AppState::Initialising);
      despawn_menu(&mut commands, &menu_root_query);
    }
  }

  for interaction in &mut play_online_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Play Online\"");
      toggle_menu_message.write(ToggleMenuMessage::set(MenuName::PlayOnlineMenu));
    }
  }
}

/// System to handle toggling the main menu based on received messages.
fn handle_toggle_menu_message(
  mut commands: Commands,
  mut messages: MessageReader<ToggleMenuMessage>,
  menu_root_query: Query<Entity, With<MainMenuRoot>>,
  asset_server: Res<AssetServer>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::MainMenu => spawn_main_menu(&mut commands, &asset_server),
      _ => despawn_menu(&mut commands, &menu_root_query),
    }
  }
}

/// Despawns all elements with the [`MainMenuRoot`] component.
fn despawn_main_menu_system(mut commands: Commands, menu_root_query: Query<Entity, With<MainMenuRoot>>) {
  despawn_menu(&mut commands, &menu_root_query);
}
