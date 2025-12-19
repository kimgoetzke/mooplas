use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, DEFAULT_FONT, HEADER_FONT, NORMAL_FONT};
use crate::prelude::{CustomInteraction, MenuName};
use crate::shared::ToggleMenuMessage;
use crate::ui::shared::{despawn_menu, menu_base_node, spawn_background, spawn_button};
use bevy::app::{App, Plugin};
use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::log::debug;
use bevy::prelude::{
  AlignItems, Changed, Commands, Component, Entity, FlexDirection, IntoScheduleConfigs, JustifyContent, MessageReader,
  MessageWriter, Node, Query, Res, Text, TextColor, TextFont, TextShadow, Update, With, default, in_state, px,
};

/// A plugin to manage the play online menu UI. Players can choose to host or join an online game from this menu.
pub struct PlayOnlineMenuPlugin;

impl Plugin for PlayOnlineMenuPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(
      Update,
      (handle_toggle_menu_message, handle_button_interactions_system).run_if(in_state(AppState::Preparing)),
    );
  }
}

/// Marker component for the root of the main menu. Used for despawning.
#[derive(Component)]
struct PlayOnlineMenuRoot;

/// Marker component for the back button in the play online menu.
#[derive(Component)]
struct BackButton;

/// Marker component for the host game button in the play online menu.
#[derive(Component)]
struct HostGameButton;

// Marker component for the join game button in the play online menu.
#[derive(Component)]
struct JoinGameButton;

/// System to handle toggling the play online menu based on received messages.
fn handle_toggle_menu_message(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut messages: MessageReader<ToggleMenuMessage>,
  menu_root_query: Query<Entity, With<PlayOnlineMenuRoot>>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::PlayOnlineMenu => spawn_menu(&mut commands, &asset_server),
      _ => despawn_menu(&mut commands, &menu_root_query),
    }
  }
}

fn spawn_menu(commands: &mut Commands, asset_server: &AssetServer) {
  let font = asset_server.load(DEFAULT_FONT);
  let heading_font = font.clone();
  let background_image = asset_server.load("images/background_menu_main.png");

  // Background
  spawn_background(commands, PlayOnlineMenuRoot, background_image);

  // Play online UI
  commands
    .spawn(menu_base_node(PlayOnlineMenuRoot, "Play Online Menu".to_string()))
    .with_children(|parent| {
      // Title
      parent.spawn((
        Text::new("Mooplas"),
        TextFont {
          font: heading_font.clone(),
          font_size: HEADER_FONT,
          ..default()
        },
        TextColor(Color::from(ACCENT_COLOUR)),
        TextShadow::default(),
      ));

      // Buttons
      parent
        .spawn(Node {
          flex_direction: FlexDirection::Column,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          row_gap: px(20.),
          ..default()
        })
        .with_children(|parent| {
          spawn_button(parent, &asset_server, HostGameButton, "Host Game", 300, NORMAL_FONT);
          spawn_button(parent, &asset_server, JoinGameButton, "Join Game", 300, NORMAL_FONT);
          spawn_button(parent, &asset_server, BackButton, "Back", 300, NORMAL_FONT);
        });
    });
}

fn handle_button_interactions_system(
  mut host_game_button: Query<&CustomInteraction, (Changed<CustomInteraction>, With<HostGameButton>)>,
  mut join_game_button: Query<&CustomInteraction, (Changed<CustomInteraction>, With<JoinGameButton>)>,
  mut back_button_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<BackButton>)>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
) {
  for interaction in &mut host_game_button {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Host Game\"");
      toggle_menu_message.write(ToggleMenuMessage::set(MenuName::HostGameMenu));
    }
  }

  for interaction in &mut join_game_button {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Join Game\"");
      toggle_menu_message.write(ToggleMenuMessage::set(MenuName::JoinGameMenu));
    }
  }

  for interaction in &mut back_button_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Back\"");
      toggle_menu_message.write(ToggleMenuMessage::set(MenuName::MainMenu));
    }
  }
}
