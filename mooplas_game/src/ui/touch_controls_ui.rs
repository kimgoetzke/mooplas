use crate::app_state::AppState;
use crate::prelude::constants::*;
use crate::prelude::{
  AvailableControlSchemes, ControlScheme, ControlSchemeId, CustomInteraction, PlayerId, Settings, TouchControlButton,
  TouchControlsToggledMessage, colour_for_player_id,
};
use crate::shared::{InputMessage, PlayerRegistrationMessage, RegisteredPlayers};
use crate::ui::ui::{
  set_interaction_on_cancel, set_interaction_on_hover, set_interaction_on_hover_exit, set_interaction_on_press,
  set_interaction_on_release,
};
use avian2d::math::Scalar;
use bevy::color::palettes::tailwind;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use mooplas_networking::prelude::NetworkRole;
use std::fmt::Debug;

/// A plugin that sets up the touch controls UI and related systems, so that players can use touch input to control
/// their characters.
pub struct TouchControlsUiPlugin;

impl Plugin for TouchControlsUiPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<ActiveMovementTracker>()
      .add_systems(OnEnter(AppState::Registering), spawn_touch_controls_system)
      .add_systems(
        OnExit(AppState::Registering),
        despawn_unregistered_player_controls_system,
      )
      .add_systems(
        Update,
        player_movement_input_action_emitter_system.run_if(in_state(AppState::Playing)),
      )
      .add_systems(
        Update,
        handle_touch_controls_player_registration_message.run_if(in_state(AppState::Registering)),
      )
      .add_systems(Update, handle_toggle_touch_controls_message);
  }
}

const TOUCH_CONTROL_WIDTH: f32 = 60.;
const TOUCH_CONTROL_HEIGHT: f32 = 60.;
const MARGIN: f32 = 15.;
const VERTICAL_TOUCH_CONTROL_OFFSET: i32 = -25;

/// A marker component for the root entity of the touch controls UI. Used for despawning. All other UI components must
/// be children of this.
#[derive(Component)]
struct TouchControlsUiRoot;

/// A marker component for the touch controls container for a specific player.
#[derive(Component)]
struct PlayerTouchControls {
  control_scheme_id: ControlSchemeId,
}

/// A component for touch control buttons identifying their function. Used to map button interactions to input actions.
#[derive(Component, Clone, Copy, Debug)]
enum TouchControl {
  Movement(PlayerId, Scalar),
  Action(PlayerId),
}

impl Into<InputMessage> for &TouchControl {
  fn into(self) -> InputMessage {
    match self {
      TouchControl::Movement(player_id, direction) => InputMessage::Move(*player_id, *direction),
      TouchControl::Action(player_id) => InputMessage::Action(*player_id),
    }
  }
}

impl TouchControl {
  fn get_player_id(&self) -> PlayerId {
    match self {
      TouchControl::Movement(player_id, _) => *player_id,
      TouchControl::Action(player_id) => *player_id,
    }
  }
}

// Resource that tracks currently active movements per player.
#[derive(Resource, Default)]
struct ActiveMovementTracker {
  players: HashMap<PlayerId, (Entity, TouchControl)>,
}

/// A system that spawns the touch controls UI if enabled in settings. Intended to be called on startup.
fn spawn_touch_controls_system(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  settings: Res<Settings>,
  available_control_schemes: Res<AvailableControlSchemes>,
  registered_players: Res<RegisteredPlayers>,
  network_role: Res<NetworkRole>,
) {
  if !settings.general.enable_touch_controls {
    return;
  }

  spawn_touch_controls_ui(
    &mut commands,
    &available_control_schemes,
    &asset_server,
    &registered_players,
    &network_role,
  );
}

