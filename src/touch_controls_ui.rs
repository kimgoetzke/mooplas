use crate::app_states::AppState;
use crate::prelude::{AvailablePlayerConfig, AvailablePlayerConfigs, PlayerId, Settings, TouchControlsToggledMessage};
use crate::shared::InputAction;
use avian2d::math::Scalar;
use bevy::color::palettes::tailwind;
use bevy::input_focus::InputFocus;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::fmt::Debug;

pub struct TouchControlsUiPlugin;

impl Plugin for TouchControlsUiPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(MeshPickingPlugin)
      .init_resource::<InputFocus>()
      .init_resource::<ActiveMovementTracker>()
      .add_systems(Startup, spawn_touch_controls_ui_system)
      .add_systems(Update, button_design_system)
      .add_systems(
        Update,
        player_movement_input_action_emitter_system.run_if(in_state(AppState::Playing)),
      )
      .add_systems(Update, handle_toggle_touch_controls_message);
  }
}

const BUTTON_ALPHA_DEFAULT: f32 = 0.3;
const BUTTON_ALPHA_PRESSED: f32 = 0.8;
const BUTTON_WIDTH: f32 = 60.0;
const BUTTON_HEIGHT: f32 = 50.0;
const BUTTON_MARGIN: f32 = 15.0;
const BUTTON_BORDER_WIDTH: f32 = 2.0;

#[derive(Component)]
struct TouchControlsUi;

#[derive(Component, Clone)]
struct TouchButton;

#[derive(Component, Clone, Copy, Debug)]
struct ButtonMovement {
  player_id: PlayerId,
  direction: Scalar,
}

impl ButtonMovement {
  fn new(player_id: PlayerId, direction: Scalar) -> Self {
    Self { player_id, direction }
  }
}

impl Into<InputAction> for &ButtonMovement {
  fn into(self) -> InputAction {
    InputAction::Move(self.player_id, self.direction)
  }
}

#[derive(Component, Clone, Copy, Debug)]
struct ButtonAction {
  player_id: PlayerId,
}

impl ButtonAction {
  fn new(player_id: PlayerId) -> Self {
    Self { player_id }
  }
}

impl Into<InputAction> for &ButtonAction {
  fn into(self) -> InputAction {
    InputAction::Action(self.player_id)
  }
}

// Resource that tracks currently active movements per player.
#[derive(Resource, Default)]
struct ActiveMovementTracker {
  players: HashMap<PlayerId, (Entity, ButtonMovement)>,
}

/// A system that spawns the touch controls UI if enabled in settings. Intended to be called on startup.
fn spawn_touch_controls_ui_system(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  settings: Res<Settings>,
  available_configs: Res<AvailablePlayerConfigs>,
) {
  if !settings.general.enable_touch_controls {
    return;
  }

  spawn_touch_controls_ui(&mut commands, &available_configs, &asset_server);
}

