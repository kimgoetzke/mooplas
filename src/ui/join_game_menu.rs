#![cfg(feature = "online")]

use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, DEFAULT_FONT, HEADER_FONT, NORMAL_FONT, TEXT_COLOUR};
use crate::prelude::{ConnectionInfoMessage, CustomInteraction, MenuName};
use crate::shared::ToggleMenuMessage;
use crate::shared::constants::SMALL_FONT;
use crate::ui::shared::{despawn_menu, menu_base_node, spawn_background, spawn_button};
use bevy::app::{App, Plugin};
use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::log::debug;
use bevy::prelude::{
  AlignItems, Changed, Commands, Component, Entity, FlexDirection, IntoScheduleConfigs, JustifyContent, MessageReader,
  MessageWriter, Node, Query, Res, Text, TextColor, TextFont, TextShadow, Update, With, default, in_state, percent, px,
};

/// A plugin to manage the host game menu UI. Only included with the "online" feature.
pub struct JoinGameMenuPlugin;

impl Plugin for JoinGameMenuPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(
      Update,
      (handle_toggle_menu_message, handle_button_interactions_system).run_if(in_state(AppState::Preparing)),
    );
  }
}

/// Marker component for the root of the menu. Used for despawning.
#[derive(Component)]
struct JoinGameMenuRoot;

/// Marker component for the connect button.
#[derive(Component)]
struct ConnectButton;

/// Marker component for the back button.
#[derive(Component)]
struct BackButton;

/// System to handle toggling the play online menu based on received messages.
fn handle_toggle_menu_message(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut messages: MessageReader<ToggleMenuMessage>,
  menu_root_query: Query<Entity, With<JoinGameMenuRoot>>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::JoinGameMenu => spawn_menu(&mut commands, &asset_server),
      _ => despawn_menu(&mut commands, &menu_root_query),
    }
  }
}

fn spawn_menu(commands: &mut Commands, asset_server: &AssetServer) {
  let font = asset_server.load(DEFAULT_FONT);
  let heading_font = font.clone();
  let background_image = asset_server.load("images/background_menu_main.png");

  // Background
  spawn_background(commands, JoinGameMenuRoot, background_image.clone());

  // Host game UI
  commands
    .spawn(menu_base_node(JoinGameMenuRoot, "Join Game Menu".to_string()))
    .with_children(|parent| {
      parent
        .spawn(Node {
          width: percent(100.0),
          height: percent(100.0),
          flex_direction: FlexDirection::Column,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          row_gap: px(20),
          ..default()
        })
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

          // Text & button
          parent
            .spawn(Node {
              flex_direction: FlexDirection::Column,
              justify_content: JustifyContent::Center,
              align_items: AlignItems::Center,
              row_gap: px(20.),
              ..default()
            })
            .with_children(|parent| {
              parent.spawn((
                Text::new("Enter the host address below:"),
                TextFont {
                  font: heading_font.clone(),
                  font_size: SMALL_FONT,
                  ..default()
                },
                TEXT_COLOUR,
                TextShadow::default(),
              ));

              // TODO: Add input field for host address here

              spawn_button(parent, &asset_server, ConnectButton, "Connect", 300, NORMAL_FONT);
              spawn_button(parent, &asset_server, BackButton, "Back", 300, NORMAL_FONT);
            });
        });
    });
}

// TODO: Refactor this system to read the input field value when the connect button is pressed and send the message
//  with the provided server address and port.
fn handle_button_interactions_system(
  mut connect_button_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<ConnectButton>)>,
  mut back_button_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<BackButton>)>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
  mut connection_info_message: MessageWriter<ConnectionInfoMessage>,
) {
  for interaction in &mut connect_button_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Connect\"");
      connection_info_message.write(ConnectionInfoMessage {
        server_address: "".to_string(),
        server_port: 0,
      });
    }
  }

  for interaction in &mut back_button_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Back\"");
      toggle_menu_message.write(ToggleMenuMessage::set(MenuName::PlayOnlineMenu));
    }
  }
}
