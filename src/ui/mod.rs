use crate::prelude::constants::*;
use crate::prelude::{AppState, RegularButton, TouchControlButton};
use crate::shared::{CustomInteraction, Settings};
#[cfg(feature = "online")]
use crate::ui::host_game_menu::HostGameMenuPlugin;
#[cfg(feature = "online")]
use crate::ui::join_game_menu::JoinGameMenuPlugin;
use crate::ui::main_menu::MainMenuPlugin;
use crate::ui::play_online_menu::PlayOnlineMenuPlugin;
use crate::ui::shared::{BackgroundRoot, ButtonAnimation};
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
#[cfg(feature = "online")]
use bevy_ui_text_input::TextInputPlugin;
use in_game_ui::InGameUiPlugin;
use touch_controls_ui::TouchControlsUiPlugin;

mod host_game_menu;
mod in_game_ui;
mod join_game_menu;
mod main_menu;
mod play_online_menu;
mod shared;
mod touch_controls_ui;

/// A system that manages the user interface elements of the game, including in-game UI and touch controls UI.
pub struct UiPlugin;

impl Plugin for UiPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((
        MainMenuPlugin,
        PlayOnlineMenuPlugin,
        InGameUiPlugin,
        TouchControlsUiPlugin,
      ))
      .add_systems(
        Update,
        touch_control_button_reactive_design_system.run_if(has_touch_controls_enabled),
      )
      .add_systems(Update, (regular_button_reactive_design_system, animate_button_system))
      .add_systems(PostUpdate, clear_released_interaction_system)
      .add_systems(OnExit(AppState::Preparing), despawn_background_system);

    #[cfg(feature = "online")]
    app.add_plugins((HostGameMenuPlugin, JoinGameMenuPlugin, TextInputPlugin));
  }
}

fn has_touch_controls_enabled(settings: Res<Settings>) -> bool {
  settings.general.enable_touch_controls
}

/// A system that changes the visual appearance of regular buttons based on their interaction state.
fn regular_button_reactive_design_system(
  mut interaction_query: Query<
    (
      &CustomInteraction,
      &mut BorderColor,
      &mut BackgroundColor,
      &mut BorderGradient,
      &mut RegularButton,
    ),
    Changed<CustomInteraction>,
  >,
) {
  for (interaction, mut border_colour, mut background_colour, mut border_gradient, mut button) in &mut interaction_query
  {
    match *interaction {
      CustomInteraction::Pressed => {
        *border_gradient = shared::default_gradient(1.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_100));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      CustomInteraction::Released | CustomInteraction::Hovered => {
        *border_gradient = shared::default_gradient(1.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_300));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_PRESSED));
        button.set_changed();
      }
      CustomInteraction::None => {
        *border_gradient = shared::default_gradient(0.);
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
      }
    }
  }
}

/// A system that changes the visual appearance of touch button based on their interaction state.
fn touch_control_button_reactive_design_system(
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
      CustomInteraction::Hovered => {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_300));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
        button.set_changed();
      }
      CustomInteraction::Released | CustomInteraction::None => {
        *border_colour = BorderColor::all(Color::from(tailwind::SLATE_500));
        *background_colour = BackgroundColor(background_colour.0.with_alpha(BUTTON_ALPHA_DEFAULT));
      }
    }
  }
}

/// A system that animates button border gradients when interacted with by rotating the gradient angle.
fn animate_button_system(
  time: Res<Time>,
  mut query: Query<(&mut BorderGradient, &CustomInteraction), With<ButtonAnimation>>,
) {
  for (mut gradients, interaction) in query.iter_mut() {
    if *interaction == CustomInteraction::None {
      continue;
    }
    for gradient in gradients.0.iter_mut() {
      if let Gradient::Linear(LinearGradient { angle, .. }) = gradient {
        *angle += 1.5 * time.delta_secs();
      }
    }
  }
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
        trace!("Interaction for {entity} set to [{:?}]", *interaction);
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
        trace!("Interaction for {entity} set to [{:?}]", *interaction);
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
        trace!("Interaction for {entity} set to [{:?}]", *interaction);
      }
    });
}

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
      trace!("Interaction for {entity} set to [{:?}]", *interaction);
    });
}

//noinspection DuplicatedCode
fn set_interaction_on_cancel(
  action: On<Pointer<Cancel>>,
  mut interaction_query: Query<(Entity, &mut CustomInteraction)>,
) {
  interaction_query
    .iter_mut()
    .filter(|(entity, _)| action.entity == *entity)
    .for_each(|(entity, mut interaction)| {
      if *interaction != CustomInteraction::None {
        *interaction = CustomInteraction::None;
        interaction.set_changed();
        trace!("Interaction for {entity} set to [{:?}]", *interaction);
      }
    });
}

/// A system to clear the transient [`CustomInteraction::Released`] state by resetting to [`CustomInteraction::None`].
/// Runs only when [`CustomInteraction`] has changed. Is intended to be run in a later update stage such as
/// [`PostUpdate`].
fn clear_released_interaction_system(mut query: Query<(Entity, &mut CustomInteraction), Changed<CustomInteraction>>) {
  for (entity, mut interaction) in query.iter_mut() {
    if *interaction == CustomInteraction::Released {
      *interaction = CustomInteraction::None;
      interaction.set_changed();
      trace!("Interaction for {entity} set to [{:?}]", *interaction);
    }
  }
}

/// A system to despawn all entities with the [`BackgroundRoot`] component. Intended to be run when exiting the [`AppState::Preparing`] state.
fn despawn_background_system(mut commands: Commands, background_root_query: Query<Entity, With<BackgroundRoot>>) {
  for entity in background_root_query.iter() {
    commands.entity(entity).despawn();
  }
}
