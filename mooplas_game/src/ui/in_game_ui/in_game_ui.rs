use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, DEFAULT_COLOUR, DEFAULT_FONT, LARGE_FONT, NORMAL_FONT, TEXT_COLOUR};
use crate::prelude::{AvailableControlSchemes, PlayerId, RegisteredPlayers, Settings, WinnerInfo};
use crate::ui::in_game_ui::in_game_buttons::InGameButtonsPlugin;
use crate::ui::in_game_ui::in_game_local_ui::InGameLocalUiPlugin;
use crate::ui::in_game_ui::in_game_online_ui::{self, InGameOnlineUiPlugin};
use crate::ui::in_game_ui::{in_game_buttons, in_game_local_ui};
use crate::ui::shared::despawn_menu;
use bevy::ecs::children;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::{
  AlignItems, Alpha, AssetServer, ChildOf, Children, Color, Commands, Component, Entity, FlexDirection, Font, Handle,
  Justify, JustifyContent, LineBreak, Name, Node, OnEnter, OnExit, Pickable, Plugin, Query, Res, Text,
  TextBackgroundColor, TextColor, TextFont, TextLayout, TextShadow, Val, With, default, px,
};
use bevy::text::LineHeight;
use bevy::ui::{PositionType, percent};
use mooplas_networking::prelude::NetworkRole;

/// A plugin that manages the in-game user interface, such as the lobby and game over screens.
pub struct InGameUiPlugin;

impl Plugin for InGameUiPlugin {
  fn build(&self, app: &mut bevy::prelude::App) {
    app
      .add_plugins((InGameButtonsPlugin, InGameLocalUiPlugin))
      .add_systems(OnEnter(AppState::Registering), spawn_lobby_ui_system)
      .add_systems(OnExit(AppState::Registering), despawn_lobby_ui_system)
      .add_systems(OnEnter(AppState::GameOver), spawn_game_over_ui_system)
      .add_systems(OnExit(AppState::GameOver), despawn_game_over_ui_system);

    #[cfg(feature = "online")]
    app.add_plugins(InGameOnlineUiPlugin);
  }
}

/// Marker component for the root of the lobby UI. Used for despawning. All other Lobby UI components must be children
/// of this.
#[derive(Component)]
pub(crate) struct LobbyUiRoot;

/// Marker component for the lobby UI call-to-action (CTA) at the bottom of the player list.
#[derive(Component)]
pub(crate) struct LobbyUiCta;

/// Marker component for the root of the victory/game over UI. Used for despawning. All other Victory UI components
/// must be children of this.
#[derive(Component)]
struct VictoryUiRoot;

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

  // The "table" showing players and their statuses
  if network_role.is_none() {
    in_game_local_ui::spawn_local_lobby_ui(
      commands,
      &available_control_schemes,
      registered_players,
      &font,
      is_touch_controlled,
      root,
    );
  } else {
    in_game_online_ui::spawn_online_lobby_ui(commands, root, &font, available_control_schemes, registered_players);
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

pub(crate) fn update_call_to_action_to_start(
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

pub(crate) fn clear_ui_children(commands: &mut Commands, children: &Children) {
  for child in children.iter() {
    commands.entity(*child).despawn();
  }
}

pub(crate) fn player_slot_label(
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

pub(crate) fn default_font(font: &Handle<Font>) -> TextFont {
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

pub(crate) fn default_shadow() -> TextShadow {
  TextShadow::default()
}

/// Despawns the entire game over UI. Call when exiting the game over state.
fn despawn_game_over_ui_system(mut commands: Commands, victory_ui_root_query: Query<Entity, With<VictoryUiRoot>>) {
  despawn_menu(&mut commands, &victory_ui_root_query);
}