/// Spawns the touch controls UI.
fn spawn_touch_controls_ui(
  commands: &mut Commands,
  available_control_schemes: &AvailableControlSchemes,
  _asset_server: &Res<AssetServer>,
  registered_players: &RegisteredPlayers,
  network_role: &NetworkRole,
) {
  let parent = commands
    .spawn((
      TouchControlsUiRoot,
      Node {
        width: percent(100),
        height: percent(100),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      ZIndex(100),
      Pickable::IGNORE,
    ))
    .id();

  for control_scheme in available_control_schemes.schemes.iter() {
    let (player_id, colour) = touch_controls_player_state(control_scheme.id, registered_players, network_role);
    commands.entity(parent).with_children(|parent| {
      parent
        .spawn((
          Name::new("Controls for Player ".to_string() + &player_id.to_string()),
          controller_positioning_node(control_scheme),
          PlayerTouchControls {
            control_scheme_id: control_scheme.id,
          },
          player_id,
        ))
        .with_children(|parent| {
          parent
            .spawn((
              // Left movement button
              touch_control_button(
                None,
                BorderRadius {
                  top_left: percent(50),
                  bottom_left: percent(50),
                  top_right: percent(20),
                  bottom_right: percent(20),
                },
              ),
              TouchControl::Movement(player_id, -1.0),
            ))
            .observe(set_interaction_on_hover)
            .observe(set_interaction_on_hover_exit)
            .observe(set_interaction_on_press)
            .observe(set_interaction_on_release)
            .observe(set_interaction_on_cancel)
            .observe(stop_player_movement_on_release)
            .observe(start_movement_on_hover_over)
            .observe(stop_player_movement_on_hover_out);

          parent
            .spawn((
              // Player action button
              touch_control_button(Some(colour), BorderRadius::all(percent(20))),
              TouchControl::Action(player_id),
            ))
            .observe(set_interaction_on_hover)
            .observe(set_interaction_on_hover_exit)
            .observe(set_interaction_on_press)
            .observe(set_interaction_on_release)
            .observe(set_interaction_on_cancel)
            .observe(tap_player_action);

          parent
            .spawn((
              // Right movement button
              touch_control_button(
                None,
                BorderRadius {
                  top_left: percent(20),
                  bottom_left: percent(20),
                  top_right: percent(50),
                  bottom_right: percent(50),
                },
              ),
              TouchControl::Movement(player_id, 1.0),
            ))
            .observe(set_interaction_on_hover)
            .observe(set_interaction_on_hover_exit)
            .observe(set_interaction_on_press)
            .observe(set_interaction_on_release)
            .observe(set_interaction_on_cancel)
            .observe(stop_player_movement_on_release)
            .observe(start_movement_on_hover_over)
            .observe(stop_player_movement_on_hover_out);
        });
    });
  }
}

fn handle_touch_controls_player_registration_message(
  mut messages: MessageReader<PlayerRegistrationMessage>,
  registered_players: Res<RegisteredPlayers>,
  network_role: Res<NetworkRole>,
  mut tracker: ResMut<ActiveMovementTracker>,
  mut player_touch_controls_query: Query<(&PlayerTouchControls, &mut PlayerId, &Children)>,
  mut touch_control_query: Query<(&mut TouchControl, &mut BackgroundColor), With<TouchControlButton>>,
) {
  for message in messages.read() {
    let Some(control_scheme_id) = message.control_scheme_id else {
      continue;
    };
    let (player_id, action_colour) = touch_controls_player_state(control_scheme_id, &registered_players, &network_role);

    for (player_touch_controls, mut current_player_id, children) in &mut player_touch_controls_query {
      if player_touch_controls.control_scheme_id != control_scheme_id {
        continue;
      }

      *current_player_id = player_id;

      for child in children.iter() {
        let Ok((mut touch_control, mut background_colour)) = touch_control_query.get_mut(child) else {
          continue;
        };

        let rebound_touch_control = match *touch_control {
          TouchControl::Movement(_, direction) => TouchControl::Movement(player_id, direction),
          TouchControl::Action(_) => {
            *background_colour = BackgroundColor(action_colour.with_alpha(BUTTON_ALPHA_DEFAULT));
            TouchControl::Action(player_id)
          }
        };

        *touch_control = rebound_touch_control;
        rebind_active_touch_control(&mut tracker, child, rebound_touch_control);
      }
    }
  }
}

fn touch_controls_player_state(
  control_scheme_id: ControlSchemeId,
  registered_players: &RegisteredPlayers,
  network_role: &NetworkRole,
) -> (PlayerId, Color) {
  let slot_player_id = PlayerId(control_scheme_id.0);
  if network_role.is_none() {
    return (slot_player_id, colour_for_player_id(slot_player_id));
  }
  if let Some(player_id) = registered_players.get_local_player_id_for_control_scheme(control_scheme_id) {
    return (player_id, colour_for_player_id(player_id));
  }

  (slot_player_id, colour_for_player_id(slot_player_id))
}

fn rebind_active_touch_control(tracker: &mut ActiveMovementTracker, entity: Entity, touch_control: TouchControl) {
  let previous_player_id = tracker
    .players
    .iter()
    .find_map(|(player_id, (tracked_entity, _))| (*tracked_entity == entity).then_some(*player_id));
  if let Some(previous_player_id) = previous_player_id {
    tracker.players.remove(&previous_player_id);
    tracker
      .players
      .insert(touch_control.get_player_id(), (entity, touch_control));
  }
}

fn tap_player_action(
  click: On<Pointer<Click>>,
  mut touch_control_query: Query<Option<&TouchControl>, With<TouchControlButton>>,
  mut input_message: MessageWriter<InputMessage>,
  current_app_state: Res<State<AppState>>,
) {
  if let Ok(touch_control_action) = touch_control_query.get_mut(click.entity) {
    if let Some(action) = touch_control_action {
      if *current_app_state != AppState::Registering {
        warn!(
          "Touch input [{:?}] ignored while not in [{}] state...",
          action,
          AppState::Registering
        );
        return;
      }
      input_message.write(action.into());
    }
  }
}

/// Starts movement for a player when they hover over a movement button. This is to support clicking just outside the
/// button and then moving your finger onto the button to start movement. Since this is about touch, we don't care that
/// the user isn't technically "pressing" the button yet.
fn start_movement_on_hover_over(
  action: On<Pointer<Over>>,
  mut tracker: ResMut<ActiveMovementTracker>,
  touch_control_query: Query<&TouchControl>,
  mut input_message: MessageWriter<InputMessage>,
) {
  start_player_movement(action, &mut tracker, touch_control_query, &mut input_message);
}

fn start_player_movement<T: 'static + Clone + Debug + Reflect>(
  action: On<Pointer<T>>,
  tracker: &mut ResMut<ActiveMovementTracker>,
  touch_control_query: Query<&TouchControl>,
  input_message: &mut MessageWriter<InputMessage>,
) {
  if let Ok(touch_control) = touch_control_query.get(action.entity) {
    tracker
      .players
      .insert(touch_control.get_player_id(), (action.entity, *touch_control));
    input_message.write(touch_control.into());
  }
}

