use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, DEFAULT_COLOUR, DEFAULT_FONT, LARGE_FONT, NORMAL_FONT, TEXT_COLOUR};
use crate::prelude::{
  AvailableControlSchemes, ControlScheme, ControlSchemeId, MAX_PLAYERS, PlayerId, RegisteredPlayers, Settings,
  WinnerInfo, colour_for_player_id,
};
use crate::shared::PlayerRegistrationMessage;
use crate::ui::in_game_ui;
use crate::ui::in_game_ui::in_game_buttons::InGameButtonsPlugin;
use crate::ui::shared::despawn_menu;
use bevy::ecs::children;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::ecs::spawn::SpawnRelatedBundle;
use bevy::prelude::{
  AlignItems, Alpha, AssetServer, ChildOf, Children, Color, Commands, Component, Entity, FlexDirection, Font, Handle,
  IntoScheduleConfigs, Justify, JustifyContent, LineBreak, MessageReader, Name, Node, OnEnter, OnExit, Pickable,
  Plugin, Query, Res, Spawn, Text, TextBackgroundColor, TextColor, TextFont, TextLayout, TextShadow, Update, Val, With,
  default, in_state, px,
};
use bevy::text::LineHeight;
use bevy::ui::{BackgroundColor, PositionType, percent};
use in_game_ui::in_game_buttons;
use mooplas_networking::prelude::NetworkRole;

/// A plugin that manages the in-game user interface, such as the lobby and game over screens.
pub struct InGameUiPlugin;

impl Plugin for InGameUiPlugin {
  fn build(&self, app: &mut bevy::prelude::App) {
    app
      .add_plugins(InGameButtonsPlugin)
      .add_systems(OnEnter(AppState::Registering), spawn_lobby_ui_system)
      .add_systems(
        Update,
        (handle_player_registration_message,).run_if(in_state(AppState::Registering)),
      )
      .add_systems(OnExit(AppState::Registering), despawn_lobby_ui_system)
      .add_systems(OnEnter(AppState::GameOver), spawn_game_over_ui_system)
      .add_systems(OnExit(AppState::GameOver), despawn_game_over_ui_system);
  }
}

/// Marker component for the root of the lobby UI. Used for despawning. All other Lobby UI components must be children
/// of this.
#[derive(Component)]
pub(crate) struct LobbyUiRoot;

/// The component for each available player's information and status in the lobby UI.
#[derive(Component)]
struct LobbyUiEntry {
  control_scheme_id: ControlSchemeId,
}

#[derive(Component)]
struct OnlineLobbyUiEntry {
  player_id: PlayerId,
}

#[derive(Component)]
struct JoinByPressingPromptNode;

/// Marker component for the lobby UI call-to-action (CTA) at the bottom of the player list.
#[derive(Component)]
struct LobbyUiCta;

/// Marker component for the root of the victory/game over UI. Used for despawning. All other Victory UI components
/// must be children of this.
#[derive(Component)]
struct VictoryUiRoot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OnlineLobbyUiEntryState {
  NotRegistered,
  RegisteredLocally { control_scheme_id: ControlSchemeId },
  RegisteredRemotely,
}

/// Sets up the lobby UI, displaying available players and prompts to join.
fn spawn_lobby_ui_system(
  mut commands: Commands,
  settings: Res<Settings>,
  asset_server: Res<AssetServer>,
  available_control_schemes: Res<AvailableControlSchemes>,
  registered_players: Res<RegisteredPlayers>,
  network_role: Res<NetworkRole>,
) {
  spawn_lobby_ui(
    &mut commands,
    &settings,
    &asset_server,
    &available_control_schemes,
    &registered_players,
    &network_role,
  );
}

