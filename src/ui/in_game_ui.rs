use crate::app_states::AppState;
use crate::prelude::constants::{BUTTON_ALPHA_PRESSED, BUTTON_BORDER_WIDTH, DEFAULT_FONT};
use crate::prelude::{
  AvailablePlayerConfig, AvailablePlayerConfigs, PlayerId, RegisteredPlayers, Settings, TouchControlsToggledMessage,
  WinnerInfo,
};
use crate::shared::{ContinueMessage, PlayerRegistrationMessage};
use bevy::app::{Plugin, Update};
use bevy::asset::{AssetServer, Handle};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::ecs::children;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::ecs::spawn::SpawnRelatedBundle;
use bevy::input_focus::InputFocus;
use bevy::log::*;
use bevy::prelude::{
  AlignItems, Alpha, Bundle, Button, Changed, ChildOf, Children, Commands, Component, Entity, FlexDirection, Font,
  IntoScheduleConfigs, Justify, JustifyContent, LineBreak, MessageReader, MessageWriter, Node, OnEnter, OnExit, Query,
  Res, ResMut, Text, TextBackgroundColor, TextColor, TextFont, TextLayout, TextShadow, Val, With, ZIndex, default,
  in_state, px,
};
use bevy::prelude::{Spawn, SpawnRelated};
use bevy::text::LineHeight;
use bevy::ui::{BackgroundColor, BorderColor, BorderRadius, Interaction, PositionType, UiRect, percent};

/// A plugin that manages the in-game user interface, such as the lobby and game over screens.
pub struct InGameUiPlugin;

