use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, DEFAULT_FONT, TEXT_COLOUR};
use crate::prelude::{
  AvailableControlSchemes, ControlScheme, ControlSchemeId, MAX_PLAYERS, PlayerId, RegisteredPlayers, Settings,
  colour_for_player_id,
};
use crate::shared::PlayerRegistrationMessage;
use crate::ui::in_game_ui::in_game_ui;
use bevy::app::{App, Plugin, Update};
use bevy::asset::AssetServer;
use bevy::ecs::children;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::ecs::spawn::{Spawn, SpawnRelatedBundle};
use bevy::prelude::{
  AlignItems, Alpha, ChildOf, Children, Color, Commands, Component, Entity, FlexDirection, Font, Handle,
  IntoScheduleConfigs, Justify, JustifyContent, LineBreak, MessageReader, Node, Pickable, Query, Res, Text, TextColor,
  TextFont, TextLayout, TextShadow, With, default, in_state,
};
use bevy::text::LineHeight;
use bevy::ui::{BackgroundColor, percent};
use mooplas_networking::prelude::NetworkRole;

/// A plugin that manages the online-only in-game lobby UI, such as remote player slots and the join prompt.
pub struct InGameOnlineUiPlugin;

impl Plugin for InGameOnlineUiPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(
      Update,
      handle_online_player_registration_message
        .run_if(in_state(AppState::Registering))
        .run_if(|network_role: Res<NetworkRole>| !network_role.is_none()),
    );
  }
}

/// The component for each player and their status in the lobby UI.
#[derive(Component)]
struct OnlineLobbyUiEntry {
  player_id: PlayerId,
}

/// Marker component for the prompt to join by pressing an available action key.
#[derive(Component)]
struct JoinPromptNode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OnlineLobbyUiEntryState {
  NotRegistered,
  RegisteredLocally { control_scheme_id: ControlSchemeId },
  RegisteredRemotely,
}

pub(crate) fn spawn_online_lobby_ui(
  commands: &mut Commands,
  root: Entity,
  font: &Handle<Font>,
  available_control_schemes: &AvailableControlSchemes,
  registered_players: &RegisteredPlayers,
) {
  for player_index in 0..MAX_PLAYERS {
    let player_id = PlayerId(player_index);
    let entry = commands
      .spawn((
        OnlineLobbyUiEntry { player_id },
        BackgroundColor::from(Color::BLACK.with_alpha(0.5)),
        Node {
          flex_direction: FlexDirection::Row,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          width: percent(100.),
          ..default()
        },
        Pickable::IGNORE,
      ))
      .id();
    commands.entity(root).add_child(entry);
    spawn_online_lobby_ui_entry_children(
      commands,
      entry,
      font,
      player_id,
      available_control_schemes,
      registered_players,
    );
  }

  // Join prompt
  let join_prompt = commands
    .spawn((
      JoinPromptNode,
      Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
      Pickable::IGNORE,
    ))
    .with_children(|parent| {
      spawn_join_prompt(parent, font, available_control_schemes, registered_players);
    })
    .id();
  commands.entity(root).add_child(join_prompt);
}

/// A system that handles player registration messages and updates the online-only lobby UI based on the player's
/// registration status.
fn handle_online_player_registration_message(
  mut commands: Commands,
  settings: Res<Settings>,
  mut player_registration_message: MessageReader<PlayerRegistrationMessage>,
  asset_server: Res<AssetServer>,
  available_control_schemes: Res<AvailableControlSchemes>,
  registered_players: Res<RegisteredPlayers>,
  online_entries_query: Query<(Entity, &OnlineLobbyUiEntry, &Children)>,
  join_prompt_query: Query<(Entity, Option<&Children>), With<JoinPromptNode>>,
  cta_query: Query<(Entity, &Children), With<in_game_ui::LobbyUiCta>>,
  network_role: Res<NetworkRole>,
) {
  for message in player_registration_message.read() {
    // The "table" showing players and their statuses
    let font = asset_server.load(DEFAULT_FONT);
    for (entity, entry, children) in online_entries_query.iter() {
      if entry.player_id != message.player_id {
        continue;
      }
      in_game_ui::clear_ui_children(&mut commands, children);
      spawn_online_lobby_ui_entry_children(
        &mut commands,
        entity,
        &font,
        entry.player_id,
        &available_control_schemes,
        &registered_players,
      );
    }

    // Join prompt
    update_join_prompt(
      &mut commands,
      &join_prompt_query,
      &asset_server,
      &available_control_schemes,
      &registered_players,
    );

    // Call to action
    in_game_ui::update_call_to_action_to_start(
      &mut commands,
      message.is_anyone_registered,
      &cta_query,
      &asset_server,
      &settings,
      !network_role.is_client(),
    );
  }
}

