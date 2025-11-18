use crate::app_states::AppState;
use crate::prelude::constants::DEFAULT_FONT;
use crate::prelude::{AvailablePlayerConfig, AvailablePlayerConfigs, PlayerId, RegisteredPlayers, WinnerInfo};
use crate::shared::PlayerRegistrationMessage;
use bevy::app::{Plugin, Update};
use bevy::asset::{AssetServer, Handle};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::{
  AlignItems, ChildOf, Children, Commands, Component, Entity, FlexDirection, Font, IntoScheduleConfigs, Justify,
  JustifyContent, LineBreak, MessageReader, Node, OnEnter, OnExit, Query, Res, Text, TextColor, TextFont, TextLayout,
  TextShadow, Val, With, default, in_state,
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
        handle_player_registration_event.run_if(in_state(AppState::Registering)),
      )
      .add_systems(OnExit(AppState::Registering), despawn_lobby_ui_system)
      .add_systems(OnEnter(AppState::GameOver), spawn_game_over_ui_system)
      .add_systems(OnExit(AppState::GameOver), despawn_game_over_ui_system);
  }
}

/// Marker component for the root of the lobby UI. Used for despawning. All other Lobby UI components must be children
/// of this.
#[derive(Component)]
struct LobbyUiRoot;

/// Marker component for each available player's information and status in the lobby UI.
#[derive(Component)]
struct LobbyUiEntry {
  player_id: PlayerId,
}

/// Marker component for the lobby UI call-to-action (CTA) at the bottom of the player list.
#[derive(Component)]
struct LobbyUiCta;

/// Marker component for the root of the victory/game over UI. Used for despawning. All other Victory UI components
/// must be children of this.
#[derive(Component)]
struct VictoryUiRoot;

/// Sets up the lobby UI, displaying available players and prompts to join.
fn setup_lobby_ui_system(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  available_configs: Res<AvailablePlayerConfigs>,
) {
  let font = asset_server.load(DEFAULT_FONT);
  let default_font = default_font(&font);
  let default_shadow = default_shadow();

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

  for available_config in &available_configs.configs {
    let colour = available_configs
      .configs
      .iter()
      .find(|p| p.id == available_config.id)
      .map(|p| p.colour)
      .unwrap_or(Color::WHITE);
    let entry = commands
      .spawn((
        LobbyUiEntry {
          player_id: available_config.id,
        },
        Node {
          flex_direction: FlexDirection::Row,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
        },
      ))
      .with_children(|parent| {
        // Player
        parent.spawn((
          Text::new(format!("Player {}", available_config.id.0)),
          default_font.clone(),
          TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          TextColor(colour),
          default_shadow,
        ));

        // Player prompt
        player_join_prompt(&font, available_config, parent);
      })
      .id();
    commands.entity(root).add_child(entry);
  }

  // Call to action
  let cta = commands
    .spawn((
      LobbyUiCta,
      Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
    ))
    .with_children(|parent| {
      // Part 1 - Always white
      parent.spawn((
        Text::new("More players needed to start..."),
        default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
        TextColor(Color::WHITE),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        default_shadow,
      ));
      // Part 2 - Always yellow and initially empty
      parent.spawn((
        Text::new(""),
        default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
        TextColor(Color::from(tailwind::YELLOW_400)),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        default_shadow,
      ));
      // Part 3 - Always white and initially empty
      parent.spawn((
        Text::new(""),
        default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
        TextColor(Color::WHITE),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        default_shadow,
      ));
    })
    .id();
  commands.entity(root).add_child(cta);
}

fn handle_player_registration_event(
  mut player_registration_message: MessageReader<PlayerRegistrationMessage>,
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut entries_query: Query<(Entity, &LobbyUiEntry, &Children)>,
  cta_query: Query<&Children, With<LobbyUiCta>>,
  mut texts_query: Query<&mut Text>,
) {
  for message in player_registration_message.read() {
    let font = asset_server.load(DEFAULT_FONT);
    let config = available_configs.configs.iter().find(|p| p.id == message.player_id);

    // Update entry for player
    match message.has_registered {
      false => {
        for (entity, entry, children) in &mut entries_query {
          if entry.player_id == message.player_id {
            if let Some(prompt_node) = children.get(1) {
              commands.entity(*prompt_node).despawn();
              if let Some(ref available_config) = config {
                commands.entity(entity).with_children(|parent| {
                  player_join_prompt(&font, available_config, parent);
                });
              }
            }
          }
        }
      }
      true => {
        for (entity, entry, children) in &mut entries_query {
          if entry.player_id == message.player_id {
            if let Some(prompt_node) = children.get(1) {
              commands.entity(*prompt_node).despawn();
              commands.entity(entity).with_children(|parent| {
                player_registered_prompt(&font, parent);
              });
            }
          }
        }
      }
    }

    // Update call to action under player list
    update_call_to_action_to_start(message.is_anyone_registered, &cta_query, &mut texts_query);
  }
}

