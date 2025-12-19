#![cfg(feature = "online")]

use crate::app_state::AppState;
use crate::prelude::constants::{
  ACCENT_COLOUR, BUTTON_ALPHA_DEFAULT, DEFAULT_FONT, HEADER_FONT, NORMAL_FONT, TEXT_COLOUR,
};
use crate::prelude::{ConnectionInfoMessage, CustomInteraction, MenuName};
use crate::shared::ToggleMenuMessage;
use crate::shared::constants::SMALL_FONT;
use crate::ui::shared::{despawn_menu, menu_base_node, spawn_background, spawn_button};
use bevy::app::{App, Plugin};
use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::log::*;
use bevy::prelude::{
  AlignItems, Alpha, BackgroundColor, BorderColor, BorderRadius, Changed, Commands, Component, Entity, FlexDirection,
  IntoScheduleConfigs, Justify, JustifyContent, Local, MessageReader, MessageWriter, Name, Node, OnExit, Query, Res,
  Text, TextColor, TextFont, TextShadow, UiRect, Update, With, default, in_state, percent, px,
};
use bevy::text::LineHeight;
use bevy_ui_text_input::actions::{TextInputAction, TextInputEdit};
use bevy_ui_text_input::{TextInputMode, TextInputNode, TextInputPrompt, TextInputQueue};

/// A plugin to manage the game menu UI used to host an online multiplayer game. Only included with the "online"
/// feature.
pub struct HostGameMenuPlugin;

impl Plugin for HostGameMenuPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        (
          handle_toggle_menu_message,
          handle_button_interactions_system,
          handle_connection_info_updated_message,
        )
          .chain()
          .run_if(in_state(AppState::Preparing)),
      )
      .add_systems(OnExit(AppState::Preparing), despawn_menu_system);
  }
}

/// Marker component for the root of the menu. Used for despawning.
#[derive(Component)]
struct HostGameMenuRoot;

/// Marker component for the back button.
#[derive(Component)]
struct BackButton;

/// System to handle toggling the play online menu based on received messages.
fn handle_toggle_menu_message(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut messages: MessageReader<ToggleMenuMessage>,
  menu_root_query: Query<Entity, With<HostGameMenuRoot>>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::HostGameMenu => spawn_menu(&mut commands, &asset_server),
      _ => despawn_menu(&mut commands, &menu_root_query),
    }
  }
}

fn spawn_menu(commands: &mut Commands, asset_server: &AssetServer) {
  let font = asset_server.load(DEFAULT_FONT);
  let heading_font = font.clone();
  let background_image = asset_server.load("images/background_menu_main.png");

  // Background
  spawn_background(commands, HostGameMenuRoot, background_image.clone());

  // Host game UI
  commands
    .spawn(menu_base_node(HostGameMenuRoot, "Host Game Menu".to_string()))
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

      // The actual menu
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
          // Instructions
          parent.spawn((
            Text::new("Your friends can now join by connecting to:"),
            TextFont {
              font: heading_font.clone(),
              font_size: SMALL_FONT,
              ..default()
            },
            TEXT_COLOUR,
            TextShadow::default(),
          ));

          // Text input field to copy the host address from
          parent.spawn((
            Name::new("Input Field"),
            TextInputNode {
              mode: TextInputMode::SingleLine,
              max_chars: Some(50),
              clear_on_submit: true,
              justification: Justify::Center,
              ..Default::default()
            },
            TextInputPrompt::new("Determining your address..."),
            TextFont {
              font,
              font_size: NORMAL_FONT,
              ..Default::default()
            },
            TextColor(Color::from(ACCENT_COLOUR)),
            BorderRadius::all(px(10)),
            BorderColor::all(Color::from(tailwind::SLATE_500)),
            BackgroundColor(Color::from(tailwind::SLATE_500.with_alpha(BUTTON_ALPHA_DEFAULT))),
            Node {
              width: percent(100),
              height: px(45.),
              padding: UiRect::all(px(10.)),
              align_items: AlignItems::Center,
              justify_content: JustifyContent::Center,
              ..default()
            },
          ));

          // Information text
          parent.spawn((
            Text::new("Waiting for at least one player to join..."),
            TextFont {
              font: heading_font.clone(),
              font_size: SMALL_FONT,
              ..default()
            }
            .with_line_height(LineHeight::RelativeToFont(2.)),
            TEXT_COLOUR,
            TextShadow::default(),
          ));

          // Back button
          spawn_button(parent, &asset_server, BackButton, "Back", 300, NORMAL_FONT);
        });
    });
}

/// System to handle updating the host address text when the connection info is updated.
fn handle_connection_info_updated_message(
  mut messages: MessageReader<ConnectionInfoMessage>,
  mut text_input_queue: Query<&mut TextInputQueue>,
  mut retryable_connection_info_message: Local<Option<(ConnectionInfoMessage, bool)>>,
) {
  for message in messages.read() {
    trace!("Received connection info message: [{}]", message.connection_string,);
    *retryable_connection_info_message = Some((message.clone(), false));
  }

  if let Some((message, is_processed)) = &mut *retryable_connection_info_message {
    if !*is_processed {
      for mut queue in &mut text_input_queue {
        queue.add_front(TextInputAction::Edit(TextInputEdit::Paste(
          message.connection_string.clone(),
        )));
        trace!("Successfully updated text input field with connection info");
        *is_processed = true;
      }
    }
    if *is_processed {
      *retryable_connection_info_message = None;
    }
  }
}

/// System to handle all host game menu button interactions.
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

/// Despawns all elements with the [`HostGameMenuRoot`] component.
fn despawn_menu_system(mut commands: Commands, menu_root_query: Query<Entity, With<HostGameMenuRoot>>) {
  despawn_menu(&mut commands, &menu_root_query);
}
