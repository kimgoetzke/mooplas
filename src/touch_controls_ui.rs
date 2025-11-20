use crate::app_states::AppState;
use crate::prelude::constants::DEFAULT_FONT;
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
// TODO: Ensure input action can be used to register player (decouple from keyboard input)
/// Spawns the touch controls UI.
fn spawn_touch_controls_ui(
  commands: &mut Commands,
  available_configs: &AvailablePlayerConfigs,
  asset_server: &Res<AssetServer>,
) {
  let default_font = asset_server.load(DEFAULT_FONT);
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
      parent.spawn((
        Button,
        node.clone(),
        ButtonMovement::new(config.id.into(), -1.0),
        BackgroundColor(Color::from(tailwind::SLATE_600)),
        BorderColor::all(Color::from(tailwind::SLATE_100)),
        BorderRadius::MAX,
        children![(
          Text::new("<"),
          TextFont {
            font: default_font.clone(),
            font_size: 33.0,
            ..default()
          },
          TextColor(Color::srgb(0.9, 0.9, 0.9)),
        )],
      ));
      parent.spawn((
        Button,
        node.clone(),
        ButtonAction::new(config.id.into()),
        BackgroundColor(Color::from(tailwind::SLATE_600)),
        BorderColor::all(Color::from(tailwind::SLATE_100)),
        BorderRadius::MAX,
        children![(
          Text::new("  "),
          TextFont {
            font: default_font.clone(),
            font_size: 33.0,
            ..default()
          },
          TextColor(Color::srgb(0.9, 0.9, 0.9)),
        )],
      ));
      parent.spawn((
        Button,
        node.clone(),
        ButtonMovement::new(config.id.into(), 1.0),
        BackgroundColor(Color::from(tailwind::SLATE_600)),
        BorderColor::all(Color::from(tailwind::SLATE_100)),
        BorderRadius::MAX,
        children![(
          Text::new(">"),
          TextFont {
            font: default_font.clone(),
            font_size: 33.0,
            ..default()
          },
          TextColor(Color::srgb(0.9, 0.9, 0.9)),
        )],
      ));
    });
  }
}

fn button_node() -> Node {
  Node {
    width: px(40),
    height: px(40),
    border: UiRect::all(px(5)),
    justify_content: JustifyContent::Center,
    align_items: AlignItems::Center,
    margin: UiRect::all(px(10)),
    ..default()
  }
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
  mut interaction_query: Query<(Entity, &Interaction, &mut BorderColor, &mut Button), Changed<Interaction>>,
) {
  for (entity, interaction, mut border_color, mut button) in &mut interaction_query {
    match *interaction {
      Interaction::Pressed => {
        input_focus.set(entity);
        *border_color = BorderColor::all(Color::from(tailwind::RED_300));
        button.set_changed();
      }
      Interaction::Hovered => {
        input_focus.set(entity);
        *border_color = BorderColor::all(Color::from(tailwind::BLUE_300));
        button.set_changed();
      }
      Interaction::None => {
        input_focus.clear();
        *border_color = BorderColor::all(Color::from(tailwind::SLATE_100));
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
