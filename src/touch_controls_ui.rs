use crate::app_states::AppState;
use crate::prelude::constants::{RESOLUTION_HEIGHT, RESOLUTION_WIDTH};
use crate::prelude::{AvailablePlayerConfig, AvailablePlayerConfigs, PlayerId, Settings, TouchControlsToggledMessage};
use crate::shared::InputAction;
use avian2d::math::Scalar;
use bevy::color::palettes::tailwind;
use bevy::input_focus::InputFocus;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct TouchControlsUiPlugin;

impl Plugin for TouchControlsUiPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, spawn_touch_controls_ui_system)
      .init_resource::<InputFocus>()
      .add_systems(
        Update,
        (
          button_design_system,
          button_action_input_system,
          handle_touch_input_system,
          debug_touch_input_system,
          debug_button_hit_gizmos_system,
        ),
      )
      .add_systems(Update, button_movement_input_system.run_if(in_state(AppState::Playing)))
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
struct TouchButton {
  size: Vec2,
}

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

/// Tracks which touch IDs are currently pressing which buttons
#[derive(Resource, Default)]
struct TouchState {
  active_touches: HashMap<u64, Entity>,
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

  commands.init_resource::<TouchState>();
  spawn_touch_controls_ui(&mut commands, &available_configs, &asset_server);
}

// TODO: Merge controls for a single player into one set of buttons
// TODO: Design better buttons
// TODO: Replace text with icons
// TODO: Position buttons around the screen for more comfortable access
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
      parent.spawn((
        Name::new("Controls for Player ".to_string() + &config.id.to_string()),
        controls_positioning_node(config),
        children![
          (
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
          ),
          (
            // Player action button
            node.clone(),
            button_with_custom_style(config.colour),
            ButtonAction::new(config.id.into()),
            BorderRadius::all(Val::Percent(20.0)),
            config.id,
          ),
          (
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
          )
        ],
      ));
    });
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

fn button_with_style() -> (/*Button,*/ TouchButton, BackgroundColor, BorderColor) {
  (
    // Button,
    TouchButton {
      size: Vec2::new(BUTTON_WIDTH, BUTTON_HEIGHT),
    },
    BackgroundColor(Color::from(tailwind::SLATE_600).with_alpha(BUTTON_ALPHA_DEFAULT)),
    BorderColor::all(Color::from(tailwind::SLATE_500).with_alpha(0.2)),
  )
}

fn button_with_custom_style(colour: Color) -> (/*Button,*/ TouchButton, BackgroundColor, BorderColor) {
  (
    // Button,
    TouchButton {
      size: Vec2::new(BUTTON_WIDTH, BUTTON_HEIGHT),
    },
    BackgroundColor(Color::from(colour).with_alpha(BUTTON_ALPHA_DEFAULT)),
    BorderColor::all(Color::from(tailwind::SLATE_500).with_alpha(0.2)),
  )
}

/// A system that handles toggling the touch controls UI via messages.
fn handle_toggle_touch_controls_message(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut messages: ResMut<Messages<TouchControlsToggledMessage>>,
  mut query: Query<Entity, With<TouchControlsUi>>,
  available_configs: Res<AvailablePlayerConfigs>,
) {
  for message in messages.drain() {
    if message.enabled {
      commands.init_resource::<TouchState>();
      spawn_touch_controls_ui(&mut commands, &available_configs, &asset_server);
    } else {
      query.iter_mut().for_each(|e| {
        commands.entity(e).despawn();
      });
      commands.remove_resource::<TouchState>();
    }
  }
}

