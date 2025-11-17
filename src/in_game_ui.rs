use crate::app_states::AppState;
use crate::prelude::constants::DEFAULT_FONT;
use crate::prelude::{
  AvailablePlayerConfig, AvailablePlayerConfigs, PlayerId, RegisteredPlayer, RegisteredPlayers, WinnerInfo,
};
use bevy::app::{Plugin, Update};
use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::input::ButtonInput;
use bevy::log::debug;
use bevy::prelude::{
  AlignItems, Children, Commands, Component, Entity, FlexDirection, IntoScheduleConfigs, Justify, JustifyContent,
  KeyCode, LineBreak, Node, OnEnter, OnExit, Query, Res, ResMut, Text, TextColor, TextFont, TextLayout, Val, With,
  default, in_state,
};
use bevy::text::LineHeight;

/// A plugin that manages the in-game user interface, such as the lobby and game over screens.
pub struct InGameUiPlugin;

impl Plugin for InGameUiPlugin {
  fn build(&self, app: &mut bevy::prelude::App) {
    app
      .add_systems(OnEnter(AppState::Registering), setup_lobby_ui_system)
      .add_systems(
        Update,
        registration_input_system.run_if(in_state(AppState::Registering)),
      )
      .add_systems(OnExit(AppState::Registering), despawn_lobby_ui_system)
      .add_systems(OnEnter(AppState::GameOver), spawn_game_over_ui_system)
      .add_systems(OnExit(AppState::GameOver), despawn_game_over_ui_system);
  }
}

#[derive(Component)]
struct LobbyUiRoot;

#[derive(Component)]
struct LobbyUiEntry {
  player_id: PlayerId,
}

#[derive(Component)]
struct VictoryUiRoot;

fn setup_lobby_ui_system(
  mut commands: Commands,
  available: Res<AvailablePlayerConfigs>,
  asset_server: Res<AssetServer>,
  available_configs: Res<AvailablePlayerConfigs>,
) {
  let font = asset_server.load(DEFAULT_FONT);

  let root = commands
    .spawn((
      LobbyUiRoot,
      Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
    ))
    .id();

  for available_input in &available.configs {
    let (player_label, prompt) = press_to_join_text(available_input);
    let colour = available_configs
      .configs
      .iter()
      .find(|p| p.id == available_input.id)
      .map(|p| p.colour)
      .unwrap_or(Color::WHITE);
    let entry = commands
      .spawn((
        LobbyUiEntry {
          player_id: available_input.id,
        },
        Node {
          width: Val::Percent(100.0),
          flex_direction: FlexDirection::Row,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
        },
      ))
      .with_children(|parent| {
        // Player
        parent.spawn((
          Text::new(player_label),
          TextFont {
            font: font.clone(),
            font_size: 38.0,
            ..default()
          },
          TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          TextColor(colour),
        ));

        // Prompt
        parent.spawn((
          Text::new(prompt),
          TextFont {
            font: font.clone(),
            font_size: 38.0,
            ..default()
          },
          TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          TextColor(Color::WHITE),
        ));
      })
      .id();

    commands.entity(root).add_child(entry);
  }

  // TODO: Only show this if at least one player has registered
  let prompt = commands
    .spawn((
      Text::new("Press [Space] to start..."),
      TextFont {
        font,
        font_size: 38.0,
        line_height: LineHeight::RelativeToFont(3.0),
        ..default()
      },
      TextColor(Color::WHITE),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
    ))
    .id();

  commands.entity(root).add_child(prompt);
}

// TODO: Move to controls plugin and use messages to notify registration changes
fn registration_input_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut registered_players: ResMut<RegisteredPlayers>,
  mut entries: Query<(Entity, &LobbyUiEntry, &Children)>,
  mut texts: Query<&mut Text>,
) {
  for available_config in &available_configs.configs {
    if !keyboard_input.just_pressed(available_config.input.action) {
      continue;
    }

    // Unregister if already registered
    if let Some(pos) = registered_players
      .players
      .iter()
      .position(|p| p.id == available_config.id)
    {
      registered_players.players.remove(pos);

      for (_entity, entry, children) in &mut entries {
        if entry.player_id == available_config.id {
          debug!("Player [{}] has unregistered", available_config.id.0);
          let (player_label, prompt) = press_to_join_text(available_config);
          if let Some(first_child) = children.get(0) {
            if let Ok(mut t) = texts.get_mut(*first_child) {
              t.0 = player_label.clone();
            }
          }
          if let Some(second_child) = children.get(1) {
            if let Ok(mut t) = texts.get_mut(*second_child) {
              t.0 = prompt.clone();
            }
          }
        }
      }
      continue;
    }

    // Register if not already registered
    registered_players.players.push(RegisteredPlayer {
      id: available_config.id,
      input: available_config.input.clone(),
      colour: available_config.colour,
      alive: true,
    });
    for (_entity, entry, children) in &mut entries {
      if entry.player_id == available_config.id {
        debug!("Player [{}] has registered", available_config.id.0);
        // Set first child to "Player N" and second child to ": Registered!"
        if let Some(first_child) = children.get(0) {
          if let Ok(mut t) = texts.get_mut(*first_child) {
            t.0 = format!("Player {}", available_config.id.0);
          }
        }
        if let Some(second_child) = children.get(1) {
          if let Ok(mut t) = texts.get_mut(*second_child) {
            t.0 = String::from(": Registered!");
          }
        }
      }
    }
  }
}

fn press_to_join_text(available_config: &AvailablePlayerConfig) -> (String, String) {
  (
    format!("Player {}", available_config.id.0),
    format!(": Press [{:?}] to join", available_config.input.action),
  )
}

fn despawn_lobby_ui_system(mut commands: Commands, roots: Query<Entity, With<LobbyUiRoot>>) {
  for entity in &roots {
    commands.entity(entity).despawn();
  }
}

fn spawn_game_over_ui_system(
  mut commands: Commands,
  winner: Res<WinnerInfo>,
  asset_server: Res<AssetServer>,
  registered_players: Res<RegisteredPlayers>,
) {
  let font = asset_server.load(DEFAULT_FONT);

  commands
    .spawn((
      VictoryUiRoot,
      Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
    ))
    .with_children(|parent| {
      match winner.winner {
        Some(id) => {
          let colour = registered_players
            .players
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.colour)
            .unwrap_or(Color::WHITE);
          parent
            .spawn((Node {
              flex_direction: FlexDirection::Row,
              justify_content: JustifyContent::Center,
              align_items: AlignItems::Center,
              ..default()
            },))
            .with_children(|row| {
              row.spawn((
                Text::new(format!("Player {}", id.0)),
                TextFont {
                  font: font.clone(),
                  font_size: 60.0,
                  ..default()
                },
                TextColor(colour),
              ));
              row.spawn((
                Text::new(" wins!"),
                TextFont {
                  font: font.clone(),
                  font_size: 60.0,
                  ..default()
                },
                TextColor(Color::WHITE),
              ));
            });
        }
        None => {
          parent.spawn((
            Text::new("No winner this round."),
            TextFont {
              font: font.clone(),
              font_size: 60.0,
              ..default()
            },
            TextColor(Color::WHITE),
          ));
        }
      }

      parent.spawn((
        Text::new("Press [Space] to continue..."),
        TextFont {
          font,
          font_size: 38.0,
          line_height: LineHeight::RelativeToFont(3.0),
          ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      ));
    });
}

fn despawn_game_over_ui_system(mut commands: Commands, roots: Query<Entity, With<VictoryUiRoot>>) {
  for entity in &roots {
    commands.entity(entity).despawn();
  }
}
