use crate::app_state::AppState;
use crate::prelude::constants::{DEFAULT_FONT, NORMAL_FONT, PIXEL_PERFECT_LAYER, RESOLUTION_HEIGHT, RESOLUTION_WIDTH};
use crate::prelude::{CustomInteraction, MenuName};
use crate::shared::ToggleMenuMessage;
use crate::ui::spawn_button;
use bevy::app::{App, Plugin};
use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::log::debug;
use bevy::math::Vec2;
use bevy::prelude::{
  AlignItems, Changed, Commands, Component, Entity, FlexDirection, IntoScheduleConfigs, JustifyContent, MessageReader,
  MessageWriter, Name, Node, PositionType, Query, Res, Sprite, Text, TextColor, TextFont, TextShadow, Transform,
  Update, With, default, in_state, percent, px,
};

// TODO: Refactor this and MainMenuPlugin to reduce code duplication once clearer patterns emerge
/// A plugin to manage the online multiplayer menu UIs.
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
      MenuName::PlayOnlineMenu => spawn_play_online_menu(&mut commands, &asset_server),
      _ => despawn_play_online_menu(&mut commands, &menu_root_query),
    }
  }
}

fn spawn_play_online_menu(commands: &mut Commands, asset_server: &AssetServer) {
  let font = asset_server.load(DEFAULT_FONT);
  let heading_font = font.clone();
  let background_image = asset_server.load("images/background_menu_main.png");

  // Background
  commands.spawn((
    Name::new("Play Online Menu Background"),
    PlayOnlineMenuRoot,
    Sprite {
      image: background_image.clone(),
      custom_size: Some(Vec2::new(RESOLUTION_WIDTH as f32, RESOLUTION_HEIGHT as f32)),
      ..default()
    },
    Transform::from_xyz(0., 0., -1.),
    PIXEL_PERFECT_LAYER,
  ));

  // Play online UI
  commands
    .spawn((
      Name::new("Play Online Menu"),
      PlayOnlineMenuRoot,
      Node {
        width: percent(100),
        height: percent(100),
        position_type: PositionType::Relative,
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
    ))
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
              font_size: 120.,
              ..default()
            },
            TextColor(Color::from(tailwind::AMBER_300)),
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

fn despawn_play_online_menu(commands: &mut Commands, menu_root_query: &Query<Entity, With<PlayOnlineMenuRoot>>) {
  for root in menu_root_query {
    commands.entity(root).despawn();
  }
}
