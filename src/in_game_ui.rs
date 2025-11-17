use crate::app_states::AppState;
use crate::prelude::constants::DEFAULT_FONT;
use crate::prelude::{
  AvailablePlayerConfig, AvailablePlayerConfigs, PlayerId, RegisteredPlayer, RegisteredPlayers, WinnerInfo,
};
use bevy::app::{Plugin, Update};
use bevy::asset::{AssetServer, Handle};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::input::ButtonInput;
use bevy::log::debug;
use bevy::prelude::{
  AlignItems, ChildOf, Children, Commands, Component, Entity, FlexDirection, Font, IntoScheduleConfigs, Justify,
  JustifyContent, KeyCode, LineBreak, Node, OnEnter, OnExit, Query, Res, ResMut, Text, TextColor, TextFont, TextLayout,
  Val, With, default, in_state,
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
struct LobbyUiPrompt;

#[derive(Component)]
struct VictoryUiRoot;

fn setup_lobby_ui_system(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  available_configs: Res<AvailablePlayerConfigs>,
) {
  let font = asset_server.load(DEFAULT_FONT);
  let default_font = default_font(&font);

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
        ));

        // Player prompt
        player_join_prompt(&font, available_config, parent);
      })
      .id();
    commands.entity(root).add_child(entry);
  }

  // Call to action
  let prompt = commands
    .spawn((
      LobbyUiPrompt,
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
      ));
      // Part 2 - Always yellow and initially empty
      parent.spawn((
        Text::new(""),
        default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
        TextColor(Color::from(tailwind::YELLOW_400)),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      ));
      // Part 3 - Always white and initially empty
      parent.spawn((
        Text::new(""),
        default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
        TextColor(Color::WHITE),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      ));
    })
    .id();
  commands.entity(root).add_child(prompt);
}

// TODO: Move to controls plugin and use messages to notify registration changes
fn registration_input_system(
  mut commands: Commands,
  keyboard_input: Res<ButtonInput<KeyCode>>,
  available_configs: Res<AvailablePlayerConfigs>,
  asset_server: Res<AssetServer>,
  mut registered_players: ResMut<RegisteredPlayers>,
  mut entries_query: Query<(Entity, &LobbyUiEntry, &Children)>,
  prompt_query: Query<(Entity, &Children), With<LobbyUiPrompt>>,
  mut texts_query: Query<&mut Text>,
) {
  let font = asset_server.load(DEFAULT_FONT);
  for available_config in &available_configs.configs {
    if !keyboard_input.just_pressed(available_config.input.action) {
      continue;
    }

    // Unregister if already registered
    if let Some(position) = registered_players
      .players
      .iter()
      .position(|p| p.id == available_config.id)
    {
      registered_players.players.remove(position);
      for (entity, entry, children) in &mut entries_query {
        if entry.player_id == available_config.id {
          debug!("Player [{}] has unregistered", available_config.id.0);
          if let Some(prompt_node) = children.get(1) {
            commands.entity(*prompt_node).despawn();
            commands.entity(entity).with_children(|parent| {
              player_join_prompt(&font, available_config, parent);
            });
          }
        }
      }

      // Update call to action under player list
      update_call_to_action_to_start(&*registered_players, &prompt_query, &mut texts_query);
      continue;
    }

    // Register if not already registered
    registered_players.players.push(RegisteredPlayer {
      id: available_config.id,
      input: available_config.input.clone(),
      colour: available_config.colour,
      alive: true,
    });
    for (entity, entry, children) in &mut entries_query {
      if entry.player_id == available_config.id {
        debug!("Player [{}] has registered", available_config.id.0);
        if let Some(prompt_node) = children.get(1) {
          commands.entity(*prompt_node).despawn();
          commands.entity(entity).with_children(|parent| {
            player_registered_prompt(&font, available_config, parent);
          });
        }
      }
    }

    // Update call to action under player list
    update_call_to_action_to_start(&*registered_players, &prompt_query, &mut texts_query);
  }
}

fn player_join_prompt(
  font: &Handle<Font>,
  available_config: &AvailablePlayerConfig,
  parent: &mut RelatedSpawnerCommands<ChildOf>,
) {
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
      ));
      // ...[Key]...
      parent.spawn((
        Text::new(format!("[{:?}]", available_config.input.action)),
        text_font.clone(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::from(tailwind::YELLOW_400)),
      ));
      // ...to join
      parent.spawn((
        Text::new(" to join"),
        text_font,
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::WHITE),
      ));
    });
}

fn player_registered_prompt(
  font: &Handle<Font>,
  _available_config: &AvailablePlayerConfig,
  parent: &mut RelatedSpawnerCommands<ChildOf>,
) {
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
      ));
    });
}

fn update_call_to_action_to_start(
  registered_players: &RegisteredPlayers,
  prompt_query: &Query<(Entity, &Children), With<LobbyUiPrompt>>,
  texts_query: &mut Query<&mut Text>,
) {
  for (_entity, children) in prompt_query.iter() {
    let has_players = !registered_players.players.is_empty();
    // Part 1
    if let Some(prefix_ent) = children.get(0) {
      if let Ok(mut t) = texts_query.get_mut(*prefix_ent) {
        t.0 = if has_players {
          "Press ".to_string()
        } else {
          "More players needed to start...".to_string()
        };
      }
    }
    // Part 2
    if let Some(key_ent) = children.get(1) {
      if let Ok(mut t) = texts_query.get_mut(*key_ent) {
        t.0 = if has_players {
          "[Space]".to_string()
        } else {
          String::new()
        };
      }
    }
    // Part 3
    if let Some(suffix_ent) = children.get(2) {
      if let Ok(mut t) = texts_query.get_mut(*suffix_ent) {
        t.0 = if has_players {
          " to start...".to_string()
        } else {
          String::new()
        };
      }
    }
  }
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
      // Match result
      let large_text = large_text(&font);
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
                large_text.clone(),
                TextColor(colour),
              ));
              row.spawn((Text::new(" wins!"), large_text.clone(), TextColor(Color::WHITE)));
            });
        }
        None => {
          parent.spawn((
            Text::new("No winner this round."),
            large_text.clone(),
            TextColor(Color::WHITE),
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
          ));
          parent.spawn((
            Text::new("[Space]"),
            default_font(&font).with_line_height(LineHeight::RelativeToFont(3.0)),
            TextColor(Color::from(tailwind::YELLOW_400)),
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          ));
          parent.spawn((
            Text::new(" to continue..."),
            default_font(&font).with_line_height(LineHeight::RelativeToFont(3.0)),
            TextColor(Color::WHITE),
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
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

fn despawn_game_over_ui_system(mut commands: Commands, roots: Query<Entity, With<VictoryUiRoot>>) {
  for entity in &roots {
    commands.entity(entity).despawn();
  }
}
