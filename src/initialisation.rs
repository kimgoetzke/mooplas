use crate::app_states::AppState;
use crate::prelude::SpawnPoints;
use crate::prelude::constants::{EDGE_MARGIN, RESOLUTION_HEIGHT, RESOLUTION_WIDTH};
use bevy::app::{App, Plugin};
use bevy::log::debug;
use bevy::platform::collections::HashSet;
use bevy::prelude::{IntoScheduleConfigs, NextState, OnEnter, Res, ResMut, Resource, Update, in_state};
use rand::Rng;
use rand::prelude::ThreadRng;

// A plugin that initialises the game by loading resources and generation data such as spawn points.
pub struct InitialisationPlugin;

impl Plugin for InitialisationPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<InitialisationTracker>()
      .add_systems(
        Update,
        check_initialisation_progress_system.run_if(in_state(AppState::Initialising)),
      )
      .add_systems(
        OnEnter(AppState::Initialising),
        (reset_initialisation_tracker_system, generate_valid_spawn_points_system).chain(),
      );
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InitialisationStep {
  GenerateSpawnPoints,
}

#[derive(Resource, Default)]
struct InitialisationTracker {
  completed: HashSet<InitialisationStep>,
  required: Vec<InitialisationStep>,
}

impl InitialisationTracker {
  fn reset(&mut self, required: Vec<InitialisationStep>) {
    self.completed = HashSet::new();
    self.required = required;
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
fn reset_initialisation_tracker_system(mut tracker: ResMut<InitialisationTracker>) {
  let required = vec![InitialisationStep::GenerateSpawnPoints];
  tracker.reset(required);
}

/// Polling system that runs in parallel to the initialisation process and advances the app state once all steps are
/// complete.
fn check_initialisation_progress_system(
  tracker: Res<InitialisationTracker>,
  mut next_state: ResMut<NextState<AppState>>,
) {
  if tracker.all_done() {
    debug!("✅  Initialisation completed");
    next_state.set(AppState::Waiting);
  }
}

// TODO: Ensure spawn points are not to close to each other.
/// A system that provides random but safe spawn points for players.
fn generate_valid_spawn_points_system(
  mut tracker: ResMut<InitialisationTracker>,
  mut spawn_points: ResMut<SpawnPoints>,
) {
  let mut rng = rand::rng();
  for i in 0..5 {
    let (x, y) = random_start_position(&mut rng);
    spawn_points.points.push((x, y));
    debug!("Generated spawn point [{}] at position: ({}, {})", i + 1, x, y);
  }
  tracker.mark_done(InitialisationStep::GenerateSpawnPoints);
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