impl Plugin for InGameUiPlugin {
  fn build(&self, app: &mut bevy::prelude::App) {
    app
      .add_systems(OnEnter(AppState::Registering), spawn_lobby_ui_system)
      .add_systems(
        Update,
        (
          handle_player_registration_message,
          handle_touch_controls_toggled_message,
          toggle_touch_controls_button_system,
          continue_button_system,
        )
          .run_if(in_state(AppState::Registering)),
      )
      .add_systems(OnExit(AppState::Registering), despawn_lobby_ui_system)
      .add_systems(OnEnter(AppState::GameOver), spawn_game_over_ui_system)
      .add_systems(OnExit(AppState::GameOver), despawn_game_over_ui_system)
      .add_systems(Update, continue_button_system.run_if(in_state(AppState::GameOver)));
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

// TODO: Disable mouse so buttons work
/// Marker component for the touch controls toggle button.
#[derive(Component)]
struct ToggleTouchControlsButton;

/// Marker component for the touch continue button.
#[derive(Component)]
struct ContinueButton;

/// Sets up the lobby UI, displaying available players and prompts to join.
fn spawn_lobby_ui_system(
  mut commands: Commands,
  settings: Res<Settings>,
  asset_server: Res<AssetServer>,
  available_configs: Res<AvailablePlayerConfigs>,
  registered_players: Res<RegisteredPlayers>,
) {
  spawn_lobby_ui(
    &mut commands,
    &settings,
    &asset_server,
    &available_configs,
    &registered_players,
  );
}

fn spawn_lobby_ui(
  commands: &mut Commands,
  settings: &Res<Settings>,
  asset_server: &Res<AssetServer>,
  available_configs: &Res<AvailablePlayerConfigs>,
  registered_players: &Res<RegisteredPlayers>,
) {
  let font = asset_server.load(DEFAULT_FONT);
  let default_font = default_font(&font);
  let default_shadow = default_shadow();
  let is_touch_controlled = settings.general.enable_touch_controls;

  let root = commands
    .spawn((
      LobbyUiRoot,
      Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
      children![(
        Node {
          width: px(200),
          height: px(100),
          position_type: PositionType::Absolute,
          align_items: AlignItems::Center,
          justify_content: JustifyContent::Center,
          top: Val::ZERO,
          right: Val::ZERO,
          ..default()
        },
        ZIndex(1000),
        children![button(asset_server, ToggleTouchControlsButton, "Touch Controls", 22.)],
      )],
    ))
    .id();

  for available_config in &available_configs.configs {
    let colour = available_configs
      .configs
      .iter()
      .find(|p| p.id == available_config.id)
      .map(|p| p.colour)
      .unwrap_or(Color::WHITE);
    let is_registered = registered_players.players.iter().any(|p| p.id == available_config.id);
    let entry = commands
      .spawn((
        LobbyUiEntry {
          player_id: available_config.id,
        },
        BackgroundColor::from(Color::BLACK.with_alpha(0.5)),
        Node {
          flex_direction: FlexDirection::Row,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          width: percent(100.),
          ..default()
        },
      ))
      .with_children(|parent| {
        parent.spawn((
          // Player
          Text::new(format!("Player {}", available_config.id.0)),
          default_font.clone(),
          TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          TextColor(colour),
          default_shadow,
        ));
        if is_registered {
          parent.spawn(player_registered_prompt(&font));
        } else {
          parent.spawn(player_join_prompt(&font, available_config, is_touch_controlled));
        }
      })
      .id();
    commands.entity(root).add_child(entry);
  }

  // Call to action
  let has_any_registered = registered_players.players.len() > 0;
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
) {
  if !has_any_registered {
    parent.spawn((
      Text::new("More players needed to start..."),
      default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
      TextColor(Color::WHITE),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      default_shadow,
    ));
  } else {
    parent.spawn((
      Text::new("Press "),
      default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
      TextColor(Color::WHITE),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      default_shadow,
    ));
    if is_touch_controlled {
      parent.spawn(button(asset_server, ContinueButton, "HERE", 38.));
    } else {
      parent.spawn((
        Text::new("[Space]"),
        default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
        TextColor(Color::from(tailwind::YELLOW_400)),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        default_shadow,
      ));
    }
    parent.spawn((
      Text::new(" to start..."),
      default_font.clone().with_line_height(LineHeight::RelativeToFont(3.)),
      TextColor(Color::WHITE),
      TextLayout::new(Justify::Center, LineBreak::WordBoundary),
      default_shadow,
    ));
  }
}

fn button(asset_server: &AssetServer, button_type: impl Component, button_text: &str, font_size: f32) -> impl Bundle {
  (
    Button,
    button_type,
    Node {
      width: px(170),
      height: px(65),
      border: UiRect::all(px(BUTTON_BORDER_WIDTH)),
      justify_content: JustifyContent::Center, // Horizontally center child text
      align_items: AlignItems::Center,         // Vertically center child text
      padding: UiRect::all(px(2)),
      ..default()
    },
    BorderRadius::all(px(10)),
    BorderColor::all(Color::from(tailwind::SLATE_500)),
    BackgroundColor(Color::from(tailwind::SLATE_500.with_alpha(BUTTON_ALPHA_PRESSED))),
    children![(
      Text::new(button_text),
      TextFont {
        font: asset_server.load(DEFAULT_FONT),
        font_size,
        ..default()
      },
      TextColor(Color::srgb(0.9, 0.9, 0.9)),
      TextShadow::default(),
    )],
  )
}

/// A system that toggles touch controls when the corresponding button is pressed.
fn toggle_touch_controls_button_system(
  mut query: Query<&Interaction, (Changed<Interaction>, With<ToggleTouchControlsButton>)>,
  mut touch_controls_toggled_message: MessageWriter<TouchControlsToggledMessage>,
  mut settings: ResMut<Settings>,
) {
  for interaction in &mut query {
    if *interaction == Interaction::Pressed {
      settings.general.enable_touch_controls = !settings.general.enable_touch_controls;
      touch_controls_toggled_message.write(TouchControlsToggledMessage::new(settings.general.enable_touch_controls));
      info!(
        "[Button] Set touch controls to [{:?}]",
        settings.general.enable_touch_controls
      );
    }
  }
}

/// A system that handles the continue button press by sending [`ContinueMessage`].
fn continue_button_system(
  mut query: Query<&Interaction, (Changed<Interaction>, With<ContinueButton>)>,
  mut continue_message: MessageWriter<ContinueMessage>,
) {
  for interaction in &mut query {
    if *interaction == Interaction::Pressed {
      continue_message.write(ContinueMessage);
      info!("[Button] Pressed continue button");
    }
  }
}

/// A system that handles player registration messages and updates the lobby UI based on the player's registration
/// status.
fn handle_player_registration_message(
  mut commands: Commands,
  settings: Res<Settings>,
  mut player_registration_message: MessageReader<PlayerRegistrationMessage>,
  asset_server: Res<AssetServer>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut entries_query: Query<(Entity, &LobbyUiEntry, &Children)>,
  cta_query: Query<(Entity, &Children), With<LobbyUiCta>>,
) {
  for message in player_registration_message.read() {
    let font = asset_server.load(DEFAULT_FONT);
    let config = available_configs.configs.iter().find(|p| p.id == message.player_id);
    let is_touch_controlled = settings.general.enable_touch_controls;

    // Update entry for player
    match message.has_registered {
      false => {
        for (entity, entry, children) in &mut entries_query {
          if entry.player_id == message.player_id {
            if let Some(prompt_node) = children.get(1) {
              commands.entity(*prompt_node).despawn();
              if let Some(ref available_config) = config {
                let player = commands
                  .spawn(player_join_prompt(&font, available_config, is_touch_controlled))
                  .id();
                commands.entity(entity).add_child(player);
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
              let registered_prompt = commands.spawn(player_registered_prompt(&font)).id();
              commands.entity(entity).add_child(registered_prompt);
            }
          }
        }
      }
    }

    // Update call to action under player list
    update_call_to_action_to_start(
      &mut commands,
      message.is_anyone_registered,
      &cta_query,
      &asset_server,
      &settings,
    );
  }
}

fn player_join_prompt(
  font: &Handle<Font>,
  available_config: &AvailablePlayerConfig,
  is_touch_controlled: bool,
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
  let text_font = default_font(font);
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
        text_font.clone(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::WHITE),
        default_shadow,
      ),
      if !is_touch_controlled {
        (
          // ...[Key]...
          Text::new(format!("[{:?}]", available_config.input.action)),
          text_font.clone(),
          TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          TextColor(Color::from(tailwind::YELLOW_400)),
          default_shadow,
        )
      } else {
        (
          // ...your colour...
          Text::new("your colour"),
          text_font.clone(),
          TextLayout::new(Justify::Center, LineBreak::WordBoundary),
          TextColor(available_config.colour),
          default_shadow,
        )
      },
      (
        // ...to join
        Text::new(" to join"),
        text_font,
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(Color::WHITE),
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
      TextColor(Color::WHITE),
      default_shadow(),
    )],
  )
}

