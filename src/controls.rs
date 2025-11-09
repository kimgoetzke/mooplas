use crate::constants::{MOVEMENT_SPEED, RESOLUTION_HEIGHT, RESOLUTION_WIDTH, ROTATION_SPEED, WRAPAROUND_MARGIN};
use crate::shared::{Player, WrapAroundEntity};
use avian2d::math::{AdjustPrecision, Scalar};
use avian2d::prelude::{AngularVelocity, LinearVelocity};
use bevy::app::{App, Plugin, Update};
use bevy::input::ButtonInput;
use bevy::log::debug;
use bevy::math::Vec3;
use bevy::prelude::{KeyCode, Message, MessageReader, MessageWriter, Query, Res, Time, Transform, With};

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<InputAction>()
      .add_systems(Update, (keyboard_input_system, movement_system, wraparound_system));
  }
}

/// A [`Message`] written for an input action.
#[derive(Message)]
enum InputAction {
  Move(Scalar),
  Action,
}

/// Sends [`InputAction`] events based on keyboard input.
fn keyboard_input_system(
  mut input_action_writer: MessageWriter<InputAction>,
  keyboard_input: Res<ButtonInput<KeyCode>>,
) {
  let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
  let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
  let horizontal = right as i8 - left as i8;
  let direction = horizontal as Scalar;
  if direction != 0.0 {
    input_action_writer.write(InputAction::Move(direction));
  }
  if keyboard_input.just_pressed(KeyCode::Space) {
    input_action_writer.write(InputAction::Action);
  }
}

/// Responds to [`InputAction`] events and moves character controllers accordingly.
fn movement_system(
  time: Res<Time>,
  mut input_action_messages: MessageReader<InputAction>,
  mut controllers: Query<(&Transform, &mut LinearVelocity, &mut AngularVelocity), With<Player>>,
) {
  let delta_time = time.delta_secs_f64().adjust_precision();
  for (transform, mut linear_velocity, mut angular_velocity) in &mut controllers {
    let mut has_movement_input = false;
    let direction = (transform.rotation * Vec3::Y).normalize_or_zero();
    let velocity = direction * MOVEMENT_SPEED;
    linear_velocity.x = velocity.x;
    linear_velocity.y = velocity.y;

    for event in input_action_messages.read() {
      has_movement_input = true;
      match event {
        InputAction::Move(direction) => {
          angular_velocity.0 = -*direction * ROTATION_SPEED * delta_time;
        }
        InputAction::Action => {
          debug!("[Not implemented] Action received");
        }
      }
    }
    if !has_movement_input {
      angular_velocity.0 = 0.;
    }
  }
}

/// Wraps the relevant entities around the screen edges, making them reappear on the opposite side.
fn wraparound_system(mut entities: Query<&mut Transform, With<WrapAroundEntity>>) {
  let extents = Vec3::new(RESOLUTION_WIDTH as f32 / 2., RESOLUTION_HEIGHT as f32 / 2., 0.);
  for mut transform in entities.iter_mut() {
    if transform.translation.x > (extents.x + WRAPAROUND_MARGIN) {
      transform.translation.x = -extents.x - WRAPAROUND_MARGIN;
    } else if transform.translation.x < (-extents.x - WRAPAROUND_MARGIN) {
      transform.translation.x = extents.x + WRAPAROUND_MARGIN;
    }
    if transform.translation.y > (extents.y + WRAPAROUND_MARGIN) {
      transform.translation.y = -extents.y - WRAPAROUND_MARGIN;
    } else if transform.translation.y < (-extents.y - WRAPAROUND_MARGIN) {
      transform.translation.y = extents.y + WRAPAROUND_MARGIN;
    }
  }
}