fn debug_touch_input_system(
  touches: Res<Touches>,
  mut gizmos: Gizmos,
  window_query: Query<&Window, With<PrimaryWindow>>,
) {
  let Ok(window) = window_query.single() else { return };

  // Same logical canvas size used for the in-game world
  let canvas_w = RESOLUTION_WIDTH as f32;
  let canvas_h = RESOLUTION_HEIGHT as f32;

  // Compute integer scale that fit_canvas_system uses
  let h_scale = window.width() / canvas_w;
  let v_scale = window.height() / canvas_h;
  let scale = h_scale.min(v_scale).round().max(1.0);

  // Size of the rendered canvas in *window* pixels
  let rendered_w = canvas_w * scale;
  let rendered_h = canvas_h * scale;

  // Letterbox offsets (canvas is centered in the window)
  let offset_x = (window.width() - rendered_w) * 0.5;
  let offset_y = (window.height() - rendered_h) * 0.5;

  for touch in touches.iter() {
    let screen = touch.position(); // window space, origin top-left

    // Map from window pixels to canvas pixels (origin top-left)
    let canvas_x = (screen.x - offset_x) / scale;
    let canvas_y = (screen.y - offset_y) / scale;

    // Shift origin to canvas center and flip Y to match your world
    let world_x = canvas_x - canvas_w * 0.5;
    let world_y = (canvas_h * 0.5) - canvas_y;

    let position = Vec2::new(world_x, world_y);
    gizmos.circle_2d(position, 30.0, Color::srgb(1.0, 0.0, 0.0));
  }
}

fn debug_button_hit_gizmos_system(
  touches: Res<Touches>,
  mut gizmos: Gizmos,
  window_query: Query<&Window, With<PrimaryWindow>>,
  button_query: Query<(&UiGlobalTransform, &TouchButton), With<TouchButton>>,
) {
  let Ok(window) = window_query.single() else { return };

  // logical canvas size (same as your game world / UI)
  let canvas_w = RESOLUTION_WIDTH as f32;
  let canvas_h = RESOLUTION_HEIGHT as f32;

  // compute integer scale and letterbox offsets (same as your other systems)
  let h_scale = window.width() / canvas_w;
  let v_scale = window.height() / canvas_h;
  let scale = h_scale.min(v_scale).round().max(1.0);
  let rendered_w = canvas_w * scale;
  let rendered_h = canvas_h * scale;
  let offset_x = (window.width() - rendered_w) * 0.5;
  let offset_y = (window.height() - rendered_h) * 0.5;

  // helpers
  let to_canvas = |p: Vec2| -> Option<Vec2> {
    let cx = (p.x - offset_x) / scale;
    let cy = (p.y - offset_y) / scale;
    if cx < 0.0 || cx > canvas_w || cy < 0.0 || cy > canvas_h {
      None
    } else {
      Some(Vec2::new(cx, cy))
    }
  };

  let canvas_to_world = |c: Vec2| -> Vec2 {
    // shift origin to canvas center and flip Y to match your world coord convention
    Vec2::new(c.x - canvas_w * 0.5, (canvas_h * 0.5) - c.y)
  };

  // Draw each button's assumed area (center + corner markers)
  for (ui_global, touch_button) in &button_query {
    let button_center_canvas = ui_global.translation; // Vec2 in canvas space (top-left origin)
    let half_size = touch_button.size * 0.5;
    let min = button_center_canvas - half_size;
    let max = button_center_canvas + half_size;

    // center gizmo
    let center_world = canvas_to_world(button_center_canvas);
    gizmos.circle_2d(center_world, 18.0, Color::srgba(0.2, 0.6, 1.0, 0.35)); // bluish semi-transparent

    // corner gizmos (smaller)
    let min_world = canvas_to_world(min);
    let max_world = canvas_to_world(max);
    gizmos.circle_2d(min_world, 6.0, Color::srgba(0.2, 0.6, 1.0, 0.25));
    gizmos.circle_2d(
      Vec2::new(min_world.x, max_world.y),
      6.0,
      Color::srgba(0.2, 0.6, 1.0, 0.25),
    );
    gizmos.circle_2d(
      Vec2::new(max_world.x, min_world.y),
      6.0,
      Color::srgba(0.2, 0.6, 1.0, 0.25),
    );
    gizmos.circle_2d(max_world, 6.0, Color::srgba(0.2, 0.6, 1.0, 0.25));
  }

  // For each touch, compute canvas pos and test against button rects — draw touch gizmo green if hit, red if not.
  for touch in touches.iter() {
    if let Some(canvas_pos) = to_canvas(touch.position()) {
      let mut hit = false;

      // test against buttons using same logic as your input system
      for (ui_global, touch_button) in &button_query {
        let button_pos = ui_global.translation;
        let half_size = touch_button.size * 0.5;
        let min = button_pos - half_size;
        let max = button_pos + half_size;

        if canvas_pos.x >= min.x && canvas_pos.x <= max.x && canvas_pos.y >= min.y && canvas_pos.y <= max.y {
          hit = true;
          break;
        }
      }

      let touch_world = canvas_to_world(canvas_pos);
      if hit {
        gizmos.circle_2d(touch_world, 14.0, Color::srgba(0.0, 1.0, 0.0, 0.45)); // green
      } else {
        gizmos.circle_2d(touch_world, 14.0, Color::srgba(1.0, 0.0, 0.0, 0.45)); // red
      }
    } else {
      // touch outside canvas — show faint off-canvas marker near where it maps in window (optional)
      // convert window pos to canvas-space clamped inside canvas bounds for visualization
      let raw = touch.position();
      let clamped_x = (raw.x - offset_x) / scale;
      let clamped_y = (raw.y - offset_y) / scale;
      let clamped = Vec2::new(clamped_x.clamp(0.0, canvas_w), clamped_y.clamp(0.0, canvas_h));
      let world = canvas_to_world(clamped);
      gizmos.circle_2d(world, 10.0, Color::srgba(1.0, 0.65, 0.0, 0.25)); // orange for out-of-canvas
    }
  }
}

