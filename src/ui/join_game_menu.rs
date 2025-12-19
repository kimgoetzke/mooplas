#![cfg(feature = "online")]

use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, DEFAULT_FONT, HEADER_FONT, NORMAL_FONT};
use crate::prelude::{ConnectionInfoMessage, CustomInteraction, MenuName};
use crate::shared::ToggleMenuMessage;
use crate::shared::constants::BUTTON_ALPHA_DEFAULT;
use crate::ui::shared::{despawn_menu, menu_base_node, spawn_background, spawn_button};
use bevy::app::{App, Plugin};
use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::log::debug;
use bevy::prelude::{
  AlignItems, Alpha, BackgroundColor, BorderColor, BorderRadius, Changed, Click, Commands, Component, Entity,
  FlexDirection, IntoScheduleConfigs, Justify, JustifyContent, MessageReader, MessageWriter, Name, Node, On, OnExit,
  Pointer, Query, Res, Text, TextColor, TextFont, TextShadow, UiRect, Update, With, default, in_state, percent, px,
};
use bevy_ui_text_input::actions::TextInputAction;
use bevy_ui_text_input::{SubmitText, TextInputMode, TextInputNode, TextInputPrompt, TextInputQueue};

/// A plugin to manage the game menu UI used to joining an online multiplayer game. Only included with the "online"
/// feature.
pub struct JoinGameMenuPlugin;

impl Plugin for JoinGameMenuPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        (
          handle_toggle_menu_message,
          handle_button_interactions_system,
          handle_submit_text_messages,
        )
          .run_if(in_state(AppState::Preparing)),
      )
      .add_systems(OnExit(AppState::Preparing), despawn_menu_system);
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

// TODO: Display error message on failed connection attempt
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
          width: percent(75),
          row_gap: px(20.),
          ..default()
        })
        .with_children(|parent| {
          // Input field
          let editor = parent
            .spawn((
              Name::new("Input Field"),
              TextInputNode {
                mode: TextInputMode::SingleLine,
                max_chars: Some(50),
                clear_on_submit: true,
                justification: Justify::Center,
                ..Default::default()
              },
              TextFont {
                font,
                font_size: NORMAL_FONT,
                ..Default::default()
              },
              TextColor(Color::from(ACCENT_COLOUR)),
              BorderRadius::all(px(10)),
              BorderColor::all(Color::from(tailwind::SLATE_500)),
              BackgroundColor(Color::from(tailwind::SLATE_500.with_alpha(BUTTON_ALPHA_DEFAULT))),
              TextInputPrompt::new("Paste connection string here..."),
              Node {
                width: percent(100),
                height: px(45.),
                padding: UiRect::all(px(10.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
              },
            ))
            .id();

          // Spacer to make sure the UI has the same height as all other menus
          parent.spawn((
            Name::new("Spacer"),
            Node {
              width: percent(100.0),
              height: px(0.0), // The node's row gap is sufficient here
              ..default()
            },
          ));

          // Button: Connect incl. submission observer
          let connect_button = spawn_button(parent, &asset_server, ConnectButton, "Connect", 300, NORMAL_FONT);
          parent.commands().entity(connect_button).observe(
            move |_: On<Pointer<Click>>, mut query: Query<&mut TextInputQueue>| {
              query.get_mut(editor).unwrap().add(TextInputAction::Submit);
            },
          );

          // Button: Back
          spawn_button(parent, &asset_server, BackButton, "Back", 300, NORMAL_FONT);
        });
    });
}

/// A system to handle button interactions, excluding the connect button which is handled via an observer because it
/// uses a message from an external crate.
//noinspection DuplicatedCode
fn handle_button_interactions_system(
  mut back_button_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<BackButton>)>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
) {
  for interaction in &mut back_button_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Back\"");
      toggle_menu_message.write(ToggleMenuMessage::set(MenuName::PlayOnlineMenu));
    }
  }
}

/// A system to handle submitted text input messages from the text input field. The message handled here comes from the
/// bevy_ui_text_input crate, not this application directly.
fn handle_submit_text_messages(
  mut messages: MessageReader<SubmitText>,
  mut connection_info_message: MessageWriter<ConnectionInfoMessage>,
) {
  for message in messages.read() {
    if message.text.is_empty() {
      debug!("Received empty connection string submission, ignoring...");
      continue;
    }
    debug!("Sending [ConnectionInfoMessage] with text: {:?}", message.text);
    connection_info_message.write(ConnectionInfoMessage {
      connection_string: message.text.to_string(),
    });
  }
}

/// Despawns all elements with the [`JoinGameMenuRoot`] component.
fn despawn_menu_system(mut commands: Commands, menu_root_query: Query<Entity, With<JoinGameMenuRoot>>) {
  despawn_menu(&mut commands, &menu_root_query);
}
