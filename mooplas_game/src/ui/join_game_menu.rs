#![cfg(feature = "online")]

use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, DEFAULT_FONT, NORMAL_FONT};
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
use bevy::ecs::children;
use bevy::image::TextureAtlasLayout;
use bevy::log::debug;
use bevy::prelude::{
  AlignItems, Alpha, BackgroundColor, BorderColor, BorderRadius, Changed, Click, Commands, Component, DetectChangesMut,
  Entity, FlexDirection, IntoScheduleConfigs, Justify, JustifyContent, MessageReader, MessageWriter, Name, Node, On,
  OnExit, Pointer, PositionType, Query, Res, ResMut, Single, Text, TextColor, TextFont, UiRect, Update, With, Without,
  default, in_state, percent, px,
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
          handle_ui_notification_messages,
        )
          .run_if(in_state(AppState::Preparing)),
      )
      .add_systems(OnExit(AppState::Preparing), despawn_menu_system);
  }
}

/// Marker component for the root of the menu. Used for despawning.
#[derive(Component)]
struct JoinGameMenuRoot;

/// Marker component for displaying a notifications.
#[derive(Component)]
struct NotificationText;

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
          // TODO: Replace bevy_ui_text_input, so I can upgrade to Bevy 0.18 and copy & paste works more reliably
          // Input field
          let input_field_entity = parent
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
                font: font.clone(),
                font_size: NORMAL_FONT,
                ..Default::default()
              },
              TextColor(Color::from(ACCENT_COLOUR)),
              BorderColor::all(Color::from(tailwind::SLATE_500)),
              BackgroundColor(Color::from(tailwind::SLATE_500.with_alpha(BUTTON_ALPHA_DEFAULT))),
              TextInputPrompt::new("Paste connection string here..."),
              Node {
                width: percent(100),
                height: px(45.),
                padding: UiRect::all(px(10.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(10)),
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
              query.get_mut(input_field_entity).unwrap().add(TextInputAction::Submit);
            },
          );

          // Button: Back
          spawn_button(parent, &asset_server, BackButton, "Back", 300, NORMAL_FONT);
        });
    });

  commands.spawn((
    Name::new("Notification UI"),
    JoinGameMenuRoot,
    Node {
      width: percent(100),
      height: percent(25),
      bottom: px(0.0),
      position_type: PositionType::Absolute,
      flex_direction: FlexDirection::Column,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },
    children![(
      TextFont {
        font,
        font_size: NORMAL_FONT,
        ..default()
      },
      TextColor(Color::from(tailwind::RED_500)),
      Text::default(),
      NotificationText,
    )],
  ));
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
  mut connect_button_interaction: Single<&mut CustomInteraction, (With<ConnectButton>, Without<BackButton>)>,
  mut back_button_interaction: Single<&mut CustomInteraction, (With<BackButton>, Without<ConnectButton>)>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  for message in messages.read() {
    // Let user know we're trying to connect
    ui_message.write(UiNotification::info("Attempting to connect...".to_string()));

    // Ignore empty submissions
    if message.text.is_empty() {
      debug!("Received empty connection string submission, ignoring button press...");
      continue;
    }

    // Ignore if connect button is disabled
    if **connect_button_interaction == CustomInteraction::Disabled {
      debug!("Connect button is disabled, ignoring button press...");
      return;
    }

    // Disable buttons to prevent multiple submissions and race conditions on client-related resources
    **connect_button_interaction = CustomInteraction::Disabled;
    connect_button_interaction.set_changed();
    **back_button_interaction = CustomInteraction::Disabled;
    back_button_interaction.set_changed();

    // Send connection info message for networking systems to process
    debug!("Sending [ConnectionInfoMessage] with text: {:?}", message.text);
    connection_info_message.write(ConnectionInfoMessage {
      connection_string: message.text.to_string(),
    });
  }
}

/// A system to handle UI error messages and display them in the menu. Also re-enables the connect and back buttons.
fn handle_ui_notification_messages(
  mut messages: MessageReader<UiNotification>,
  mut notification_text_query: Query<(&mut Text, &mut TextColor), With<NotificationText>>,
  mut connect_button_interaction: Single<&mut CustomInteraction, (With<ConnectButton>, Without<BackButton>)>,
  mut back_button_interaction: Single<&mut CustomInteraction, (With<BackButton>, Without<ConnectButton>)>,
) {
  for notification in messages.read() {
    for (mut text, mut text_colour) in &mut notification_text_query {
      text.0 = notification.text.clone();
      text_colour.0 = *TextColor(notification.colour());
    }
    if notification.should_reset_custom_interaction() {
      **connect_button_interaction = CustomInteraction::None;
      connect_button_interaction.set_changed();
      **back_button_interaction = CustomInteraction::None;
      back_button_interaction.set_changed();
    }
  }
}

/// Despawns all elements with the [`JoinGameMenuRoot`] component.
fn despawn_menu_system(mut commands: Commands, menu_root_query: Query<Entity, With<JoinGameMenuRoot>>) {
  despawn_menu(&mut commands, &menu_root_query);
}