/// Handles touch input and map it to button interactions.
fn handle_touch_input_system(
  touches: Res<Touches>,
  mut touch_state: ResMut<TouchState>,
  mut button_query: Query<
    (
      Entity,
      &UiGlobalTransform,
      &TouchButton,
      &mut BorderColor,
      &mut BackgroundColor,
      &PlayerId,
      Option<&ButtonMovement>,
      Option<&ButtonAction>,
    ),
    With<TouchButton>,
  >,
  mut input_focus: ResMut<InputFocus>,
  mut input_action_writer: MessageWriter<InputAction>,
  window_query: Query<&Window, With<PrimaryWindow>>,
) {
  let Ok(window) = window_query.single() else { return };

  // \* same logical canvas space as the game world and UI
  let canvas_w = RESOLUTION_WIDTH as f32;
  let canvas_h = RESOLUTION_HEIGHT as f32;

  // \* scale and letterboxing used by `fit_canvas_system`
  let h_scale = window.width() / canvas_w;
  let v_scale = window.height() / canvas_h;
  let scale = h_scale.min(v_scale).round().max(1.0);
  let rendered_w = canvas_w * scale;
  let rendered_h = canvas_h * scale;
  let offset_x = (window.width() - rendered_w) * 0.5;
  let offset_y = (window.height() - rendered_h) * 0.5;

  // helper: window touch \-\> canvas space (origin top\-left, *not* centered)
  let to_canvas = |p: Vec2| -> Option<Vec2> {
    let cx = (p.x - offset_x) / scale;
    let cy = (p.y - offset_y) / scale;
    if cx < 0.0 || cx > canvas_w || cy < 0.0 || cy > canvas_h {
      None
    } else {
      Some(Vec2::new(cx, cy))
    }
  };

  // released touches
  handle_released_touches(&touches, &mut touch_state, &mut button_query, &mut input_focus);

  // new presses
  for touch in touches.iter_just_pressed() {
    let Some(canvas_pos) = to_canvas(touch.position()) else {
      continue;
    };
    let mut pressed_entity: Option<Entity> = None;

    for (
      entity,
      ui_transform,
      touch_button,
      mut border_colour,
      mut background_colour,
      _player_id,
      maybe_move,
      maybe_action,
    ) in &mut button_query
    {
      // UiGlobalTransform is in the same canvas space (origin top\-left)
      let button_pos = ui_transform.translation;
      let half_size = touch_button.size * 0.5;
      let min = button_pos - half_size;
      let max = button_pos + half_size;

      // debug!(
      //   "Testing canvas_pos={:?} against button {:?}: min={:?} max={:?}",
      //   canvas_pos, entity, min, max
      // );

      if canvas_pos.x >= min.x && canvas_pos.x <= max.x && canvas_pos.y >= min.y && canvas_pos.y <= max.y {
        pressed_entity = Some(entity);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_100));
        background_colour.0.set_alpha(BUTTON_ALPHA_PRESSED);
        input_focus.set(entity);

        if let Some(movement) = maybe_move {
          input_action_writer.write(movement.into());
        } else if let Some(action) = maybe_action {
          input_action_writer.write(action.into());
        }

        touch_state.active_touches.insert(touch.id(), entity);
        break;
      }
    }

    if pressed_entity.is_none() {
      touch_state.active_touches.remove(&touch.id());
    }
  }

  // held touches
  for touch in touches.iter() {
    let Some(canvas_position) = to_canvas(touch.position()) else {
      continue;
    };

    if let Some(&entity) = touch_state.active_touches.get(&touch.id()) {
      if let Ok((
        _,
        ui_transform,
        touch_button,
        mut border_colour,
        mut background_colour,
        _player_id,
        _maybe_move,
        _maybe_action,
      )) = button_query.get_mut(entity)
      {
        let button_pos = ui_transform.translation;
        let half_size = touch_button.size * 0.5;
        let min = button_pos - half_size;
        let max = button_pos + half_size;

        if canvas_position.x < min.x
          || canvas_position.x > max.x
          || canvas_position.y < min.y
          || canvas_position.y > max.y
        {
          *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
          background_colour.0.set_alpha(BUTTON_ALPHA_DEFAULT);
          input_focus.clear();
          touch_state.active_touches.remove(&touch.id());
        } else {
          *border_colour = BorderColor::all(Color::from(tailwind::SLATE_100));
          background_colour.0.set_alpha(BUTTON_ALPHA_PRESSED);
        }
      }
    }
  }
}