pub(crate) fn spawn_lobby_ui(
  commands: &mut Commands,
  settings: &Res<Settings>,
  asset_server: &Res<AssetServer>,
  available_control_schemes: &Res<AvailableControlSchemes>,
  registered_players: &Res<RegisteredPlayers>,
  network_role: &Res<NetworkRole>,
) {
  let font = asset_server.load(DEFAULT_FONT);
  let default_font = default_font(&font);
  let default_shadow = default_shadow();
  let is_touch_controlled = settings.general.enable_touch_controls;
  let is_permitted_action = !network_role.is_client();
  let is_online = is_online_role(network_role);

  let root = commands
    .spawn((
      LobbyUiRoot,
      Name::new("Lobby UI"),
      Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
      Pickable::IGNORE,
    ))
    .with_children(|parent| {
      // TODO: Replace all button text below with icons
      // Buttons at the top right
      parent
        .spawn(Node {
          width: px(470),
          height: px(100),
          position_type: PositionType::Absolute,
          align_items: AlignItems::Center,
          justify_content: JustifyContent::Center,
          top: Val::ZERO,
          right: Val::ZERO,
          ..default()
        })
        .with_children(|parent| {
          in_game_buttons::spawn_in_game_buttons(asset_server, parent);
        });
    })
    .id();

  if is_online {
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
        &font,
        player_id,
        available_control_schemes,
        registered_players,
      );
    }

    let join_by_pressing_prompt = commands
      .spawn((
        JoinByPressingPromptNode,
        Node {
          flex_direction: FlexDirection::Row,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
        },
        Pickable::IGNORE,
      ))
      .with_children(|parent| {
        spawn_join_by_pressing_prompt_children(parent, &font, available_control_schemes, registered_players);
      })
      .id();
    commands.entity(root).add_child(join_by_pressing_prompt);
  } else {
    for control_scheme in &available_control_schemes.schemes {
      let entry = commands
        .spawn((
          LobbyUiEntry {
            control_scheme_id: control_scheme.id,
          },
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
      spawn_local_lobby_ui_entry_children(
        commands,
        entry,
        &font,
        control_scheme,
        registered_players,
        is_touch_controlled,
      );
    }
  }

  // Call to action
  let has_any_registered = registered_players.count() > 0;
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
      spawn_call_to_action_to_start(
        asset_server,
        default_font,
        default_shadow,
        is_touch_controlled,
        has_any_registered,
        parent,
        is_permitted_action,
      );
    })
    .id();

  commands.entity(root).add_child(cta);
}

fn spawn_call_to_action_to_start(
  asset_server: &Res<AssetServer>,
  default_font: TextFont,
  default_shadow: TextShadow,
  is_touch_controlled: bool,
  has_any_registered: bool,
  parent: &mut RelatedSpawnerCommands<ChildOf>,
  is_permitted_action: bool,
) {
  if !has_any_registered {
    parent.spawn((
      Text::new("More players needed to start..."),
      default_font.clone(),
      LineHeight::RelativeToFont(3.),
      TEXT_COLOUR,
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      default_shadow,
    ));
  } else if !is_permitted_action {
    parent.spawn((
      Text::new("Waiting for host to start..."),
      default_font.clone(),
      LineHeight::RelativeToFont(3.),
      TEXT_COLOUR,
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      default_shadow,
    ));
  } else {
    parent.spawn((
      Text::new("Press "),
      default_font.clone(),
      LineHeight::RelativeToFont(3.),
      TEXT_COLOUR,
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      default_shadow,
    ));
    if is_touch_controlled {
      in_game_buttons::spawn_continue_button(asset_server, parent);
    } else {
      parent.spawn((
        Text::new("[Space]"),
        default_font.clone(),
        LineHeight::RelativeToFont(3.),
        TextColor(Color::from(ACCENT_COLOUR)),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        default_shadow,
      ));
    }
    parent.spawn((
      Text::new(" to start..."),
      default_font.clone(),
      LineHeight::RelativeToFont(3.),
      TEXT_COLOUR,
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      default_shadow,
    ));
  }
}

/// A system that handles player registration messages and updates the lobby UI based on the player's registration
/// status.
fn handle_player_registration_message(
  mut commands: Commands,
  settings: Res<Settings>,
  mut player_registration_message: MessageReader<PlayerRegistrationMessage>,
  asset_server: Res<AssetServer>,
  available_control_schemes: Res<AvailableControlSchemes>,
  registered_players: Res<RegisteredPlayers>,
  local_entries_query: Query<(Entity, &LobbyUiEntry, &Children)>,
  online_entries_query: Query<(Entity, &OnlineLobbyUiEntry, &Children)>,
  join_by_pressing_query: Query<(Entity, &Children), With<JoinByPressingPromptNode>>,
  cta_query: Query<(Entity, &Children), With<LobbyUiCta>>,
  network_role: Res<NetworkRole>,
) {
  for message in player_registration_message.read() {
    let font = asset_server.load(DEFAULT_FONT);
    let is_touch_controlled = settings.general.enable_touch_controls;
    let is_permitted_action = !network_role.is_client();
    let is_online = is_online_role(&network_role);

    if is_online {
      for (entity, entry, children) in online_entries_query.iter() {
        if entry.player_id != message.player_id {
          continue;
        }

        clear_ui_children(&mut commands, children);
        spawn_online_lobby_ui_entry_children(
          &mut commands,
          entity,
          &font,
          entry.player_id,
          &available_control_schemes,
          &registered_players,
        );
      }

      update_join_by_pressing_prompt(
        &mut commands,
        &join_by_pressing_query,
        &asset_server,
        &available_control_schemes,
        &registered_players,
      );
    } else if let Some(control_scheme_id) = message.control_scheme_id {
      let Some(control_scheme) = available_control_schemes.find_by_id(control_scheme_id) else {
        continue;
      };

      for (entity, entry, children) in local_entries_query.iter() {
        if entry.control_scheme_id != control_scheme_id {
          continue;
        }

        clear_ui_children(&mut commands, children);
        spawn_local_lobby_ui_entry_children(
          &mut commands,
          entity,
          &font,
          control_scheme,
          &registered_players,
          is_touch_controlled,
        );
      }
    }

    // Update call to action under player list
    update_call_to_action_to_start(
      &mut commands,
      message.is_anyone_registered,
      &cta_query,
      &asset_server,
      &settings,
      is_permitted_action,
    );
  }
}