/// Stops movement for a player when they release a movement button.
fn stop_player_movement_on_release(action: On<Pointer<Release>>, mut tracker: ResMut<ActiveMovementTracker>) {
  remove_player_from_movement_tracker(action, &mut tracker);
}

/// Stops movement for a player when they move their pointer/finger outside the button bounds.
fn stop_player_movement_on_hover_out(action: On<Pointer<Out>>, mut tracker: ResMut<ActiveMovementTracker>) {
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

/// A system that despawns any player touch controls for players that have not registered for the current session.
fn despawn_unregistered_player_controls_system(
  mut commands: Commands,
  player_touch_controls_query: Query<(Entity, &PlayerId), With<PlayerTouchControls>>,
  registered_players: Res<RegisteredPlayers>,
) {
  for (entity, player_id) in player_touch_controls_query.iter() {
    if !registered_players.players.iter().any(|p| p.id == *player_id) {
      commands.entity(entity).despawn();
    }
  }
}

/// Per-frame emitter system for [`InputMessage::Move`] for every active movement of every player. Reads the current
/// active movements from the [`ActiveMovementTracker`] resource and emits corresponding input actions.
fn player_movement_input_action_emitter_system(
  tracker: Res<ActiveMovementTracker>,
  mut input_message: MessageWriter<InputMessage>,
) {
  if tracker.players.is_empty() {
    return;
  }
  for (_player, (_entity, touch_control)) in tracker.players.iter() {
    match touch_control {
      TouchControl::Movement(_, _) => input_message.write(touch_control.into()),
      _ => {
        warn!(
          "Unexpected touch control in movement tracker for touch controls: {:?}",
          touch_control
        );
        continue;
      }
    };
  }
}

/// The node that positions the touch controls for a given player on screen based on their control scheme ID.
fn controller_positioning_node(control_scheme: &ControlScheme) -> (Node, UiTransform) {
  const HORIZONTAL_OFFSET: f32 = -((((TOUCH_CONTROL_WIDTH + ((MARGIN + BUTTON_BORDER_WIDTH) * 2.0)) * 3.) / 2.) + 4.);
  const VERTICAL_OFFSET: f32 = -((TOUCH_CONTROL_HEIGHT / 3.) + (MARGIN + BUTTON_BORDER_WIDTH) * 2.);

  match control_scheme.id.0 {
    0 | 1 => (
      // Bottom row (players 1 and 2)
      Node {
        position_type: PositionType::Absolute,
        bottom: px(10),
        left: percent(33 + control_scheme.id.0 * 33),
        margin: UiRect::all(px(10)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      UiTransform {
        translation: Val2::new(px(HORIZONTAL_OFFSET), Val::Auto),
        ..default()
      },
    ),
    2 => (
      // Right side (player 3)
      Node {
        position_type: PositionType::Absolute,
        top: percent(50),
        right: px(VERTICAL_TOUCH_CONTROL_OFFSET),
        margin: UiRect::all(px(10)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      UiTransform {
        translation: Val2::new(px(TOUCH_CONTROL_HEIGHT), px(VERTICAL_OFFSET)),
        rotation: Rot2::degrees(-90.),
        ..default()
      },
    ),
    3 => (
      // Top center (player 4)
      Node {
        position_type: PositionType::Absolute,
        top: px(10),
        left: percent(50),
        margin: UiRect::all(px(10)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      UiTransform {
        translation: Val2::new(px(HORIZONTAL_OFFSET), Val::Auto),
        rotation: Rot2::degrees(180.),
        ..default()
      },
    ),
    4 => (
      // Left side (player 5)
      Node {
        position_type: PositionType::Absolute,
        top: percent(50),
        left: px(VERTICAL_TOUCH_CONTROL_OFFSET),
        margin: UiRect::all(px(10)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
      },
      UiTransform {
        translation: Val2::new(px(-TOUCH_CONTROL_HEIGHT), px(VERTICAL_OFFSET)),
        rotation: Rot2::degrees(-270.),
        ..default()
      },
    ),
    _ => panic!(
      "Unsupported control scheme ID for touch controls UI: {}",
      control_scheme.id.0
    ),
  }
}

/// A bundle for a button that is involved in controlling a player.
fn touch_control_button(custom_colour: Option<Color>, border_radius: BorderRadius) -> impl Bundle {
  (
    Node {
      width: px(TOUCH_CONTROL_WIDTH),
      height: px(TOUCH_CONTROL_HEIGHT),
      border: UiRect::all(px(BUTTON_BORDER_WIDTH)),
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      margin: UiRect::all(px(MARGIN)),
      border_radius,
      ..default()
    },
    TouchControlButton,
    CustomInteraction::default(),
    BorderColor::all(Color::from(tailwind::SLATE_500)),
    if let Some(colour) = custom_colour {
      BackgroundColor(Color::from(colour).with_alpha(BUTTON_ALPHA_DEFAULT))
    } else {
      BackgroundColor(Color::from(tailwind::SLATE_600).with_alpha(BUTTON_ALPHA_DEFAULT))
    },
  )
}

/// A system that handles toggling the touch controls UI via messages.
fn handle_toggle_touch_controls_message(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut messages: MessageReader<TouchControlsToggledMessage>,
  mut touch_controls_ui_query: Query<Entity, With<TouchControlsUiRoot>>,
  available_control_schemes: Res<AvailableControlSchemes>,
  registered_players: Res<RegisteredPlayers>,
  network_role: Res<NetworkRole>,
) {
  for message in messages.read() {
    if message.enabled {
      spawn_touch_controls_ui(
        &mut commands,
        &available_control_schemes,
        &asset_server,
        &registered_players,
        &network_role,
      );
    } else {
      touch_controls_ui_query
        .iter_mut()
        .for_each(|e| commands.entity(e).despawn());
    }
  }
}

#[cfg(all(test, feature = "online"))]
mod tests {
  use super::*;
  #[cfg(feature = "online")]
  use crate::shared::RegisteredPlayer;
  #[cfg(feature = "online")]
  use crate::shared::{SharedMessagesPlugin, SharedResourcesPlugin};
  #[cfg(feature = "online")]
  use bevy::MinimalPlugins;

  #[cfg(feature = "online")]
  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SharedMessagesPlugin, SharedResourcesPlugin));
    app.init_resource::<ActiveMovementTracker>();
    app.add_systems(
      Update,
      (
        handle_touch_controls_player_registration_message,
        player_movement_input_action_emitter_system,
      ),
    );
    app
  }

  #[cfg(feature = "online")]
  fn test_control_scheme(id: u8) -> ControlScheme {
    ControlScheme::new(
      ControlSchemeId(id),
      KeyCode::ArrowLeft,
      KeyCode::ArrowRight,
      KeyCode::Space,
    )
  }

  #[cfg(feature = "online")]
  #[test]
  fn handle_touch_controls_player_registration_message_rebinds_touch_controls_to_authoritative_player() {
    let mut app = setup();
    *app.world_mut().resource_mut::<NetworkRole>() = NetworkRole::Client;

    {
      let mut registered_players = app.world_mut().resource_mut::<RegisteredPlayers>();
      registered_players
        .register(RegisteredPlayer::new_mutable(
          PlayerId(4),
          test_control_scheme(0),
          colour_for_player_id(PlayerId(4)),
        ))
        .expect("Expected authoritative local registration to succeed");
    }

    let parent_entity = app
      .world_mut()
      .spawn((
        PlayerTouchControls {
          control_scheme_id: ControlSchemeId(0),
        },
        PlayerId(0),
      ))
      .with_children(|parent| {
        parent.spawn((
          TouchControl::Movement(PlayerId(0), -1.0),
          TouchControlButton,
          BackgroundColor(Color::BLACK),
        ));
        parent.spawn((
          TouchControl::Action(PlayerId(0)),
          TouchControlButton,
          BackgroundColor(colour_for_player_id(PlayerId(0)).with_alpha(BUTTON_ALPHA_DEFAULT)),
        ));
      })
      .id();

    app
      .world_mut()
      .write_message(PlayerRegistrationMessage {
        player_id: PlayerId(4),
        control_scheme_id: Some(ControlSchemeId(0)),
        is_anyone_registered: true,
      })
      .expect("Expected PlayerRegistrationMessage to be queued");

    app.update();

    let player_id = app
      .world()
      .get::<PlayerId>(parent_entity)
      .copied()
      .expect("Expected touch controls to keep a PlayerId component");
    assert_eq!(player_id, PlayerId(4));

    let children = app
      .world()
      .get::<Children>(parent_entity)
      .expect("Expected touch controls to have spawned child buttons");

    let mut saw_rebound_action_button = false;
    for child in children.iter() {
      let touch_control = app
        .world()
        .get::<TouchControl>(child)
        .copied()
        .expect("Expected each child to carry a TouchControl");

      match touch_control {
        TouchControl::Movement(player_id, _) => assert_eq!(player_id, PlayerId(4)),
        TouchControl::Action(player_id) => {
          assert_eq!(player_id, PlayerId(4));
          let background_colour = app
            .world()
            .get::<BackgroundColor>(child)
            .expect("Expected action button to keep a background colour");
          assert_eq!(
            background_colour.0,
            colour_for_player_id(PlayerId(4)).with_alpha(BUTTON_ALPHA_DEFAULT)
          );
          saw_rebound_action_button = true;
        }
      }
    }

    assert!(saw_rebound_action_button, "Expected an action button to be rebound");
  }

  #[cfg(feature = "online")]
  #[test]
  fn handle_touch_controls_player_registration_message_updates_active_movement_tracker_to_authoritative_player() {
    let mut app = setup();
    *app.world_mut().resource_mut::<NetworkRole>() = NetworkRole::Client;

    {
      let mut registered_players = app.world_mut().resource_mut::<RegisteredPlayers>();
      registered_players
        .register(RegisteredPlayer::new_mutable(
          PlayerId(4),
          test_control_scheme(0),
          colour_for_player_id(PlayerId(4)),
        ))
        .expect("Expected authoritative local registration to succeed");
    }

    let mut movement_entity = None;
    app
      .world_mut()
      .spawn((
        PlayerTouchControls {
          control_scheme_id: ControlSchemeId(0),
        },
        PlayerId(0),
      ))
      .with_children(|parent| {
        movement_entity = Some(
          parent
            .spawn((
              TouchControl::Movement(PlayerId(0), -1.0),
              TouchControlButton,
              BackgroundColor(Color::BLACK),
            ))
            .id(),
        );
      });

    let movement_entity = movement_entity.expect("Expected to spawn a movement button");

    {
      let mut tracker = app.world_mut().resource_mut::<ActiveMovementTracker>();
      tracker.players.insert(
        PlayerId(0),
        (movement_entity, TouchControl::Movement(PlayerId(0), -1.0)),
      );
    }

    app
      .world_mut()
      .write_message(PlayerRegistrationMessage {
        player_id: PlayerId(4),
        control_scheme_id: Some(ControlSchemeId(0)),
        is_anyone_registered: true,
      })
      .expect("Expected PlayerRegistrationMessage to be queued");

    app.update();
    app.update();

    let messages = app
      .world_mut()
      .get_resource_mut::<Messages<InputMessage>>()
      .expect("Expected Messages<InputMessage> to be registered");
    assert!(
      messages
        .iter_current_update_messages()
        .any(|message| matches!(message, InputMessage::Move(PlayerId(4), -1.0))),
      "Expected movement emitter to use the authoritative PlayerId after rebinding"
    );
  }
}
