use crate::prelude::constants::*;
use crate::prelude::{TouchButton, TouchControlButton};
use crate::shared::{CustomInteraction, Settings};
use bevy::app::Update;
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::log::debug;
use bevy::prelude::{
  Alpha, App, BackgroundColor, BorderColor, BorderGradient, Changed, Component, DetectChangesMut, Entity,
  IntoScheduleConfigs, On, Out, Over, Plugin, Pointer, Press, Query, Release, Res, Time, With, default,
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
      .add_plugins((InGameUiPlugin, TouchControlsUiPlugin))
      .add_systems(
        Update,
        touch_contro_button_reactive_design_system.run_if(has_touch_controls_enabled),
      )
      .add_systems(Update, (touch_button_reactive_design_system, animate_button));
  }
}

/// Marker component for button animations.
#[derive(Component)]
struct ButtonAnimation;

fn has_touch_controls_enabled(settings: Res<Settings>) -> bool {
  settings.general.enable_touch_controls
}

/// A system that updates regular button colours based on their interaction state to provide visual feedback.
fn touch_button_reactive_design_system(
  mut interaction_query: Query<
    (
      &CustomInteraction,
      &mut BorderColor,
      &mut BackgroundColor,
      &mut BorderGradient,
      &mut TouchButton,
    ),
    Changed<CustomInteraction>,
  >,
) {
  for (interaction, mut border_colour, mut background_colour, mut border_gradient, mut button) in &mut interaction_query
  {
    match *interaction {
      CustomInteraction::Pressed => {
        *border_gradient = default_gradient(1.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_100));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      CustomInteraction::Released | CustomInteraction::Hovered => {
        *border_gradient = default_gradient(1.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_300));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      CustomInteraction::None => {
        *border_gradient = default_gradient(0.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
      }
    }
  }
}

/// A system that updates touch button colours based on their interaction state to provide visual feedback. Does not
/// require input focus changes and is therefore multitouch friendly.
fn touch_contro_button_reactive_design_system(
  mut interaction_query: Query<
    (
      &CustomInteraction,
      &mut BorderColor,
      &mut BackgroundColor,
      &mut TouchControlButton,
    ),
    Changed<CustomInteraction>,
  >,
) {
  for (interaction, mut border_colour, mut background_colour, mut button) in &mut interaction_query {
    match *interaction {
      CustomInteraction::Pressed => {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_100));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      CustomInteraction::Released | CustomInteraction::Hovered => {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_300));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
        button.set_changed();
      }
      CustomInteraction::None => {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
      }
    }
  }
}

fn animate_button(time: Res<Time>, mut query: Query<(&mut BorderGradient, &CustomInteraction), With<ButtonAnimation>>) {
  for (mut gradients, interaction) in query.iter_mut() {
    if *interaction == CustomInteraction::None {
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

//noinspection DuplicatedCode
fn set_interaction_on_hover(action: On<Pointer<Over>>, mut interaction_query: Query<(Entity, &mut CustomInteraction)>) {
  interaction_query
    .iter_mut()
    .filter(|(entity, _)| action.entity == *entity)
    .for_each(|(entity, mut interaction)| {
      if *interaction != CustomInteraction::Hovered {
        *interaction = CustomInteraction::Hovered;
        interaction.set_changed();
        debug!("Interaction for {entity} set to [{:?}]", *interaction);
      }
    });
}

//noinspection DuplicatedCode
fn set_interaction_on_hover_exit(
  action: On<Pointer<Out>>,
  mut interaction_query: Query<(Entity, &mut CustomInteraction)>,
) {
  interaction_query
    .iter_mut()
    .filter(|(entity, _)| action.entity == *entity)
    .for_each(|(entity, mut interaction)| {
      if *interaction != CustomInteraction::Hovered || *interaction != CustomInteraction::Pressed {
        *interaction = CustomInteraction::None;
        interaction.set_changed();
        debug!("Interaction for {entity} set to [{:?}]", *interaction);
      }
    });
}

//noinspection DuplicatedCode
fn set_interaction_on_press(
  action: On<Pointer<Press>>,
  mut interaction_query: Query<(Entity, &mut CustomInteraction)>,
) {
  interaction_query
    .iter_mut()
    .filter(|(entity, _)| action.entity == *entity)
    .for_each(|(entity, mut interaction)| {
      if *interaction != CustomInteraction::Pressed {
        *interaction = CustomInteraction::Pressed;
        interaction.set_changed();
        debug!("Interaction for {entity} set to [{:?}]", *interaction);
      }
    });
}

//noinspection DuplicatedCode
fn set_interaction_on_release(
  action: On<Pointer<Release>>,
  mut interaction_query: Query<(Entity, &mut CustomInteraction)>,
) {
  interaction_query
    .iter_mut()
    .filter(|(entity, _)| action.entity == *entity)
    .for_each(|(entity, mut interaction)| {
      *interaction = CustomInteraction::Released;
      interaction.set_changed();
      debug!("Interaction for {entity} set to [{:?}]", *interaction);
    });
}
