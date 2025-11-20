use crate::app_states::AppState;
use crate::prelude::{AvailablePlayerConfigs, PlayerId, Settings, TouchControlsToggledMessage};
use crate::shared::InputAction;
use avian2d::math::Scalar;
use bevy::color::palettes::tailwind;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;

pub struct TouchControlsUiPlugin;

impl Plugin for TouchControlsUiPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, spawn_touch_controls_ui_system)
      .init_resource::<InputFocus>()
      .add_systems(Update, (button_design_system, button_action_input_system))
      .add_systems(Update, button_movement_input_system.run_if(in_state(AppState::Playing)))
      .add_systems(Update, handle_toggle_touch_controls_message);
  }
}

const BUTTON_ALPHA_DEFAULT: f32 = 0.3;
const BUTTON_ALPHA_PRESSED: f32 = 0.8;

#[derive(Component)]
struct TouchControlsUi;

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

// TODO: Merge controls for a single player into one set of buttons
// TODO: Design better buttons
// TODO: Replace text with icons
// TODO: Position buttons around the screen for more comfortable access
// TODO: Make buttons respond to touch input
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
        align_items: AlignItems::End,
        justify_content: JustifyContent::Center,
        ..default()
      },
      ZIndex::from(ZIndex(100)),
    ))
    .id();

  for config in available_configs.configs.iter() {
    commands.entity(parent).with_children(|parent| {
      let node = button_node();
      let button_style = button_with_style();
      parent.spawn((
        Name::new("Controls for Player ".to_string() + &config.id.to_string()),
        Node {
          flex_direction: FlexDirection::Row,
          margin: UiRect::all(Val::Px(10.0)),
          ..default()
        },
        children![
          (
            // Left movement button
            node.clone(),
            button_style.clone(),
            ButtonMovement::new(config.id.into(), -1.0),
            BorderRadius {
              top_left: Val::Percent(50.0),
              bottom_left: Val::Percent(50.0),
              top_right: Val::Percent(20.0),
              bottom_right: Val::Percent(20.0),
            },
          ),
          (
            // Player action button
            node.clone(),
            button_with_custom_style(config.colour),
            ButtonAction::new(config.id.into()),
            BorderRadius::all(Val::Percent(20.0)),
          ),
          (
            // Right movement button
            node.clone(),
            button_style.clone(),
            ButtonMovement::new(config.id.into(), 1.0),
            BorderRadius {
              top_left: Val::Percent(20.0),
              bottom_left: Val::Percent(20.0),
              top_right: Val::Percent(50.0),
              bottom_right: Val::Percent(50.0),
            },
          )
        ],
      ));
    });
  }
}

fn button_node() -> Node {
  Node {
    width: px(40),
    height: px(40),
    border: UiRect::all(px(2)),
    justify_content: JustifyContent::Center,
    align_items: AlignItems::Center,
    margin: UiRect::all(px(10)),
    ..default()
  }
}

fn button_with_style() -> (Button, BackgroundColor, BorderColor) {
  (
    Button,
    BackgroundColor(Color::from(tailwind::SLATE_600).with_alpha(BUTTON_ALPHA_DEFAULT)),
    BorderColor::all(Color::from(tailwind::SLATE_500).with_alpha(0.2)),
  )
}

fn button_with_custom_style(colour: Color) -> (Button, BackgroundColor, BorderColor) {
  (
    Button,
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
      spawn_touch_controls_ui(&mut commands, &available_configs, &asset_server);
    } else {
      query.iter_mut().for_each(|e| {
        commands.entity(e).despawn();
      });
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