fn player_join_prompt(
  font: &Handle<Font>,
  available_config: &AvailablePlayerConfig,
  parent: &mut RelatedSpawnerCommands<ChildOf>,
) {
  let default_shadow = default_shadow();
  parent
    .spawn((Node {
      flex_direction: FlexDirection::Row,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },))
    .with_children(|parent| {
      let text_font = default_font(font);
      parent.spawn((
        // Press...
        Text::new(": Press "),
        text_font.clone(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::WHITE),
        default_shadow,
      ));
      // ...[Key]...
      parent.spawn((
        Text::new(format!("[{:?}]", available_config.input.action)),
        text_font.clone(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::from(tailwind::YELLOW_400)),
        default_shadow,
      ));
      // ...to join
      parent.spawn((
        Text::new(" to join"),
        text_font,
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::WHITE),
        default_shadow,
      ));
    });
}

fn player_registered_prompt(font: &Handle<Font>, parent: &mut RelatedSpawnerCommands<ChildOf>) {
  parent
    .spawn((Node {
      flex_direction: FlexDirection::Row,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },))
    .with_children(|parent| {
      parent.spawn((
        // Press...
        Text::new(": Registered"),
        default_font(font),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::WHITE),
        default_shadow(),
      ));
    });
}

fn update_call_to_action_to_start(
  has_players: bool,
  cta_query: &Query<&Children, With<LobbyUiCta>>,
  texts_query: &mut Query<&mut Text>,
) {
  for children in cta_query.iter() {
    // Part 1 - Always white
    if let Some(prefix_entity) = children.get(0) {
      if let Ok(mut text) = texts_query.get_mut(*prefix_entity) {
        text.0 = if has_players {
          "Press ".to_string()
        } else {
          "More players needed to start...".to_string()
        };
      }
    }
    // Part 2 - Always yellow
    if let Some(key_entity) = children.get(1) {
      if let Ok(mut text) = texts_query.get_mut(*key_entity) {
        text.0 = if has_players {
          "[Space]".to_string()
        } else {
          String::new()
        };
      }
    }
    // Part 3 - Always white
    if let Some(suffix_entity) = children.get(2) {
      if let Ok(mut text) = texts_query.get_mut(*suffix_entity) {
        text.0 = if has_players {
          " to start...".to_string()
        } else {
          String::new()
        };
      }
    }
  }
}

/// Despawns the entire lobby UI. Call when exiting the registration state.
fn despawn_lobby_ui_system(mut commands: Commands, roots: Query<Entity, With<LobbyUiRoot>>) {
  for entity in &roots {
    commands.entity(entity).despawn();
  }
}

/// Spawns the game over UI, displaying the winner and a prompt to continue.
fn spawn_game_over_ui_system(
  mut commands: Commands,
  winner: Res<WinnerInfo>,
  asset_server: Res<AssetServer>,
  registered_players: Res<RegisteredPlayers>,
) {
  let font = asset_server.load(DEFAULT_FONT);
  let default_shadow = default_shadow();

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
      // Match result
      let large_text = large_text(&font);
      match winner.get() {
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
                large_text.clone(),
                TextColor(colour),
                default_shadow,
              ));
              row.spawn((
                Text::new(" wins!"),
                large_text.clone(),
                TextColor(Color::WHITE),
                default_shadow,
              ));
            });
        }
        None => {
          parent.spawn((
            Text::new("No winner this round."),
            large_text.clone(),
            TextColor(Color::WHITE),
            default_shadow,
          ));
        }
      }

      // Call to action
      parent
        .spawn((Node {
          flex_direction: FlexDirection::Row,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
        },))
        .with_children(|parent| {
          parent.spawn((
            Text::new("Press "),
            default_font(&font).with_line_height(LineHeight::RelativeToFont(3.0)),
            TextColor(Color::WHITE),
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            default_shadow,
          ));
          parent.spawn((
            Text::new("[Space]"),
            default_font(&font).with_line_height(LineHeight::RelativeToFont(3.0)),
            TextColor(Color::from(tailwind::YELLOW_400)),
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            default_shadow,
          ));
          parent.spawn((
            Text::new(" to continue..."),
            default_font(&font).with_line_height(LineHeight::RelativeToFont(3.0)),
            TextColor(Color::WHITE),
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            default_shadow,
          ));
        });
    });
}

fn default_font(font: &Handle<Font>) -> TextFont {
  TextFont {
    font: font.clone(),
    font_size: 38.0,
    ..default()
  }
}

fn large_text(font: &Handle<Font>) -> TextFont {
  TextFont {
    font: font.clone(),
    font_size: 60.0,
    ..default()
  }
}

fn default_shadow() -> TextShadow {
  TextShadow::default()
}

/// Despawns the entire game over UI. Call when exiting the game over state.
fn despawn_game_over_ui_system(mut commands: Commands, victory_ui_root_query: Query<Entity, With<VictoryUiRoot>>) {
  for entity in &victory_ui_root_query {
    commands.entity(entity).despawn();
  }
}