fn player_join_prompt(
  font: &Handle<Font>,
  control_scheme: &ControlScheme,
  is_touch_controlled: bool,
  colour: Color,
) -> (
  Node,
  SpawnRelatedBundle<
    ChildOf,
    (
      Spawn<(Text, TextFont, TextLayout, TextColor, TextShadow)>,
      Spawn<(Text, TextFont, TextLayout, TextColor, TextShadow)>,
      Spawn<(Text, TextFont, TextLayout, TextColor, TextShadow)>,
    ),
  >,
) {
  let default_font = default_font(font);
  let default_shadow = default_shadow();

  (
    Node {
      flex_direction: FlexDirection::Row,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },
    children![
      (
        // Press...
        Text::new(": Press "),
        default_font.clone(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TEXT_COLOUR,
        default_shadow,
      ),
      if !is_touch_controlled {
        (
          // ...[Key]...
          Text::new(format!("[{:?}]", control_scheme.action)),
          default_font.clone(),
          TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          TextColor(Color::from(ACCENT_COLOUR)),
          default_shadow,
        )
      } else {
        (
          // ...your colour...
          Text::new("your colour"),
          default_font.clone(),
          TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          TextColor(colour),
          default_shadow,
        )
      },
      (
        // ...to join
        Text::new(" to join"),
        default_font,
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TEXT_COLOUR,
        default_shadow,
      )
    ],
  )
}

fn player_registered_prompt(
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
      Text::new(": Registered"),
      default_font(font),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      TEXT_COLOUR,
      default_shadow(),
    )],
  )
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
      default_font(font),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      TEXT_COLOUR,
      default_shadow(),
    )],
  )
}

fn player_registered_with_keys_prompt(
  font: &Handle<Font>,
  control_scheme: &ControlScheme,
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
      Text::new(format!(
        ": Play with {:?} and {:?}",
        control_scheme.left, control_scheme.right
      )),
      default_font(font),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      TEXT_COLOUR,
      default_shadow(),
    )],
  )
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
      default_font(font),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      TEXT_COLOUR,
      default_shadow(),
    )],
  )
}

fn join_by_pressing_prompt_text(
  available_control_schemes: &AvailableControlSchemes,
  registered_players: &RegisteredPlayers,
) -> Option<String> {
  let available_action_keys: Vec<String> = available_control_schemes
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

  match available_action_keys.as_slice() {
    [] => None,
    [only_key] => Some(format!("Join by pressing {}.", only_key)),
    [first_key, second_key] => Some(format!("Join by pressing {} or {}.", first_key, second_key)),
    _ => {
      let last_index = available_action_keys.len() - 1;
      let initial_keys = available_action_keys[..last_index].join(", ");
      Some(format!(
        "Join by pressing {}, or {}.",
        initial_keys, available_action_keys[last_index]
      ))
    }
  }
}

fn spawn_join_by_pressing_prompt_children(
  parent: &mut RelatedSpawnerCommands<ChildOf>,
  font: &Handle<Font>,
  available_control_schemes: &AvailableControlSchemes,
  registered_players: &RegisteredPlayers,
) {
  let Some(prompt_text) = join_by_pressing_prompt_text(available_control_schemes, registered_players) else {
    return;
  };
  parent.spawn((
    Text::new(prompt_text),
    default_font(font),
    LineHeight::RelativeToFont(2.),
    TEXT_COLOUR,
    TextLayout::new(Justify::Center, LineBreak::WordBoundary),
    default_shadow(),
  ));
}

