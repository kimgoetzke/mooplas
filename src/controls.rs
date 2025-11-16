use crate::app_states::AppState;
use crate::constants::{MOVEMENT_SPEED, ROTATION_SPEED};
use crate::shared::{DebugStateMessage, GeneralSettings, PlayerId, Settings, SnakeHead};
use avian2d::math::{AdjustPrecision, Scalar};
use avian2d::prelude::{AngularVelocity, LinearVelocity};
use bevy::app::{App, Plugin, Update};
use bevy::input::ButtonInput;
use bevy::log::*;
use bevy::math::Vec3;
use bevy::prelude::{
  IntoScheduleConfigs, KeyCode, Message, MessageReader, MessageWriter, MonitorSelection, NextState, Query, Res, ResMut,
  Time, Transform, Window, With, in_state,
};

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<InputAction>()
      .add_systems(Update, settings_controls_system)
      .add_systems(Update, start_game_system.run_if(in_state(AppState::Waiting)))
      .add_systems(
        Update,
        (player_input_system, player_action_system).run_if(in_state(AppState::Running)),
      );
  }
}

/// A [`Message`] written for an input action.
#[derive(Message)]
enum InputAction {
  Move(PlayerId, Scalar),
  Action(PlayerId),
}

/// Defines the key bindings for a given player.
struct PlayerInput {
  id: PlayerId,
  left: KeyCode,
  right: KeyCode,
  action: KeyCode,
}

impl PlayerInput {
  fn new(id: PlayerId, left: KeyCode, right: KeyCode, action: KeyCode) -> Self {
    Self {
      id,
      left,
      right,
      action,
    }
  }
}

/// Transitions the game from the loading state to the running state.
fn start_game_system(keyboard_input: Res<ButtonInput<KeyCode>>, mut next_app_state: ResMut<NextState<AppState>>) {
  if keyboard_input.any_pressed([
    KeyCode::Space,
    KeyCode::Enter,
    KeyCode::Escape,
    KeyCode::KeyA,
    KeyCode::KeyW,
    KeyCode::KeyS,
    KeyCode::KeyD,
  ]) {
    debug_once!("Waiting for keyboard input to start the game...");
  } else {
    return;
  }
  next_app_state.set(AppState::Running);
}

// TODO: Move player input to resource and declare once on startup
/// Sends [`InputAction`] events based on keyboard input.
fn player_input_system(mut input_action_writer: MessageWriter<InputAction>, keyboard_input: Res<ButtonInput<KeyCode>>) {
  for player_input in [
    PlayerInput::new(PlayerId(0), KeyCode::KeyA, KeyCode::KeyD, KeyCode::Space),
    PlayerInput::new(
      PlayerId(1),
      KeyCode::ArrowLeft,
      KeyCode::ArrowRight,
      KeyCode::ShiftRight,
    ),
  ] {
    process_inputs(&mut input_action_writer, &keyboard_input, player_input);
  }
}

fn process_inputs(
  input_action_writer: &mut MessageWriter<InputAction>,
  keyboard_input: &Res<ButtonInput<KeyCode>>,
  player_input: PlayerInput,
) {
  let left = keyboard_input.any_pressed([player_input.left]);
  let right = keyboard_input.any_pressed([player_input.right]);
  let horizontal_p1 = right as i8 - left as i8;
  let direction = horizontal_p1 as Scalar;
  if direction != 0.0 {
    input_action_writer.write(InputAction::Move(player_input.id, direction));
  }
  if keyboard_input.just_pressed(player_input.action) {
    input_action_writer.write(InputAction::Action(player_input.id));
  }
}

/// Responds to [`InputAction`] events and moves character controllers accordingly.
fn player_action_system(
  time: Res<Time>,
  mut input_action_messages: MessageReader<InputAction>,
  mut controllers: Query<(&Transform, &mut LinearVelocity, &mut AngularVelocity, &PlayerId), With<SnakeHead>>,
) {
  let delta_time = time.delta_secs_f64().adjust_precision();
  let messages: Vec<&InputAction> = input_action_messages.read().collect();

  for (transform, mut linear_velocity, mut angular_velocity, player_id) in &mut controllers {
    let mut has_movement_input = false;
    let direction = (transform.rotation * Vec3::Y).normalize_or_zero();
    let velocity = direction * MOVEMENT_SPEED;
    linear_velocity.x = velocity.x;
    linear_velocity.y = velocity.y;

    for event in messages.iter() {
      match event {
        InputAction::Move(id, direction) if id == player_id => {
          has_movement_input = true;
          angular_velocity.0 = -*direction * ROTATION_SPEED * delta_time;
        }
        InputAction::Action(pid) if pid == player_id => {
          debug!("[Not implemented] Action received for: {:?}", player_id);
        }
        _ => {}
      }
    }

    if !has_movement_input {
      angular_velocity.0 = 0.;
    }
  }
}

fn settings_controls_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  mut settings: ResMut<Settings>,
  mut general_settings: ResMut<GeneralSettings>,
  mut debug_state_message: MessageWriter<DebugStateMessage>,
  mut windows: Query<&mut Window>,
) {
  if keyboard_input.just_pressed(KeyCode::F9) {
    settings.general.display_player_gizmos = !settings.general.display_player_gizmos;
    general_settings.display_player_gizmos = settings.general.display_player_gizmos;
    info!(
      "[F9] Set display player gizmos to [{}]",
      settings.general.display_player_gizmos
    );
    debug_state_message.write(DebugStateMessage {
      display_player_gizmos: settings.general.display_player_gizmos,
    });
  }
  if keyboard_input.just_pressed(KeyCode::F11) {
    let mut window = windows.single_mut().expect("Failed to get primary window");
    window.mode = match window.mode {
      bevy::window::WindowMode::Windowed => bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current),
      _ => bevy::window::WindowMode::Windowed,
    };
    info!("[F11] Set window mode to [{:?}]", window.mode);
  }
}
