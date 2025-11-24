use crate::prelude::TouchButton;
use crate::prelude::constants::*;
use crate::shared::Settings;
use bevy::app::Update;
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::input_focus::InputFocus;
use bevy::prelude::{
  Alpha, App, BackgroundColor, BorderColor, BorderGradient, Button, Changed, Component, DetectChangesMut, Entity,
  Interaction, IntoScheduleConfigs, MeshPickingPlugin, Plugin, Query, Res, ResMut, Time, With, default,
};
use bevy::ui::{Gradient, LinearGradient};
use in_game_ui::InGameUiPlugin;
use touch_controls_ui::TouchControlsUiPlugin;

pub mod in_game_ui;
pub mod touch_controls_ui;

pub struct UiPlugin;

impl Plugin for UiPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<InputFocus>()
      .add_plugins(MeshPickingPlugin)
      .add_plugins((InGameUiPlugin, TouchControlsUiPlugin))
      .add_systems(
        Update,
        touch_button_reactive_design_system.run_if(has_touch_controls_enabled),
      )
      .add_systems(Update, (button_reactive_design_system, animate_button));
  }
}

/// Marker component for button animations.
#[derive(Component)]
struct ButtonAnimation;

fn has_touch_controls_enabled(settings: Res<Settings>) -> bool {
  settings.general.enable_touch_controls
}

/// A system that updates regular button colours based on their interaction state to provide visual feedback.
fn button_reactive_design_system(
  mut input_focus: ResMut<InputFocus>,
  mut interaction_query: Query<
    (
      Entity,
      &Interaction,
      &mut BorderColor,
      &mut BackgroundColor,
      &mut BorderGradient,
      &mut Button,
    ),
    Changed<Interaction>,
  >,
) {
  for (entity, interaction, mut border_colour, mut background_colour, mut border_gradient, mut button) in
    &mut interaction_query
  {
    match *interaction {
      Interaction::Pressed => {
        input_focus.set(entity);
        *border_gradient = default_gradient(1.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_100));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      Interaction::Hovered => {
        input_focus.set(entity);
        *border_gradient = default_gradient(1.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_300));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      Interaction::None => {
        input_focus.clear();
        *border_gradient = default_gradient(0.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
      }
    }
  }
}

/// A system that updates touch button colours based on their interaction state to provide visual feedback. Does not
/// require input focus changes and is therefore multitouch friendly.
fn touch_button_reactive_design_system(
  mut interaction_query: Query<
    (&Interaction, &mut BorderColor, &mut BackgroundColor, &mut TouchButton),
    Changed<Interaction>,
  >,
) {
  for (interaction, mut border_colour, mut background_colour, mut button) in &mut interaction_query {
    match *interaction {
      Interaction::Pressed => {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_100));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      Interaction::Hovered => {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_300));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
        button.set_changed();
      }
      Interaction::None => {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
      }
    }
  }
}

fn animate_button(time: Res<Time>, mut query: Query<(&mut BorderGradient, &Interaction), With<ButtonAnimation>>) {
  for (mut gradients, interaction) in query.iter_mut() {
    if *interaction == Interaction::None {
      continue;
    }
    for gradient in gradients.0.iter_mut() {
      if let Gradient::Linear(LinearGradient { angle, .. }) = gradient {
        *angle += 1. * time.delta_secs();
      }
    }
  }
}

fn default_gradient(transparency: f32) -> BorderGradient {
  BorderGradient::from(LinearGradient {
    stops: vec![
      tailwind::YELLOW_400.with_alpha(transparency).into(),
      tailwind::YELLOW_50.with_alpha(transparency).into(),
      tailwind::YELLOW_400.with_alpha(transparency).into(),
    ],
    ..default()
  })
}
