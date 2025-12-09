use crate::app_states::AppState;
use crate::prelude::constants::{MOVEMENT_SPEED, ROTATION_SPEED};
use crate::prelude::{
  AvailablePlayerConfigs, ContinueMessage, InputMessage, NetworkRole, PlayerId, PlayerInput, RegisteredPlayers,
  Settings, SnakeHead, TouchControlsToggledMessage, has_registered_players,
};
use avian2d::math::{AdjustPrecision, Scalar};
use avian2d::prelude::{AngularVelocity, LinearVelocity};
use bevy::app::{App, Plugin, Update};
use bevy::input::ButtonInput;
use bevy::log::*;
use bevy::math::Vec3;
use bevy::prelude::{
  IntoScheduleConfigs, KeyCode, MessageReader, MessageWriter, MonitorSelection, Query, Res, ResMut, Single, Time,
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
        send_continue_message_on_key_press_system
          .run_if(in_state(AppState::Registering))
          .run_if(has_registered_players)
          .run_if(|network_role: Res<NetworkRole>| !network_role.is_client()),
      )
      .add_systems(
        Update,
        (player_input_system, player_action_system).run_if(in_state(AppState::Playing)),
      )
      .add_systems(
        Update,
        send_continue_message_on_key_press_system
          .run_if(in_state(AppState::GameOver))
          .run_if(|network_role: Res<NetworkRole>| !network_role.is_client()),
      );
  }
}

/// Handles player registration and unregistration based on keyboard input. Sends an event for the UI to update.
fn player_input_action_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut input_message: MessageWriter<InputMessage>,
) {
  for available_config in &available_configs.configs {
    if !keyboard_input.just_pressed(available_config.input.action) {
      continue;
    }

    input_message.write(InputMessage::Action(available_config.into()));
  }
}

/// Sends a [`ContinueMessage`] when the player presses one of the selected key. Can be used to start the game or
/// continue after game over.
fn send_continue_message_on_key_press_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  mut continue_message: MessageWriter<ContinueMessage>,
) {
  if keyboard_input.any_pressed([KeyCode::Space, KeyCode::Enter, KeyCode::Escape]) {
    continue_message.write(ContinueMessage);
  }
}

/// Sends [`InputMessage`] events based on keyboard input, but only for registered players.
fn player_input_system(
  mut input_message: MessageWriter<InputMessage>,
  keyboard_input: Res<ButtonInput<KeyCode>>,
  registered_players: Option<Res<RegisteredPlayers>>,
) {
  let Some(registered) = registered_players else {
    return;
  };
  for player in &registered.players {
    if player.is_remote() {
      continue;
    }
    process_inputs(&mut input_message, &keyboard_input, player.input.clone());
  }
}

fn process_inputs(
  input_message: &mut MessageWriter<InputMessage>,
  keyboard_input: &Res<ButtonInput<KeyCode>>,
  player_input: PlayerInput,
) {
  let left = keyboard_input.any_pressed([player_input.left]);
  let right = keyboard_input.any_pressed([player_input.right]);
  let horizontal_p1 = right as i8 - left as i8;
  let direction = horizontal_p1 as Scalar;
  if direction != 0.0 {
    input_message.write(InputMessage::Move(player_input.id, direction));
  }
  if keyboard_input.just_pressed(player_input.action) {
    input_message.write(InputMessage::Action(player_input.id));
  }
}

