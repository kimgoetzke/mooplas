#![cfg(feature = "online")]

use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, CLIENT_HAND_SHAKE_TIMEOUT_SECS, DEFAULT_FONT, NORMAL_FONT};
use crate::prelude::{ConnectionInfoMessage, CustomInteraction, MenuName, UiNotification};
use crate::shared::ToggleMenuMessage;
use crate::shared::constants::BUTTON_ALPHA_DEFAULT;
use crate::ui::shared::{
  BackgroundRoot, despawn_menu, menu_base_node, spawn_background_if_not_exists, spawn_button, spawn_logo,
};
use bevy::app::{App, Plugin};
use bevy::asset::{AssetServer, Assets};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::image::TextureAtlasLayout;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::log::debug;
use bevy::prelude::{
  AlignItems, Alpha, BackgroundColor, BorderColor, BorderRadius, ButtonInput, Changed, Commands, Component,
  DetectChangesMut, Entity, FlexDirection, FontSize, IntoScheduleConfigs, Justify, JustifyContent, KeyCode,
  MessageReader, MessageWriter, Name, Node, OnExit, Query, Res, ResMut, TextColor, TextFont, TextLayout, UiRect,
  Update, With, Without, default, in_state, percent, px,
};
use bevy::text::{EditableText, TextCursorStyle};

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
          handle_submit_connection_button_system,
          handle_submit_connection_keyboard_system,
          handle_ui_notification_messages,
        )
          .chain()
          .run_if(in_state(AppState::Preparing)),
      )
      .add_systems(OnExit(AppState::Preparing), despawn_menu_system);
  }
}

/// Marker component for the root of the menu. Used for despawning.
#[derive(Component)]
struct JoinGameMenuRoot;

/// Marker component for the room input field.
#[derive(Component)]
struct JoinRoomInputField;

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
  background_root_query: Query<Entity, With<BackgroundRoot>>,
  mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::JoinGameMenu => spawn_menu(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        background_root_query,
      ),
      _ => despawn_menu(&mut commands, &menu_root_query),
    }
  }
}

fn spawn_menu(
  commands: &mut Commands,
  asset_server: &AssetServer,
  texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
  background_root_query: Query<Entity, With<BackgroundRoot>>,
) {
  let font = asset_server.load(DEFAULT_FONT);
  let background_image = asset_server.load("images/background.png");
  let logo_image = asset_server.load("images/logo_animated.png");
  let mut room_input = EditableText::default();
  room_input.max_characters = Some(200);
  room_input.visible_width = Some(45.);

  // Background & logo
  spawn_background_if_not_exists(
    commands,
    BackgroundRoot,
    background_image,
    texture_atlas_layouts,
    background_root_query,
  );
  spawn_logo(commands, JoinGameMenuRoot, logo_image, texture_atlas_layouts);

  // Host game UI
  commands
    .spawn(menu_base_node(JoinGameMenuRoot, "Join Game Menu".to_string()))
    .with_children(|parent| {
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
          parent.spawn((
            Name::new("Input Field"),
            JoinRoomInputField,
            room_input,
            TextLayout::no_wrap().with_justify(Justify::Center),
            TextFont {
              font: font.clone().into(),
              font_size: FontSize::Px(NORMAL_FONT),
              ..Default::default()
            },
            TextColor(Color::from(ACCENT_COLOUR)),
            TextCursorStyle::default(),
            TabIndex(0),
            BorderColor::all(Color::from(tailwind::SLATE_500)),
            BackgroundColor(Color::from(tailwind::SLATE_500.with_alpha(BUTTON_ALPHA_DEFAULT))),
            Node {
              width: percent(100),
              height: px(55.),
              padding: UiRect::all(px(10.)),
              align_items: AlignItems::Center,
              justify_content: JustifyContent::Center,
              border_radius: BorderRadius::all(px(10)),
              ..default()
            },
          ));

          // Spacer to make sure the UI has the same height as all other menus
          parent.spawn((
            Name::new("Spacer"),
            Node {
              width: percent(100.0),
              height: px(0.0), // The node's row gap is sufficient here
              ..default()
            },
          ));

          // Button: Connect
          spawn_button(parent, asset_server, ConnectButton, "Connect", 300, NORMAL_FONT);

          // Button: Back
          spawn_button(parent, asset_server, BackButton, "Back", 300, NORMAL_FONT);
        });
    });
}

/// A system to handle all back button interactions.
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

