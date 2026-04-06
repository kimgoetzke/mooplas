use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, DEFAULT_FONT, TEXT_COLOUR};
use crate::prelude::{
  AvailableControlSchemes, ControlScheme, ControlSchemeId, PlayerId, PlayerRegistrationMessage, RegisteredPlayers,
  Settings, colour_for_player_id,
};
use crate::ui::in_game_ui::in_game_ui::{
  LobbyUiCta, clear_ui_children, default_font, default_shadow, player_slot_label, update_call_to_action_to_start,
};
use bevy::app::{App, Plugin, Update};
use bevy::asset::{AssetServer, Handle};
use bevy::color::{Alpha, Color};
use bevy::ecs::children;
use bevy::ecs::spawn::SpawnRelatedBundle;
use bevy::picking::Pickable;
use bevy::prelude::{
  AlignItems, BackgroundColor, ChildOf, Children, Commands, Component, Entity, FlexDirection, Font,
  IntoScheduleConfigs, Justify, JustifyContent, LineBreak, MessageReader, Node, Query, Res, Spawn, Text, TextColor,
  TextFont, TextLayout, TextShadow, With, default, in_state, percent,
};
use mooplas_networking::prelude::NetworkRole;

/// A plugin that manages the local-only in-game lobby UI, such as rows with player colour/controls.
pub struct InGameLocalUiPlugin;

impl Plugin for InGameLocalUiPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(
      Update,
      handle_local_player_registration_message
        .run_if(in_state(AppState::Registering))
        .run_if(|network_role: Res<NetworkRole>| network_role.is_none()),
    );
  }
}

/// The component for each available player's information and status in the lobby UI.
#[derive(Component)]
pub(crate) struct LobbyUiEntry {
  pub(crate) control_scheme_id: ControlSchemeId,
}

/// A system that handles player registration messages and updates the local multiplayer lobby UI based on the player's
/// registration status.
fn handle_local_player_registration_message(
  mut commands: Commands,
  settings: Res<Settings>,
  mut player_registration_message: MessageReader<PlayerRegistrationMessage>,
  asset_server: Res<AssetServer>,
  available_control_schemes: Res<AvailableControlSchemes>,
  registered_players: Res<RegisteredPlayers>,
  local_entries_query: Query<(Entity, &LobbyUiEntry, &Children)>,
  cta_query: Query<(Entity, &Children), With<LobbyUiCta>>,
) {
  for message in player_registration_message.read() {
    let font = asset_server.load(DEFAULT_FONT);
    let is_touch_controlled = settings.general.enable_touch_controls;

    if let Some(control_scheme_id) = message.control_scheme_id {
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
      true,
    );
  }
}

pub(crate) fn spawn_local_lobby_ui(
  commands: &mut Commands,
  available_control_schemes: &&Res<AvailableControlSchemes>,
  registered_players: &Res<RegisteredPlayers>,
  font: &Handle<Font>,
  is_touch_controlled: bool,
  root: Entity,
) {
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