fn update_call_to_action_to_start(
  commands: &mut Commands,
  has_any_players: bool,
  cta_query: &Query<(Entity, &Children), With<LobbyUiCta>>,
  asset_server: &Res<AssetServer>,
  settings: &Res<Settings>,
) {
  for (entity, children) in cta_query.iter() {
    for child in children.iter() {
      commands.entity(*child).despawn();
    }
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
      );
    });
  }
}

/// A system that handles messages toggling touch controls to update the lobby UI's prompts accordingly. Makes sure that
/// the prompt doesn't ask for a key press when touch controls are enabled and vice versa.
fn handle_touch_controls_toggled_message(
  mut commands: Commands,
  mut messages: MessageReader<TouchControlsToggledMessage>,
  lobby_ui_root_query: Query<Entity, With<LobbyUiRoot>>,
  settings: Res<Settings>,
  asset_server: Res<AssetServer>,
  available_configs: Res<AvailablePlayerConfigs>,
  registered_players: Res<RegisteredPlayers>,
) {
  for _ in messages.read() {
    for entity in &lobby_ui_root_query {
      commands.entity(entity).despawn();
    }
    spawn_lobby_ui(
      &mut commands,
      &settings,
      &asset_server,
      &available_configs,
      &registered_players,
    );
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
) {
  let font = asset_server.load(DEFAULT_FONT);
  let default_shadow = default_shadow();

  commands
    .spawn((
      VictoryUiRoot,
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
      let large_text = large_text(&font);
      match winner.get() {
        Some(id) => {
          let colour = registered_players
            .players
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.colour)
            .unwrap_or(Color::WHITE);
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
                large_text.clone(),
                TextColor(colour),
                default_shadow,
                TextBackgroundColor::from(Color::BLACK.with_alpha(0.5)),
              ),
              (
                Text::new(" wins!  "),
                large_text.clone(),
                TextColor(Color::WHITE),
                default_shadow,
                TextBackgroundColor::from(Color::BLACK.with_alpha(0.5)),
              )
            ],
          ));
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
        .spawn(Node {
          flex_direction: FlexDirection::Row,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
        })
        .with_children(|parent| {
          parent.spawn((
            Text::new("Press "),
            default_font(&font).with_line_height(LineHeight::RelativeToFont(3.0)),
            TextColor(Color::WHITE),
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            default_shadow,
          ));

          if settings.general.enable_touch_controls {
            parent.spawn(button(&asset_server, ContinueButton, "HERE", 38.));
          } else {
            parent.spawn((
              Text::new("[Space]"),
              default_font(&font).with_line_height(LineHeight::RelativeToFont(3.0)),
              TextColor(Color::from(tailwind::YELLOW_400)),
              TextLayout::new(Justify::Center, LineBreak::WordBoundary),
              default_shadow,
            ));
          }

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