/// A system to handle connect button submissions.
fn handle_submit_connection_button_system(
  mut connect_button_query: Query<&mut CustomInteraction, (Changed<CustomInteraction>, With<ConnectButton>)>,
  mut back_button_query: Query<&mut CustomInteraction, (With<BackButton>, Without<ConnectButton>)>,
  input_query: Query<&EditableText, With<JoinRoomInputField>>,
  mut connection_info_message: MessageWriter<ConnectionInfoMessage>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  let Ok(input) = input_query.single() else {
    return;
  };
  let Ok(mut back_button_interaction) = back_button_query.single_mut() else {
    return;
  };

  for mut connect_button_interaction in &mut connect_button_query {
    if *connect_button_interaction != CustomInteraction::Released {
      continue;
    }

    if submit_connection_string(
      input.value().to_string(),
      &mut connect_button_interaction,
      &mut back_button_interaction,
      &mut connection_info_message,
      &mut ui_message,
    ) {
      connect_button_interaction.set_changed();
      back_button_interaction.set_changed();
    }
  }
}

/// A system to handle Enter key submissions from the focused text input field.
fn handle_submit_connection_keyboard_system(
  input_focus: Res<InputFocus>,
  keyboard_input: Res<ButtonInput<KeyCode>>,
  input_query: Query<&EditableText, With<JoinRoomInputField>>,
  mut connect_button_query: Query<&mut CustomInteraction, (With<ConnectButton>, Without<BackButton>)>,
  mut back_button_query: Query<&mut CustomInteraction, (With<BackButton>, Without<ConnectButton>)>,
  mut connection_info_message: MessageWriter<ConnectionInfoMessage>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  if !keyboard_input.just_pressed(KeyCode::Enter) {
    return;
  }
  let Some(focused_entity) = input_focus.get() else {
    return;
  };
  let Ok(input) = input_query.get(focused_entity) else {
    return;
  };
  if input.is_composing() {
    return;
  }
  let Ok(mut connect_button_interaction) = connect_button_query.single_mut() else {
    return;
  };
  let Ok(mut back_button_interaction) = back_button_query.single_mut() else {
    return;
  };

  if submit_connection_string(
    input.value().to_string(),
    &mut connect_button_interaction,
    &mut back_button_interaction,
    &mut connection_info_message,
    &mut ui_message,
  ) {
    connect_button_interaction.set_changed();
    back_button_interaction.set_changed();
  }
}

fn submit_connection_string(
  connection_string: String,
  connect_button_interaction: &mut CustomInteraction,
  back_button_interaction: &mut CustomInteraction,
  connection_info_message: &mut MessageWriter<ConnectionInfoMessage>,
  ui_message: &mut MessageWriter<UiNotification>,
) -> bool {
  let connection_string = connection_string.trim().to_string();

  // Ignore empty submissions
  if connection_string.is_empty() {
    debug!("Received empty connection string, ignoring button press...");
    return false;
  }

  // Ignore if connect button is disabled
  if *connect_button_interaction == CustomInteraction::Disabled {
    debug!("Connect button is disabled, ignoring button press...");
    return false;
  }

  // Let user know we're trying to connect
  ui_message.write(UiNotification::info(format!(
    "Attempting to connect... This can take up to {} seconds.",
    CLIENT_HAND_SHAKE_TIMEOUT_SECS
  )));

  // Disable buttons to prevent multiple submissions and race conditions on client-related resources
  *connect_button_interaction = CustomInteraction::Disabled;
  *back_button_interaction = CustomInteraction::Disabled;

  // Send connection info message for networking systems to process
  debug!("Sending [ConnectionInfoMessage] with text: {:?}", connection_string);
  connection_info_message.write(ConnectionInfoMessage { connection_string });

  true
}

/// A system to handle UI error messages and display them in the menu. Also re-enables the connect and back buttons.
fn handle_ui_notification_messages(
  mut messages: MessageReader<UiNotification>,
  mut connect_button_interaction: Query<&mut CustomInteraction, (With<ConnectButton>, Without<BackButton>)>,
  mut back_button_interaction: Query<&mut CustomInteraction, (With<BackButton>, Without<ConnectButton>)>,
) {
  for notification in messages.read() {
    if notification.should_reset_custom_interaction() {
      if let Ok(mut connect_button_interaction) = connect_button_interaction.single_mut() {
        *connect_button_interaction = CustomInteraction::None;
        connect_button_interaction.set_changed();
      }
      if let Ok(mut back_button_interaction) = back_button_interaction.single_mut() {
        *back_button_interaction = CustomInteraction::None;
        back_button_interaction.set_changed();
      }
    }
  }
}

/// Despawns all elements with the [`JoinGameMenuRoot`] component.
fn despawn_menu_system(mut commands: Commands, menu_root_query: Query<Entity, With<JoinGameMenuRoot>>) {
  despawn_menu(&mut commands, &menu_root_query);
}