/// Responds to [`InputMessage`] events and moves character controllers accordingly. Skips remote (immutable)
/// players because they are controlled by server state updates.
fn player_action_system(
  time: Res<Time>,
  mut input_messages: MessageReader<InputMessage>,
  mut snake_head_query: Query<(&Transform, &mut LinearVelocity, &mut AngularVelocity, &PlayerId), With<SnakeHead>>,
  registered_players: Option<Res<RegisteredPlayers>>,
  network_role: Res<NetworkRole>,
) {
  let delta_time = time.delta_secs_f64().adjust_precision();
  let messages: Vec<&InputMessage> = input_messages.read().collect();

  for (transform, mut linear_velocity, mut angular_velocity, player_id) in &mut snake_head_query {
    // Skip remote (immutable) players on clients
    if network_role.is_client()
      && registered_players
        .as_ref()
        .and_then(|players| players.players.iter().find(|p| p.id == *player_id))
        .map_or(false, |player| player.is_remote())
    {
      continue;
    }

    let mut has_movement_input = false;
    let direction = (transform.rotation * Vec3::Y).normalize_or_zero();
    let velocity = direction * MOVEMENT_SPEED;
    linear_velocity.x = velocity.x;
    linear_velocity.y = velocity.y;

    for message in messages.iter() {
      match message {
        InputMessage::Move(id, direction) if id == player_id => {
          has_movement_input = true;
          angular_velocity.0 = -*direction * ROTATION_SPEED * delta_time;
        }
        InputMessage::Action(pid) if pid == player_id => {
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
  mut window: Single<&mut Window>,
  mut touch_controls_message: MessageWriter<TouchControlsToggledMessage>,
) {
  if keyboard_input.just_pressed(KeyCode::F11) {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app_states::AppStatePlugin;
  use crate::prelude::{
    AvailablePlayerConfig, PlayerId, RegisteredPlayer, SharedMessagesPlugin, SharedResourcesPlugin,
  };
  use bevy::MinimalPlugins;
  use bevy::prelude::Color;
  use bevy::prelude::{Messages, Mut, NextState, State};
  use bevy::state::app::StatesPlugin;

  #[allow(unused)]
  enum TestKeyboardInput {
    Press(KeyCode),
    Release(KeyCode),
  }

  fn setup() -> App {
    let mut app = App::new();
    app
      .add_plugins((
        MinimalPlugins,
        ControlsPlugin,
        StatesPlugin,
        AppStatePlugin,
        SharedMessagesPlugin,
        SharedResourcesPlugin,
      ))
      .init_resource::<ButtonInput<KeyCode>>();
    app
  }

  fn handle_key_input(app: &mut App, desired_input: TestKeyboardInput) {
    let mut keyboard_input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    match desired_input {
      TestKeyboardInput::Press(key_code) => keyboard_input.press(key_code),
      TestKeyboardInput::Release(key_code) => keyboard_input.release(key_code),
    };
    app.update();
  }

  fn change_app_state(app: &mut App, state: AppState) {
    let mut next_state = app.world_mut().resource_mut::<NextState<AppState>>();
    next_state.set(state);
    app.update();
  }

  #[test]
  fn shared_messages_plugin_does_not_panic_on_empty_app() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ControlsPlugin);
  }

  #[test]
  fn player_input_action_system_sends_input_message() {
    let mut app = setup();

    // Prepare an available player config that reacts to KeyX
    let mut available_configs = app
      .world_mut()
      .get_resource_mut::<AvailablePlayerConfigs>()
      .expect("AvailablePlayerConfigs resource missing");
    available_configs.configs.push(AvailablePlayerConfig {
      id: PlayerId(0),
      input: PlayerInput::new(PlayerId(0), KeyCode::KeyZ, KeyCode::KeyC, KeyCode::KeyX),
      colour: Color::WHITE,
    });
    drop(available_configs);

    // Verify initial state
    let state = app.world().resource::<State<AppState>>();
    assert_eq!(state.get(), &AppState::Loading);

    // Manually advance state to the state in which the function runs
    change_app_state(&mut app, AppState::Registering);

    // Verify state has been advanced since this system only runs in Registering state
    let state = app.world().resource::<State<AppState>>();
    assert_eq!(state.get(), &AppState::Registering);

    // Simulate pressing the action key
    handle_key_input(&mut app, TestKeyboardInput::Press(KeyCode::KeyX));

    // Read produced messages
    let messages = app
      .world_mut()
      .get_resource_mut::<Messages<InputMessage>>()
      .expect("Messages<InputAction> missing");

    // Ensure at least one message was written
    let has_input_action = messages
      .iter_current_update_messages()
      .any(|ia| matches!(ia, InputMessage::Action(_)));
    assert!(has_input_action, "Expected an Action InputAction to be sent");
  }

  #[test]
  fn send_continue_message_on_key_press_system_sends_continue_message() {
    let mut app = setup();

    // Verify initial state
    let state = app.world().resource::<State<AppState>>();
    assert_eq!(state.get(), &AppState::Loading);

    // Manually advance state to the state in which the system runs
    change_app_state(&mut app, AppState::Registering);

    // Verify state has been advanced since this system only runs in this state
    let state = app.world().resource::<State<AppState>>();
    assert_eq!(state.get(), &AppState::Registering);

    // Register player to fulfill has_registered_players condition
    let mut registered_players = app
      .world_mut()
      .get_resource_mut::<RegisteredPlayers>()
      .expect("RegisteredPlayers resource missing");
    registered_players.players.push(RegisteredPlayer::new_immutable(
      PlayerId(0),
      PlayerInput::new(PlayerId(0), KeyCode::KeyZ, KeyCode::KeyC, KeyCode::KeyX),
      Color::WHITE,
    ));
    drop(registered_players);

    // Simulate pressing Space which should trigger a ContinueMessage
    handle_key_input(&mut app, TestKeyboardInput::Press(KeyCode::Space));

    // Ensure at least one continue message was written
    let mut messages: Mut<Messages<ContinueMessage>> = app
      .world_mut()
      .get_resource_mut::<Messages<ContinueMessage>>()
      .expect("Messages<ContinueMessage> missing");
    assert!(
      messages.drain().next().is_some(),
      "Expected a ContinueMessage to be sent"
    );
  }

  #[test]
  fn player_input_system_sends_move_and_action_messages() {
    let mut app = setup();
    let player_input = PlayerInput::new(PlayerId(0), KeyCode::KeyZ, KeyCode::KeyC, KeyCode::KeyX);

    // Register a player for the input system to process
    let mut registered_players = app
      .world_mut()
      .get_resource_mut::<RegisteredPlayers>()
      .expect("RegisteredPlayers resource missing");
    registered_players.players.push(RegisteredPlayer::new_mutable(
      player_input.id,
      player_input.clone(),
      Color::WHITE,
    ));
    drop(registered_players);

    // Manually advance state to the state in which the system runs
    change_app_state(&mut app, AppState::Playing);

    // Verify state has been advanced since this system only runs in this state
    let state = app.world().resource::<State<AppState>>();
    assert_eq!(state.get(), &AppState::Playing);

    // Simulate pressing left, right and action
    handle_key_input(&mut app, TestKeyboardInput::Press(player_input.left));
    handle_key_input(&mut app, TestKeyboardInput::Press(player_input.right));
    handle_key_input(&mut app, TestKeyboardInput::Press(player_input.action));

    // Read produced messages
    let messages = app
      .world_mut()
      .get_resource_mut::<Messages<InputMessage>>()
      .expect("Messages<InputAction> missing");

    // Verify that both move and action messages were sent
    let has_action = messages
      .iter_current_update_messages()
      .any(|ia| matches!(ia, InputMessage::Action(_)));
    let has_move = messages
      .iter_current_update_messages()
      .any(|ia| matches!(ia, InputMessage::Move(_, _)));
    assert!(has_action, "Expected an Action InputAction to be sent");
    assert!(has_move, "Expected a Move InputAction to be sent");
  }

  #[test]
  fn settings_controls_system_toggles_touch_controls() {
    let mut app = setup();

    // Make sure we have a window, so that the system runs
    app.world_mut().spawn(Window::default());

    // Ensure default settings are as expected
    let settings = app.world().get_resource::<Settings>().expect("Settings missing");
    assert!(
      !settings.general.enable_touch_controls,
      "Expected touch controls to be disabled by default"
    );

    // Press F10 to toggle touch controls
    handle_key_input(&mut app, TestKeyboardInput::Press(KeyCode::F10));

    // Verify that settings have changed
    let settings = app.world().get_resource::<Settings>().expect("Settings missing");
    assert!(
      settings.general.enable_touch_controls,
      "Expected touch controls to be enabled after pressing F10"
    );

    // Verify that a toggle message for touch controls was sent
    let touch_messages = app
      .world()
      .get_resource::<Messages<TouchControlsToggledMessage>>()
      .expect("Messages<TouchControlsToggledMessage> missing");
    assert!(
      touch_messages.iter_current_update_messages().next().is_some(),
      "Expected a TouchControlsToggledMessage to be sent"
    );
  }
}