// TODO: Design better buttons
/// Spawns the touch controls UI.
fn spawn_touch_controls_ui(
  commands: &mut Commands,
  available_configs: &AvailablePlayerConfigs,
  _asset_server: &Res<AssetServer>,
) {
  // let default_font = asset_server.load(DEFAULT_FONT);
  let parent = commands
    .spawn((
      TouchControlsUi,
      Node {
        width: percent(100),
        height: percent(100),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      ZIndex::from(ZIndex(100)),
    ))
    .id();

  for config in available_configs.configs.iter() {
    commands.entity(parent).with_children(|parent| {
      let node = button_node();
      let button = button_with_style();
      parent
        .spawn((
          Name::new("Controls for Player ".to_string() + &config.id.to_string()),
          controls_positioning_node(config),
        ))
        .with_children(|parent| {
          parent
            .spawn((
              // Left movement button
              node.clone(),
              button.clone(),
              ButtonMovement::new(config.id.into(), -1.0),
              BorderRadius {
                top_left: Val::Percent(50.0),
                bottom_left: Val::Percent(50.0),
                top_right: Val::Percent(20.0),
                bottom_right: Val::Percent(20.0),
              },
              config.id,
            ))
            .observe(start_movement_by_pressing)
            .observe(start_movement_by_hovering_over)
            .observe(stop_player_movement_by_moving_outside_button_bounds)
            .observe(stop_player_movement_by_releasing);

          parent
            .spawn((
              // Player action button
              node.clone(),
              button_with_custom_style(config.colour),
              ButtonAction::new(config.id.into()),
              BorderRadius::all(Val::Percent(20.0)),
              config.id,
            ))
            .observe(click_player_action);

          parent
            .spawn((
              // Right movement button
              node.clone(),
              button.clone(),
              ButtonMovement::new(config.id.into(), 1.0),
              BorderRadius {
                top_left: Val::Percent(20.0),
                bottom_left: Val::Percent(20.0),
                top_right: Val::Percent(50.0),
                bottom_right: Val::Percent(50.0),
              },
              config.id,
            ))
            .observe(start_movement_by_pressing)
            .observe(start_movement_by_hovering_over)
            .observe(stop_player_movement_by_moving_outside_button_bounds)
            .observe(stop_player_movement_by_releasing);
        });
    });
  }
}

fn click_player_action(
  click: On<Pointer<Click>>,
  mut button_query: Query<Option<&ButtonAction>, With<TouchButton>>,
  mut input_action_writer: MessageWriter<InputAction>,
) {
  if let Ok(button_action) = button_query.get_mut(click.entity) {
    if let Some(action) = button_action {
      input_action_writer.write(action.into());
    }
  }
}

/// Starts movement for a player when they press a movement button.
fn start_movement_by_pressing(
  action: On<Pointer<Press>>,
  mut tracker: ResMut<ActiveMovementTracker>,
  mut input_action_writer: MessageWriter<InputAction>,
  button_query: Query<&ButtonMovement>,
) {
  start_player_movement(action, &mut tracker, &mut input_action_writer, button_query);
}

/// Starts movement for a player when they hover over a movement button. This is to support clicking just outside the
/// button and then moving your finger onto the button to start movement.
fn start_movement_by_hovering_over(
  action: On<Pointer<Over>>,
  mut tracker: ResMut<ActiveMovementTracker>,
  mut input_action_writer: MessageWriter<InputAction>,
  button_query: Query<&ButtonMovement>,
) {
  start_player_movement(action, &mut tracker, &mut input_action_writer, button_query);
}

fn start_player_movement<T: 'static + Clone + Debug + Reflect>(
  action: On<Pointer<T>>,
  tracker: &mut ResMut<ActiveMovementTracker>,
  input_action_writer: &mut MessageWriter<InputAction>,
  button_query: Query<&ButtonMovement>,
) {
  if let Ok(movement) = button_query.get(action.entity) {
    tracker.players.insert(movement.player_id, (action.entity, *movement));
    input_action_writer.write(movement.into());
  }
}

/// Stops movement for a player when they release a movement button.
fn stop_player_movement_by_releasing(action: On<Pointer<Release>>, mut tracker: ResMut<ActiveMovementTracker>) {
  remove_player_from_movement_tracker(action, &mut tracker);
}

/// Stops movement for a player when they move their pointer/finger outside the button bounds.
fn stop_player_movement_by_moving_outside_button_bounds(
  action: On<Pointer<Out>>,
  mut tracker: ResMut<ActiveMovementTracker>,
) {
  remove_player_from_movement_tracker(action, &mut tracker);
}

fn remove_player_from_movement_tracker<T: 'static + Clone + Debug + Reflect>(
  action: On<Pointer<T>>,
  tracker: &mut ResMut<ActiveMovementTracker>,
) {
  if let Some(player) = tracker
    .players
    .iter()
    .find(|(_, (ent, _))| *ent == action.entity)
    .map(|(p, _)| *p)
  {
    tracker.players.remove(&player);
  }
}

/// Per-frame emitter system for [`InputAction::Move`] for every active movement of every player. Reads the current
/// active movements from the [`ActiveMovementTracker`] resource and emits corresponding input actions.
fn player_movement_input_action_emitter_system(
  tracker: Res<ActiveMovementTracker>,
  mut input_action_writer: MessageWriter<InputAction>,
) {
  if tracker.players.is_empty() {
    return;
  }
  for (_player, (_entity, movement)) in tracker.players.iter() {
    input_action_writer.write(movement.into());
  }
}