fn update_join_by_pressing_prompt(
  commands: &mut Commands,
  join_by_pressing_query: &Query<(Entity, &Children), With<JoinByPressingPromptNode>>,
  asset_server: &Res<AssetServer>,
  available_control_schemes: &AvailableControlSchemes,
  registered_players: &RegisteredPlayers,
) {
  for (entity, children) in join_by_pressing_query.iter() {
    clear_ui_children(commands, children);

    let font = asset_server.load(DEFAULT_FONT);
    commands.entity(entity).with_children(|parent| {
      spawn_join_by_pressing_prompt_children(parent, &font, available_control_schemes, registered_players);
    });
  }
}

fn update_call_to_action_to_start(
  commands: &mut Commands,
  has_any_players: bool,
  cta_query: &Query<(Entity, &Children), With<LobbyUiCta>>,
  asset_server: &Res<AssetServer>,
  settings: &Res<Settings>,
  is_permitted_action: bool,
) {
  for (entity, children) in cta_query.iter() {
    clear_ui_children(commands, children);
    let font = asset_server.load(DEFAULT_FONT);
    let default_font = default_font(&font);
    let default_shadow = default_shadow();
    let is_touch_controlled = settings.general.enable_touch_controls;

    commands.entity(entity).with_children(|parent| {
      spawn_call_to_action_to_start(
        asset_server,
        default_font,
        default_shadow,
        is_touch_controlled,
        has_any_players,
        parent,
        is_permitted_action,
      );
    });
  }
}

/// Despawns the entire lobby UI. Call when exiting the registration state.
fn despawn_lobby_ui_system(mut commands: Commands, lobby_ui_root_query: Query<Entity, With<LobbyUiRoot>>) {
  for entity in &lobby_ui_root_query {
    commands.entity(entity).despawn();
  }
}

/// Spawns the game over UI, displaying the winner and a prompt to continue.
fn spawn_game_over_ui_system(
  mut commands: Commands,
  settings: Res<Settings>,
  winner: Res<WinnerInfo>,
  asset_server: Res<AssetServer>,
  registered_players: Res<RegisteredPlayers>,
  network_role: Res<NetworkRole>,
) {
  let font = asset_server.load(DEFAULT_FONT);
  let default_shadow = default_shadow();
  let is_permitted_action = !network_role.is_client();

  commands
    .spawn((
      VictoryUiRoot,
      Name::new("Victory UI"),
      Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
    ))
    .with_children(|parent| {
      // Match result
      let large_font = large_font(&font);
      match winner.get() {
        Some(id) => {
          let colour = registered_players
            .players
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.colour)
            .unwrap_or(DEFAULT_COLOUR);
          parent.spawn((
            Node {
              flex_direction: FlexDirection::Row,
              justify_content: JustifyContent::Center,
              align_items: AlignItems::Center,
              ..default()
            },
            children![
              (
                Text::new(format!("  Player {}", id.0)),
                large_font.clone(),
                TextColor(colour),
                default_shadow,
                TextBackgroundColor::from(Color::BLACK.with_alpha(0.5)),
              ),
              (
                Text::new(" wins!  "),
                large_font.clone(),
                TEXT_COLOUR,
                default_shadow,
                TextBackgroundColor::from(Color::BLACK.with_alpha(0.5)),
              )
            ],
          ));
        }
        None => {
          parent.spawn((
            Text::new("No winner this round."),
            large_font.clone(),
            TEXT_COLOUR,
            default_shadow,
          ));
        }
      }

      // Call to action
      parent
        .spawn(Node {
          flex_direction: FlexDirection::Row,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
        })
        .with_children(|parent| {
          if !is_permitted_action {
            parent.spawn((
              Text::new("Waiting for host to continue..."),
              default_font(&font),
              LineHeight::RelativeToFont(3.0),
              TEXT_COLOUR,
              TextLayout::new(Justify::Center, LineBreak::WordBoundary),
              default_shadow,
            ));
            return;
          }

          parent.spawn((
            Text::new("Press "),
            default_font(&font),
            LineHeight::RelativeToFont(3.0),
            TEXT_COLOUR,
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            default_shadow,
          ));

          if settings.general.enable_touch_controls {
            in_game_buttons::spawn_continue_button(&asset_server, parent);
          } else {
            parent.spawn((
              Text::new("[Space]"),
              default_font(&font),
              LineHeight::RelativeToFont(3.0),
              TextColor(Color::from(ACCENT_COLOUR)),
              TextLayout::new(Justify::Center, LineBreak::WordBoundary),
              default_shadow,
            ));
          }

          parent.spawn((
            Text::new(" to continue..."),
            default_font(&font),
            LineHeight::RelativeToFont(3.0),
            TEXT_COLOUR,
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            default_shadow,
          ));
        });
    });
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
    parent.spawn(player_slot_label(font, player_id, colour_for_player_id(player_id)));
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
        parent.spawn(player_registered_with_keys_prompt(font, control_scheme));
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

