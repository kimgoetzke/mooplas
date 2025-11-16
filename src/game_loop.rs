use crate::app_states::AppState;
use crate::player::SnakeTail;
use crate::prelude::{PlayerId, PlayerInput, RegisteredPlayer, RegisteredPlayers, SnakeHead};
use crate::shared::constants::DEFAULT_FONT;
use bevy::app::{App, Plugin};
use bevy::input::ButtonInput;
use bevy::prelude::*;

pub struct GameLoopPlugin;

#[derive(Clone)]
struct AvailablePlayerInput {
  id: PlayerId,
  input: PlayerInput,
}

#[derive(Resource, Default)]
struct AvailablePlayerInputs {
  inputs: Vec<AvailablePlayerInput>,
}

#[derive(Resource, Default)]
struct WinnerInfo {
  winner: Option<PlayerId>,
}

#[derive(Component)]
struct LobbyUiEntry {
  player_id: PlayerId,
}

#[derive(Component)]
struct LobbyUiRoot;

#[derive(Component)]
struct VictoryUiRoot;

impl Plugin for GameLoopPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<AvailablePlayerInputs>()
      .init_resource::<WinnerInfo>()
      .add_systems(Startup, setup_available_player_inputs_system)
      // Registering
      .add_systems(
        OnEnter(AppState::Registering),
        (reset_for_lobby_system, setup_lobby_ui_system),
      )
      .add_systems(
        Update,
        registration_input_system.run_if(in_state(AppState::Registering)),
      )
      .add_systems(OnExit(AppState::Registering), cleanup_lobby_ui_system)
      // Playing
      .add_systems(
        Update,
        (check_snake_collisions_system, transition_to_game_over_system).run_if(in_state(AppState::Playing)),
      )
      .add_systems(OnExit(AppState::Playing), despawn_players_system)
      // GameOver
      .add_systems(OnEnter(AppState::GameOver), spawn_game_over_ui_system)
      .add_systems(OnExit(AppState::GameOver), despawn_game_over_ui_system);
  }
}

fn setup_available_player_inputs_system(mut available: ResMut<AvailablePlayerInputs>) {
  if !available.inputs.is_empty() {
    return;
  }
  available.inputs = vec![
    AvailablePlayerInput {
      id: PlayerId(0),
      input: PlayerInput::new(PlayerId(0), KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyW),
    },
    AvailablePlayerInput {
      id: PlayerId(1),
      input: PlayerInput::new(PlayerId(1), KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp),
    },
    AvailablePlayerInput {
      id: PlayerId(2),
      input: PlayerInput::new(PlayerId(2), KeyCode::KeyB, KeyCode::KeyH, KeyCode::KeyM),
    },
  ];
}

fn reset_for_lobby_system(mut registered: ResMut<RegisteredPlayers>, mut winner: ResMut<WinnerInfo>) {
  registered.players.clear();
  winner.winner = None;
}

fn setup_lobby_ui_system(
  mut commands: Commands,
  available: Res<AvailablePlayerInputs>,
  asset_server: Res<AssetServer>,
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

  for available_input in &available.inputs {
    let text = press_to_join_text(available_input);
    let entry = commands
      .spawn((
        LobbyUiEntry {
          player_id: available_input.id,
        },
        Text::new(text),
        TextFont {
          font: font.clone(),
          font_size: 26.0,
          ..default()
        },
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::WHITE),
      ))
      .id();
    commands.entity(root).add_child(entry);
  }
}

// TODO: Move UI to separate plugin and controls to existing controls plugin
fn registration_input_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  available: Res<AvailablePlayerInputs>,
  mut registered: ResMut<RegisteredPlayers>,
  mut query: Query<(&LobbyUiEntry, &mut Text)>,
) {
  for available_input in &available.inputs {
    if !keyboard_input.just_pressed(available_input.input.action) {
      continue;
    }

    // Unregister if already registered
    if let Some(pos) = registered.players.iter().position(|p| p.id == available_input.id) {
      registered.players.remove(pos);
      for (entry, mut text) in &mut query {
        if entry.player_id == available_input.id {
          debug!("Player [{}] has unregistered", available_input.id.0);
          text.0 = press_to_join_text(available_input);
        }
      }
      continue;
    }

    // Register if not already registered
    registered.players.push(RegisteredPlayer {
      id: available_input.id,
      input: available_input.input.clone(),
      alive: true,
    });
    for (entry, mut text) in &mut query {
      if entry.player_id == available_input.id {
        debug!("Player [{}] has registered", available_input.id.0);
        text.0 = format!("Player {}: Registered!", available_input.id.0);
      }
    }
  }
}

fn press_to_join_text(available_input: &AvailablePlayerInput) -> String {
  format!(
    "Player {}: Press [{:?}] to join",
    available_input.id.0, available_input.input.action
  )
}

fn cleanup_lobby_ui_system(mut commands: Commands, roots: Query<Entity, With<LobbyUiRoot>>) {
  for entity in &roots {
    commands.entity(entity).despawn();
  }
}

fn check_snake_collisions_system(
  mut registered: ResMut<RegisteredPlayers>,
  heads: Query<(&PlayerId, &Transform), With<SnakeHead>>,
  tails: Query<(&PlayerId, &Transform), With<SnakeTail>>,
) {
  // TODO: Replace with actual collision detection
  let _ = (&mut *registered, &heads, &tails);
}

fn transition_to_game_over_system(
  registered_players: ResMut<RegisteredPlayers>,
  mut winner: ResMut<WinnerInfo>,
  mut next: ResMut<NextState<AppState>>,
) {
  let alive_players: Vec<&RegisteredPlayer> = registered_players.players.iter().filter(|p| p.alive).collect();
  match (registered_players.players.len(), alive_players.len()) {
    (_, 0) => {
      winner.winner = None;
      next.set(AppState::GameOver);
    }
    (registered_players, 1) if registered_players > 1 => {
      winner.winner = Some(alive_players[0].id);
      next.set(AppState::GameOver);
    }
    _ => {}
  }
}

fn despawn_players_system(
  mut commands: Commands,
  heads: Query<Entity, With<SnakeHead>>,
  tails: Query<Entity, With<SnakeTail>>,
) {
  for entity in &heads {
    commands.entity(entity).despawn();
  }
  for entity in &tails {
    commands.entity(entity).despawn();
  }
}

fn spawn_game_over_ui_system(mut commands: Commands, winner: Res<WinnerInfo>, asset_server: Res<AssetServer>) {
  let font = asset_server.load(DEFAULT_FONT);
  let message = match winner.winner {
    Some(id) => format!("Player {} wins!\nPress [Space] to continue", id.0),
    None => "No winner this round.\nPress [Space] to continue".to_string(),
  };

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
      parent.spawn((
        Text::new(message),
        TextFont {
          font,
          font_size: 42.0,
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