fn handle_released_touches(
  touches: &Res<Touches>,
  touch_state: &mut ResMut<TouchState>,
  button_query: &mut Query<
    (
      Entity,
      &UiGlobalTransform,
      &TouchButton,
      &mut BorderColor,
      &mut BackgroundColor,
      &PlayerId,
      Option<&ButtonMovement>,
      Option<&ButtonAction>,
    ),
    With<TouchButton>,
  >,
  input_focus: &mut ResMut<InputFocus>,
) {
  for touch in touches.iter_just_released() {
    if let Some(entity) = touch_state.active_touches.remove(&touch.id()) {
      if let Ok((
        _entity,
        _ui_global_transform,
        _touch_button,
        mut border_colour,
        mut background_colour,
        _player_id,
        _maybe_move,
        _maybe_action,
      )) = button_query.get_mut(entity)
      {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
        background_colour.0.set_alpha(BUTTON_ALPHA_DEFAULT);
        input_focus.clear();
      }
    }
  }
}

fn button_design_system(
  mut input_focus: ResMut<InputFocus>,
  mut interaction_query: Query<
    (
      Entity,
      &Interaction,
      &mut BorderColor,
      &mut BackgroundColor,
      &mut Button,
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

fn button_action_input_system(
  mut input_action_writer: MessageWriter<InputAction>,
  mut interaction_query: Query<(&Interaction, &ButtonAction), Changed<Interaction>>,
) {
  for (interaction, input_action) in &mut interaction_query {
    match *interaction {
      Interaction::Pressed => {
        input_action_writer.write(input_action.into());
      }
      _ => {}
    }
  }
}

fn button_movement_input_system(
  mut input_action_writer: MessageWriter<InputAction>,
  mut interaction_query: Query<(&Interaction, &ButtonMovement)>,
) {
  for (interaction, button_movement_action) in &mut interaction_query {
    match *interaction {
      Interaction::Pressed => {
        input_action_writer.write(button_movement_action.into());
      }
      _ => {}
    }
  }
}