fn controls_positioning_node(config: &AvailablePlayerConfig) -> (Node, UiTransform) {
  match config.id.0 {
    0 | 1 => (
      Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(10.0),
        left: Val::Percent(33.0 + (config.id.0 as f32) * 33.0),
        margin: UiRect::all(Val::Px(10.0)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      UiTransform {
        translation: Val2::new(Val::Px(horizontal_offset()), Val::Auto),
        ..default()
      },
    ),
    2 => (
      Node {
        position_type: PositionType::Absolute,
        top: Val::Percent(50.0),
        right: Val::ZERO,
        margin: UiRect::all(Val::Px(10.0)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      UiTransform {
        translation: Val2::new(Val::Px(BUTTON_HEIGHT), Val::Px(vertical_offset())),
        rotation: Rot2::degrees(90.0),
        ..default()
      },
    ),
    3 => (
      Node {
        position_type: PositionType::Absolute,
        top: Val::Px(10.),
        left: Val::Percent(50.),
        margin: UiRect::all(Val::Px(10.)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      UiTransform {
        translation: Val2::new(Val::Px(horizontal_offset()), Val::Auto),
        rotation: Rot2::degrees(180.),
        ..default()
      },
    ),
    4 => (
      Node {
        position_type: PositionType::Absolute,
        top: Val::Percent(50.0),
        left: Val::ZERO,
        margin: UiRect::all(Val::Px(10.0)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      UiTransform {
        translation: Val2::new(Val::Px(-BUTTON_HEIGHT), Val::Px(vertical_offset())),
        rotation: Rot2::degrees(270.0),
        ..default()
      },
    ),
    _ => panic!("Unsupported player ID for touch controls UI: {}", config.id.0),
  }
}

fn horizontal_offset() -> f32 {
  -((((BUTTON_WIDTH + ((BUTTON_MARGIN + BUTTON_BORDER_WIDTH) * 2.0)) * 3.) / 2.) + 4.)
}

fn vertical_offset() -> f32 {
  -((BUTTON_HEIGHT / 3.) + (BUTTON_MARGIN + BUTTON_BORDER_WIDTH) * 2.)
}

fn button_node() -> Node {
  Node {
    width: px(BUTTON_WIDTH),
    height: px(BUTTON_HEIGHT),
    border: UiRect::all(px(BUTTON_BORDER_WIDTH)),
    justify_content: JustifyContent::Center,
    align_items: AlignItems::Center,
    margin: UiRect::all(px(BUTTON_MARGIN)),
    ..default()
  }
}

fn button_with_style() -> (TouchButton, BackgroundColor, BorderColor) {
  (
    TouchButton,
    BackgroundColor(Color::from(tailwind::SLATE_600).with_alpha(BUTTON_ALPHA_DEFAULT)),
    BorderColor::all(Color::from(tailwind::SLATE_500).with_alpha(0.2)),
  )
}

fn button_with_custom_style(colour: Color) -> (TouchButton, BackgroundColor, BorderColor) {
  (
    TouchButton,
    BackgroundColor(Color::from(colour).with_alpha(BUTTON_ALPHA_DEFAULT)),
    BorderColor::all(Color::from(tailwind::SLATE_500).with_alpha(0.2)),
  )
}

/// A system that handles toggling the touch controls UI via messages.
fn handle_toggle_touch_controls_message(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut messages: MessageReader<TouchControlsToggledMessage>,
  mut touch_controls_ui_query: Query<Entity, With<TouchControlsUi>>,
  available_configs: Res<AvailablePlayerConfigs>,
) {
  for message in messages.read() {
    if message.enabled {
      spawn_touch_controls_ui(&mut commands, &available_configs, &asset_server);
    } else {
      touch_controls_ui_query
        .iter_mut()
        .for_each(|e| commands.entity(e).despawn());
    }
  }
}

// TODO: Absorb into observers above
fn button_design_system(
  mut input_focus: ResMut<InputFocus>,
  mut interaction_query: Query<
    (
      Entity,
      &Interaction,
      &mut BorderColor,
      &mut BackgroundColor,
      &mut TouchButton,
    ),
    Changed<Interaction>,
  >,
) {
  for (entity, interaction, mut border_colour, mut background_colour, mut button) in &mut interaction_query {
    match *interaction {
      Interaction::Pressed => {
        input_focus.set(entity);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_100));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      Interaction::Hovered => {
        input_focus.set(entity);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_300));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
        button.set_changed();
      }
      Interaction::None => {
        input_focus.clear();
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
      }
    }
  }
}