/// Spawns a single row for an online player in the lobby UI, showing the player slot, whether they're registered, and
/// their control scheme if registered. Examples:
/// - "Player 1: Registered remotely"
/// - "Player 2: Play with \[Z] and \[C]"
/// - "Player 3: Not registered"
fn spawn_online_lobby_ui_entry_children(
  commands: &mut Commands,
  entity: Entity,
  font: &Handle<Font>,
  player_id: PlayerId,
  available_control_schemes: &AvailableControlSchemes,
  registered_players: &RegisteredPlayers,
) {
  let entry_state = online_lobby_ui_entry_state(player_id, registered_players);
  commands.entity(entity).with_children(|parent| {
    parent.spawn(in_game_ui::player_slot_label(
      font,
      player_id,
      colour_for_player_id(player_id),
    ));
    match entry_state {
      OnlineLobbyUiEntryState::NotRegistered => {
        parent.spawn(player_not_registered_prompt(font));
      }
      OnlineLobbyUiEntryState::RegisteredLocally { control_scheme_id } => {
        let control_scheme = available_control_schemes
          .find_by_id(control_scheme_id)
          .unwrap_or_else(|| {
            panic!(
              "Failed to find control scheme [{:?}] for local player slot [{}]",
              control_scheme_id, player_id
            )
          });
        spawn_player_registered_with_keys_prompt(parent, control_scheme, font);
      }
      OnlineLobbyUiEntryState::RegisteredRemotely => {
        parent.spawn(player_registered_remotely_prompt(font));
      }
    }
  });
}

fn online_lobby_ui_entry_state(player_id: PlayerId, registered_players: &RegisteredPlayers) -> OnlineLobbyUiEntryState {
  match registered_players.players.iter().find(|player| player.id == player_id) {
    Some(player) if player.is_local() => OnlineLobbyUiEntryState::RegisteredLocally {
      control_scheme_id: player.input.id,
    },
    Some(_) => OnlineLobbyUiEntryState::RegisteredRemotely,
    None => OnlineLobbyUiEntryState::NotRegistered,
  }
}

fn player_not_registered_prompt(
  font: &Handle<Font>,
) -> (
  Node,
  SpawnRelatedBundle<ChildOf, Spawn<(Text, TextFont, TextLayout, TextColor, TextShadow)>>,
) {
  (
    Node {
      flex_direction: FlexDirection::Row,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },
    children![(
      Text::new(": Not registered"),
      in_game_ui::default_font(font),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      TEXT_COLOUR,
      in_game_ui::default_shadow(),
    )],
  )
}

