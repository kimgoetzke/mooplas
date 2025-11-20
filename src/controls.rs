use crate::app_states::AppState;
use crate::prelude::constants::{MOVEMENT_SPEED, ROTATION_SPEED};
use crate::prelude::{
  AvailablePlayerConfigs, InputAction, PlayerId, PlayerInput, RegisteredPlayers, Settings, SnakeHead,
  TouchControlsToggledMessage,
};
use avian2d::math::{AdjustPrecision, Scalar};
use avian2d::prelude::{AngularVelocity, LinearVelocity};
use bevy::app::{App, Plugin, Update};
use bevy::input::ButtonInput;
use bevy::log::*;
use bevy::math::Vec3;
use bevy::prelude::{
  IntoScheduleConfigs, KeyCode, MessageReader, MessageWriter, MonitorSelection, NextState, Query, Res, ResMut, Time,
  Transform, Window, With, in_state,
};

/// A plugin that manages all player controls and input handling.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Update, settings_controls_system)
      .add_systems(
        Update,
        player_input_action_system.run_if(in_state(AppState::Registering)),
      )
      .add_systems(
        Update,
        start_game_system
          .run_if(in_state(AppState::Registering))
          .run_if(has_registered_players),
      )
      .add_systems(
        Update,
        (player_input_system, player_action_system).run_if(in_state(AppState::Playing)),
      )
      .add_systems(
        Update,
        game_over_to_reinitialising_transition_system.run_if(in_state(AppState::GameOver)),
      );
  }
}

/// Handles player registration and unregistration based on keyboard input. Sends an event for the UI to update.
fn player_input_action_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut input_action_message: MessageWriter<InputAction>,
) {
  for available_config in &available_configs.configs {
    if !keyboard_input.just_pressed(available_config.input.action) {
      continue;
    }

    input_action_message.write(InputAction::Action(available_config.into()));
  }
}

/// Transitions the game from the registration/lobby state to the running state.
fn start_game_system(keyboard_input: Res<ButtonInput<KeyCode>>, mut next_app_state: ResMut<NextState<AppState>>) {
  if keyboard_input.any_pressed([KeyCode::Space, KeyCode::Enter, KeyCode::Escape]) {
    debug_once!("Waiting for keyboard input to start the game...");
  } else {
    return;
  }
  next_app_state.set(AppState::Playing);
}

fn has_registered_players(registered: Option<Res<RegisteredPlayers>>) -> bool {
  if let Some(registered) = registered {
    !registered.players.is_empty()
  } else {
    false
  }
}

// Sends `InputAction` events based on keyboard input, but only for registered players.
fn player_input_system(
  mut input_action_writer: MessageWriter<InputAction>,
  keyboard_input: Res<ButtonInput<KeyCode>>,
  registered: Option<Res<RegisteredPlayers>>,
) {
  let Some(registered) = registered else {
    return;
  };
  for player in &registered.players {
    process_inputs(&mut input_action_writer, &keyboard_input, player.input.clone());
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

/// A system that handles various settings-related controls, such as toggling fullscreen mode.
fn settings_controls_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  mut settings: ResMut<Settings>,
  mut windows: Query<&mut Window>,
  mut touch_controls_message: MessageWriter<TouchControlsToggledMessage>,
) {
  if keyboard_input.just_pressed(KeyCode::F11) {
    let mut window = windows.single_mut().expect("Failed to get primary window");
    window.mode = match window.mode {
      bevy::window::WindowMode::Windowed => bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current),
      _ => bevy::window::WindowMode::Windowed,
    };
    info!("[F11] Set window mode to [{:?}]", window.mode);
  }
  if keyboard_input.just_pressed(KeyCode::F10) {
    settings.general.enable_touch_controls = !settings.general.enable_touch_controls;
    touch_controls_message.write(TouchControlsToggledMessage::new(settings.general.enable_touch_controls));
    info!(
      "[F10] Set touch controls to [{:?}]",
      settings.general.enable_touch_controls
    );
  }
}

fn game_over_to_reinitialising_transition_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  mut next_state: ResMut<NextState<AppState>>,
) {
  if keyboard_input.just_pressed(KeyCode::Space) {
    next_state.set(AppState::Initialising);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  // use crate::app_states::AppStatePlugin;
  // use crate::prelude::SharedResourcesPlugin;
  // use crate::shared::SharedMessagesPlugin;
  // use bevy::state::app::StatesPlugin;
  use bevy::MinimalPlugins;

  // enum TestKeyboardInput {
  //   Press(KeyCode),
  //   Release(KeyCode),
  // }
  //
  // fn setup() -> App {
  //   let mut app = App::new();
  //   app
  //     .add_plugins((
  //       MinimalPlugins,
  //       ControlsPlugin,
  //       StatesPlugin,
  //       AppStatePlugin,
  //       SharedMessagesPlugin,
  //       SharedResourcesPlugin,
  //     ))
  //     .init_resource::<ButtonInput<KeyCode>>();
  //   app
  // }

  #[test]
  fn shared_messages_plugin_does_not_panic_on_empty_app() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ControlsPlugin);
  }

  // TODO: Fix this test or remove it; cannot advance without registering a player
  // #[test]
  // fn start_game_system_changes_app_state() {
  //   let mut app = setup();
  //
  //   // Verify initial state
  //   let state = app.world().resource::<State<AppState>>();
  //   assert_eq!(state.get(), &AppState::Initialising);
  //
  //   // Manually advance state to the state in which the function runs
  //   change_app_state(&mut app);
  //
  //   // Verify state has been advanced
  //   let state = app.world().resource::<State<AppState>>();
  //   assert_eq!(state.get(), &AppState::Registering);
  //
  //   // Simulate space key press to start the game
  //   handle_key_input(&mut app, TestKeyboardInput::Press(KeyCode::Space));
  //   handle_key_input(&mut app, TestKeyboardInput::Release(KeyCode::Space));
  //
  //   // Verify state has changed by the system
  //   let state = app.world().resource::<State<AppState>>();
  //   assert_eq!(state.get(), &AppState::Registering);
  // }
  //
  // fn change_app_state(app: &mut App) {
  //   let mut next_state = app.world_mut().resource_mut::<NextState<AppState>>();
  //   next_state.set(AppState::Registering);
  //   app.update();
  // }
  //
  // fn handle_key_input(app: &mut App, desired_input: TestKeyboardInput) {
  //   let mut keyboard_input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
  //   match desired_input {
  //     TestKeyboardInput::Press(key_code) => keyboard_input.press(key_code),
  //     TestKeyboardInput::Release(key_code) => keyboard_input.release(key_code),
  //   };
  //   app.update();
  // }
}
