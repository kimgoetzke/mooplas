use crate::app_states::AppState;
use crate::prelude::constants::{EDGE_MARGIN, RESOLUTION_HEIGHT, RESOLUTION_WIDTH};
use crate::prelude::{AvailablePlayerConfig, AvailablePlayerConfigs, PlayerId, PlayerInput, SpawnPoints};
use bevy::app::{App, Plugin};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::log::*;
use bevy::platform::collections::HashSet;
use bevy::prelude::{IntoScheduleConfigs, KeyCode, NextState, OnEnter, Res, ResMut, Resource, Update, in_state};
use rand::Rng;
use rand::prelude::ThreadRng;

/// A plugin that initialises the game by loading resources and generation data such as spawn points.
pub struct InitialisationPlugin;

impl Plugin for InitialisationPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<InitialisationTracker>()
      .add_systems(Update, check_progress_system.run_if(in_state(AppState::Initialising)))
      .add_systems(
        OnEnter(AppState::Initialising),
        (
          reset_tracker_system,
          generate_valid_spawn_points_system,
          initialise_available_player_configurations_system,
        )
          .chain(),
      );
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InitialisationStep {
  GenerateSpawnPoints,
  InitialiseAvailablePlayerConfigs,
}

#[derive(Resource)]
struct InitialisationTracker {
  is_first_run: bool,
  completed: HashSet<InitialisationStep>,
  required: Vec<InitialisationStep>,
}

impl Default for InitialisationTracker {
  fn default() -> Self {
    Self {
      is_first_run: true,
      completed: HashSet::new(),
      required: Vec::new(),
    }
  }
}

impl InitialisationTracker {
  fn reset(&mut self, required: Vec<InitialisationStep>) {
    self.completed = HashSet::new();
    self.required = required;
  }

  fn should_skip(&self, step: InitialisationStep) -> bool {
    !self.required.contains(&step)
  }

  fn mark_done(&mut self, step: InitialisationStep) {
    debug!("Initialisation step [{:?}] completed", step);
    self.completed.insert(step);
  }

  fn all_done(&self) -> bool {
    self.required.iter().all(|step| self.completed.contains(step))
  }
}

/// Resets the tracker with the required [`InitialisationStep`]s.
///
/// New steps must be added here as needed. In addition, the corresponding systems must be added to the initialisation
/// chain in the plugin.
fn reset_tracker_system(mut tracker: ResMut<InitialisationTracker>) {
  let required = if tracker.is_first_run {
    vec![
      InitialisationStep::GenerateSpawnPoints,
      InitialisationStep::InitialiseAvailablePlayerConfigs,
    ]
  } else {
    vec![InitialisationStep::GenerateSpawnPoints]
  };
  tracker.is_first_run = false;
  tracker.reset(required);
}

/// Polling system that runs in parallel to the re-/initialisation process and advances the app state once all steps are
/// complete.
fn check_progress_system(tracker: Res<InitialisationTracker>, mut next_state: ResMut<NextState<AppState>>) {
  if tracker.all_done() {
    debug!("✅  Initialisation completed");
    next_state.set(AppState::Registering);
  }
}

// TODO: Create lambda equivalent for tracker boilerplate
// TODO: Ensure spawn points are not to close to each other
// TODO: Add random rotation so that players don't all face the same direction
/// A system that provides random but safe spawn points for players.
fn generate_valid_spawn_points_system(
  mut tracker: ResMut<InitialisationTracker>,
  mut spawn_points: ResMut<SpawnPoints>,
) {
  let this_step = InitialisationStep::GenerateSpawnPoints;
  if tracker.should_skip(this_step) {
    debug!("Skipping [{:?}] step...", this_step);
    return;
  }

  spawn_points.points.clear();
  let mut rng = rand::rng();
  for i in 0..10 {
    let (x, y) = random_start_position(&mut rng);
    spawn_points.points.push((x, y));
    trace!("Generated spawn point [{}] at position: ({}, {})", i + 1, x, y);
  }

  tracker.mark_done(this_step);
}

/// Calculates a random start position for the player that is at least [`EDGE_MARGIN`] pixels away from the screen edges.
fn random_start_position(rng: &mut ThreadRng) -> (f32, f32) {
  let half_w = RESOLUTION_WIDTH as f32 / 2.0;
  let half_h = RESOLUTION_HEIGHT as f32 / 2.0;
  let min_x = -half_w + EDGE_MARGIN;
  let max_x = half_w - EDGE_MARGIN;
  let min_y = -half_h + EDGE_MARGIN;
  let max_y = half_h - EDGE_MARGIN;

  if min_x > max_x || min_y > max_y {
    return (0., 0.);
  }

  let x = rng.random_range(min_x..=max_x).trunc();
  let y = rng.random_range(min_y..=max_y).trunc();

  (x, y)
}

fn initialise_available_player_configurations_system(
  mut tracker: ResMut<InitialisationTracker>,
  mut available_configs: ResMut<AvailablePlayerConfigs>,
) {
  let this_step = InitialisationStep::InitialiseAvailablePlayerConfigs;
  if tracker.should_skip(this_step) {
    debug!("Skipping [{:?}] step...", this_step);
    return;
  }

  available_configs.configs = vec![
    AvailablePlayerConfig {
      id: PlayerId(0),
      input: PlayerInput::new(PlayerId(0), KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyW),
      colour: Color::from(tailwind::LIME_500),
    },
    AvailablePlayerConfig {
      id: PlayerId(1),
      input: PlayerInput::new(PlayerId(1), KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp),
      colour: Color::from(tailwind::ROSE_500),
    },
    AvailablePlayerConfig {
      id: PlayerId(2),
      input: PlayerInput::new(PlayerId(2), KeyCode::KeyB, KeyCode::KeyM, KeyCode::KeyH),
      colour: Color::from(tailwind::SKY_500),
    },
  ];

  tracker.mark_done(this_step);
}
