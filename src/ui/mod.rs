use crate::prelude::TouchButton;
use crate::prelude::constants::*;
use bevy::app::Update;
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::input_focus::InputFocus;
use bevy::prelude::{
  Alpha, App, BackgroundColor, BorderColor, Button, Changed, DetectChangesMut, Entity, Interaction, MeshPickingPlugin,
  Plugin, Query, ResMut,
};
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
        (button_reactive_design_system, touch_button_reactive_design_system),
      );
  }
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