/// Spawns a single row for a local player in the lobby UI, showing the player slot, whether they're registered, and
/// how to register, if not registered. Examples:
/// - "Player 1: Registered"
/// - "Player 4: Press \[Home] to join"
fn spawn_local_lobby_ui_entry_children(
  commands: &mut Commands,
  entity: Entity,
  font: &Handle<Font>,
  control_scheme: &ControlScheme,
  registered_players: &RegisteredPlayers,
  is_touch_controlled: bool,
) {
  let player_id = PlayerId(control_scheme.id.0);
  let is_registered = registered_players
    .get_local_player_id_for_control_scheme(control_scheme.id)
    .is_some();
  let join_prompt_colour = colour_for_player_id(player_id);
  commands.entity(entity).with_children(|parent| {
    parent.spawn(player_slot_label(font, player_id, colour_for_player_id(player_id)));
    if is_registered {
      parent.spawn(player_registered_prompt(font));
      return;
    }
    parent.spawn(player_join_prompt(
      font,
      control_scheme,
      is_touch_controlled,
      join_prompt_colour,
    ));
  });
}

fn clear_ui_children(commands: &mut Commands, children: &Children) {
  for child in children.iter() {
    commands.entity(*child).despawn();
  }
}

fn player_slot_label(
  font: &Handle<Font>,
  player_id: PlayerId,
  slot_label_colour: Color,
) -> (Text, TextFont, TextLayout, TextColor, TextShadow) {
  (
    Text::new(format!("Player {}", player_id.0)),
    default_font(font),
    TextLayout::new(Justify::Center, LineBreak::WordBoundary),
    TextColor(slot_label_colour),
    default_shadow(),
  )
}

fn is_online_role(network_role: &NetworkRole) -> bool {
  !network_role.is_none()
}

fn default_font(font: &Handle<Font>) -> TextFont {
  TextFont {
    font: font.clone(),
    font_size: NORMAL_FONT,
    ..default()
  }
}

fn large_font(font: &Handle<Font>) -> TextFont {
  TextFont {
    font: font.clone(),
    font_size: LARGE_FONT,
    ..default()
  }
}

fn default_shadow() -> TextShadow {
  TextShadow::default()
}

/// Despawns the entire game over UI. Call when exiting the game over state.
fn despawn_game_over_ui_system(mut commands: Commands, victory_ui_root_query: Query<Entity, With<VictoryUiRoot>>) {
  despawn_menu(&mut commands, &victory_ui_root_query);
}

#[cfg(all(test, feature = "online"))]
mod tests {
  use super::*;
  #[cfg(feature = "online")]
  use crate::shared::RegisteredPlayer;
  #[cfg(feature = "online")]
  use bevy::prelude::KeyCode;

  #[cfg(feature = "online")]
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

  #[cfg(feature = "online")]
  #[test]
  fn online_lobby_ui_entry_state_returns_not_registered_for_empty_slot() {
    let registered_players = RegisteredPlayers::default();

    assert_eq!(
      online_lobby_ui_entry_state(PlayerId(4), &registered_players),
      OnlineLobbyUiEntryState::NotRegistered
    );
  }

  #[cfg(feature = "online")]
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

  #[cfg(feature = "online")]
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

  #[cfg(feature = "online")]
  #[test]
  fn join_by_pressing_prompt_text_ignores_remote_registrations() {
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
      join_by_pressing_prompt_text(&available_control_schemes, &registered_players),
      Some("Join by pressing [ArrowUp] or [KeyQ].".to_string())
    );
  }

  #[cfg(feature = "online")]
  #[test]
  fn join_by_pressing_prompt_text_omits_locally_registered_action_keys() {
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
      join_by_pressing_prompt_text(&available_control_schemes, &registered_players),
      Some("Join by pressing [KeyQ].".to_string())
    );
  }

  #[cfg(feature = "online")]
  #[test]
  fn join_by_pressing_prompt_text_returns_none_when_all_local_schemes_are_taken() {
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
      join_by_pressing_prompt_text(&available_control_schemes, &registered_players),
      None
    );
  }
}