fn spawn_player_registered_with_keys_prompt(
  parent: &mut RelatedSpawnerCommands<ChildOf>,
  control_scheme: &ControlScheme,
  font: &Handle<Font>,
) {
  let default_font = in_game_ui::default_font(font);
  let default_shadow = in_game_ui::default_shadow();

  parent
    .spawn((Node {
      flex_direction: FlexDirection::Row,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },))
    .with_children(|parent| {
      parent.spawn((
        Text::new(": Play with "),
        default_font.clone(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TEXT_COLOUR,
        default_shadow,
      ));
      parent.spawn((
        Text::new(format!("[{:?}]", control_scheme.left)),
        default_font.clone(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::from(ACCENT_COLOUR)),
        default_shadow,
      ));
      parent.spawn((
        Text::new(" and "),
        default_font.clone(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TEXT_COLOUR,
        default_shadow,
      ));
      parent.spawn((
        Text::new(format!("[{:?}]", control_scheme.right)),
        default_font,
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::from(ACCENT_COLOUR)),
        default_shadow,
      ));
    });
}

fn player_registered_remotely_prompt(
  font: &Handle<Font>,
) -> (
  Node,
  SpawnRelatedBundle<ChildOf, Spawn<(Text, TextFont, TextLayout, TextColor, TextShadow)>>,
) {
  (
    Node {
      flex_direction: FlexDirection::Row,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },
    children![(
      Text::new(": Registered remotely"),
      in_game_ui::default_font(font),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      TEXT_COLOUR,
      in_game_ui::default_shadow(),
    )],
  )
}

fn available_join_action_keys(
  available_control_schemes: &AvailableControlSchemes,
  registered_players: &RegisteredPlayers,
) -> Option<Vec<String>> {
  let keys: Vec<String> = available_control_schemes
    .schemes
    .iter()
    .filter(|control_scheme| {
      !registered_players
        .players
        .iter()
        .any(|player| player.is_local() && player.input.id == control_scheme.id)
    })
    .map(|control_scheme| format!("[{:?}]", control_scheme.action))
    .collect();

  if keys.is_empty() { None } else { Some(keys) }
}

fn update_join_prompt(
  commands: &mut Commands,
  join_prompt_query: &Query<(Entity, Option<&Children>), With<JoinPromptNode>>,
  asset_server: &Res<AssetServer>,
  available_control_schemes: &AvailableControlSchemes,
  registered_players: &RegisteredPlayers,
) {
  for (entity, children) in join_prompt_query.iter() {
    if let Some(children) = children {
      in_game_ui::clear_ui_children(commands, children);
    }
    let font = asset_server.load(DEFAULT_FONT);
    commands.entity(entity).with_children(|parent| {
      spawn_join_prompt(parent, &font, available_control_schemes, registered_players);
    });
  }
}

/// Spawns the prompt to join. Example: "Join by pressing \[A] or \[N]."
fn spawn_join_prompt(
  parent: &mut RelatedSpawnerCommands<ChildOf>,
  font: &Handle<Font>,
  available_control_schemes: &AvailableControlSchemes,
  registered_players: &RegisteredPlayers,
) {
  let Some(keys) = available_join_action_keys(available_control_schemes, registered_players) else {
    return;
  };
  let default_font = in_game_ui::default_font(font);
  let mut segments: Vec<(String, bool)> = vec![("Join by pressing ".to_string(), false)];
  match keys.as_slice() {
    [only_key] => {
      segments.push((only_key.clone(), true));
    }
    [first_key, second_key] => {
      segments.push((first_key.clone(), true));
      segments.push((" or ".to_string(), false));
      segments.push((second_key.clone(), true));
    }
    _ => {
      let last_index = keys.len() - 1;
      for (index, key) in keys.iter().enumerate() {
        if index > 0 {
          if index == last_index {
            segments.push((", or ".to_string(), false));
          } else {
            segments.push((", ".to_string(), false));
          }
        }
        segments.push((key.clone(), true));
      }
    }
  }
  segments.push((".".to_string(), false));

  for (text, is_key) in segments {
    let colour = if is_key {
      TextColor(Color::from(ACCENT_COLOUR))
    } else {
      TEXT_COLOUR
    };
    parent.spawn((
      Text::new(text),
      default_font.clone(),
      LineHeight::RelativeToFont(2.),
      colour,
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      in_game_ui::default_shadow(),
    ));
  }
}

#[cfg(all(test, feature = "online"))]
mod tests {
  use super::*;
  use crate::shared::RegisteredPlayer;
  use bevy::prelude::KeyCode;

  fn test_control_scheme(id: u8) -> ControlScheme {
    ControlScheme::new(
      ControlSchemeId(id),
      match id {
        0 => KeyCode::ArrowLeft,
        1 => KeyCode::Digit1,
        _ => KeyCode::KeyZ,
      },
      match id {
        0 => KeyCode::ArrowRight,
        1 => KeyCode::KeyA,
        _ => KeyCode::KeyC,
      },
      match id {
        0 => KeyCode::ArrowUp,
        1 => KeyCode::KeyQ,
        _ => KeyCode::KeyX,
      },
    )
  }

  #[test]
  fn online_lobby_ui_entry_state_returns_not_registered_for_empty_slot() {
    let registered_players = RegisteredPlayers::default();

    assert_eq!(
      online_lobby_ui_entry_state(PlayerId(4), &registered_players),
      OnlineLobbyUiEntryState::NotRegistered
    );
  }

  #[test]
  fn online_lobby_ui_entry_state_returns_local_registration_for_local_player() {
    let mut registered_players = RegisteredPlayers::default();
    registered_players
      .register(RegisteredPlayer::new_mutable(
        PlayerId(4),
        test_control_scheme(0),
        colour_for_player_id(PlayerId(4)),
      ))
      .expect("Expected the local player registration to succeed");

    assert_eq!(
      online_lobby_ui_entry_state(PlayerId(4), &registered_players),
      OnlineLobbyUiEntryState::RegisteredLocally {
        control_scheme_id: ControlSchemeId(0),
      }
    );
  }

  #[test]
  fn online_lobby_ui_entry_state_returns_remote_registration_for_remote_player() {
    let mut registered_players = RegisteredPlayers::default();
    registered_players
      .register(RegisteredPlayer::new_immutable(
        PlayerId(4),
        test_control_scheme(0),
        colour_for_player_id(PlayerId(4)),
      ))
      .expect("Expected the remote player registration to succeed");

    assert_eq!(
      online_lobby_ui_entry_state(PlayerId(4), &registered_players),
      OnlineLobbyUiEntryState::RegisteredRemotely
    );
  }

  #[test]
  fn available_join_action_keys_ignores_remote_registrations() {
    let available_control_schemes = AvailableControlSchemes {
      schemes: vec![test_control_scheme(0), test_control_scheme(1)],
    };
    let mut registered_players = RegisteredPlayers::default();
    registered_players
      .register(RegisteredPlayer::new_immutable(
        PlayerId(4),
        test_control_scheme(0),
        colour_for_player_id(PlayerId(4)),
      ))
      .expect("Expected the remote player registration to succeed");

    assert_eq!(
      available_join_action_keys(&available_control_schemes, &registered_players),
      Some(vec!["[ArrowUp]".to_string(), "[KeyQ]".to_string()])
    );
  }

  #[test]
  fn available_join_action_keys_omits_locally_registered_action_keys() {
    let available_control_schemes = AvailableControlSchemes {
      schemes: vec![test_control_scheme(0), test_control_scheme(1)],
    };
    let mut registered_players = RegisteredPlayers::default();
    registered_players
      .register(RegisteredPlayer::new_mutable(
        PlayerId(4),
        test_control_scheme(0),
        colour_for_player_id(PlayerId(4)),
      ))
      .expect("Expected the local player registration to succeed");

    assert_eq!(
      available_join_action_keys(&available_control_schemes, &registered_players),
      Some(vec!["[KeyQ]".to_string()])
    );
  }

  #[test]
  fn available_join_action_keys_returns_none_when_all_local_schemes_are_taken() {
    let available_control_schemes = AvailableControlSchemes {
      schemes: vec![test_control_scheme(0), test_control_scheme(1)],
    };
    let mut registered_players = RegisteredPlayers::default();
    registered_players
      .register(RegisteredPlayer::new_mutable(
        PlayerId(4),
        test_control_scheme(0),
        colour_for_player_id(PlayerId(4)),
      ))
      .expect("Expected the first local player registration to succeed");
    registered_players
      .register(RegisteredPlayer::new_mutable(
        PlayerId(5),
        test_control_scheme(1),
        colour_for_player_id(PlayerId(5)),
      ))
      .expect("Expected the second local player registration to succeed");

    assert_eq!(
      available_join_action_keys(&available_control_schemes, &registered_players),
      None
    );
  }
}
