use crate::prelude::constants::{EDGE_MARGIN, RESOLUTION_HEIGHT, RESOLUTION_WIDTH};
use crate::prelude::{
  AppState, AvailablePlayerConfig, AvailablePlayerConfigs, PlayerId, PlayerInput, Seed, SpawnPoints,
};
use bevy::app::{App, Plugin};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::log::*;
use bevy::platform::collections::HashSet;
use bevy::prelude::{IntoScheduleConfigs, KeyCode, NextState, OnEnter, Res, ResMut, Resource, Update, in_state};
use rand::prelude::StdRng;
use rand::{RngExt, SeedableRng};

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

/// An enumeration of all possible initialisation steps. Add new steps here as needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InitialisationStep {
  GenerateSpawnPoints,
  InitialiseAvailablePlayerConfigs,
}

/// A resource that tracks the progress of the initialisation steps.
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

/// Runs an initialisation step if it has not already been completed. Marks the step as done afterwards.
fn run_initialisation_step<F>(tracker: &mut InitialisationTracker, step: InitialisationStep, initialisation_logic: F)
where
  F: FnOnce(),
{
  if tracker.should_skip(step) {
    debug!("Skipping [{:?}] step...", step);
    return;
  }
  initialisation_logic();
  tracker.mark_done(step);
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

/// A system that provides random but safe spawn points for players.
fn generate_valid_spawn_points_system(
  mut tracker: ResMut<InitialisationTracker>,
  mut spawn_points: ResMut<SpawnPoints>,
  seed: Res<Seed>,
) {
  run_initialisation_step(&mut tracker, InitialisationStep::GenerateSpawnPoints, || {
    spawn_points.data.clear();
    let mut rng = StdRng::seed_from_u64(seed.get());
    let mut valid_spawn_points: Vec<(f32, f32, f32)> = Vec::new();
    while valid_spawn_points.len() < 10 {
      let (x, y) = random_start_position(&mut rng);
      let rotation = rng.random_range(0.0..=360.);
      if valid_spawn_points.iter().any(|(other_x, other_y, _)| {
        let dx = other_x - x;
        let dy = other_y - y;
        let distance_squared = dx * dx + dy * dy;
        distance_squared < (EDGE_MARGIN * EDGE_MARGIN)
      }) {
        trace!(
          "Rejected spawn point at ({}, {}) due to proximity to existing spawn points",
          x, y
        );
        continue;
      }
      valid_spawn_points.push((x, y, rotation));
      trace!(
        "Generated spawn point [{}] at position: ({}, {})",
        valid_spawn_points.len(),
        x,
        y
      );
    }
    spawn_points.data = valid_spawn_points;
  });
}

/// Calculates a random start position for the player that is at least [`EDGE_MARGIN`] pixels away from the screen edges.
fn random_start_position(rng: &mut StdRng) -> (f32, f32) {
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

/// A system that initialises all available player configurations that players can choose from.
fn initialise_available_player_configurations_system(
  mut tracker: ResMut<InitialisationTracker>,
  mut available_configs: ResMut<AvailablePlayerConfigs>,
) {
  run_initialisation_step(
    &mut tracker,
    InitialisationStep::InitialiseAvailablePlayerConfigs,
    || {
      available_configs.configs = vec![
        AvailablePlayerConfig {
          id: PlayerId(0),
          input: PlayerInput::new(PlayerId(0), KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp),
          colour: Color::from(tailwind::ROSE_500),
        },
        AvailablePlayerConfig {
          id: PlayerId(1),
          input: PlayerInput::new(PlayerId(1), KeyCode::Digit1, KeyCode::KeyA, KeyCode::KeyQ),
          colour: Color::from(tailwind::LIME_500),
        },
        AvailablePlayerConfig {
          id: PlayerId(2),
          input: PlayerInput::new(PlayerId(2), KeyCode::KeyZ, KeyCode::KeyC, KeyCode::KeyX),
          colour: Color::from(tailwind::SKY_500),
        },
        AvailablePlayerConfig {
          id: PlayerId(3),
          input: PlayerInput::new(PlayerId(3), KeyCode::KeyB, KeyCode::KeyM, KeyCode::KeyN),
          colour: Color::from(tailwind::VIOLET_500),
        },
        AvailablePlayerConfig {
          id: PlayerId(4),
          input: PlayerInput::new(PlayerId(4), KeyCode::End, KeyCode::PageUp, KeyCode::Home),
          colour: Color::from(tailwind::AMBER_500),
        },
      ];
    },
  );
}

#[cfg(test)]
mod tests {
  use super::*;
  use bevy::MinimalPlugins;
  use bevy::app::App;

  #[test]
  fn initialisation_tracker_default_and_reset_behaviour() {
    let mut tracker = InitialisationTracker::default();

    // Verify default state
    assert!(tracker.is_first_run);
    assert!(tracker.all_done());

    // Reset with required steps
    tracker.reset(vec![InitialisationStep::GenerateSpawnPoints]);
    assert!(!tracker.should_skip(InitialisationStep::GenerateSpawnPoints));
    assert!(tracker.should_skip(InitialisationStep::InitialiseAvailablePlayerConfigs));
  }

  #[test]
  fn run_initialisation_step_skips_when_not_required_and_marks_when_required() {
    let mut tracker = InitialisationTracker::default();

    // When not required the closure should not be executed
    tracker.reset(vec![]);
    let mut is_non_required_step_called = false;
    run_initialisation_step(&mut tracker, InitialisationStep::GenerateSpawnPoints, || {
      is_non_required_step_called = true;
    });
    assert!(
      !is_non_required_step_called,
      "Step was executed even though it should have been skipped"
    );

    // When required the closure should be executed and the step marked done
    tracker.reset(vec![InitialisationStep::GenerateSpawnPoints]);
    let mut is_required_step_called = false;
    run_initialisation_step(&mut tracker, InitialisationStep::GenerateSpawnPoints, || {
      is_required_step_called = true;
    });
    assert!(
      is_required_step_called,
      "Step was not executed even though it was required"
    );
    assert!(tracker.all_done(), "Tracker did not mark the step as done");
  }

  #[test]
  fn random_start_position_respects_edge_margin_and_resolution() {
    let mut rng = StdRng::seed_from_u64(1);
    let (x, y) = random_start_position(&mut rng);
    let half_w = RESOLUTION_WIDTH as f32 / 2.0;
    let half_h = RESOLUTION_HEIGHT as f32 / 2.0;
    let min_x = -half_w + EDGE_MARGIN;
    let max_x = half_w - EDGE_MARGIN;
    let min_y = -half_h + EDGE_MARGIN;
    let max_y = half_h - EDGE_MARGIN;

    assert!(
      x >= min_x && x <= max_x,
      "x out of bounds: {} not in [{}, {}]",
      x,
      min_x,
      max_x
    );
    assert!(
      y >= min_y && y <= max_y,
      "y out of bounds: {} not in [{}, {}]",
      y,
      min_y,
      max_y
    );
  }

  #[test]
  fn generate_valid_spawn_points_system_populates_spawn_points_with_separated_points() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Prepare resources
    let mut tracker = InitialisationTracker::default();
    tracker.reset(vec![InitialisationStep::GenerateSpawnPoints]);
    app
      .insert_resource(tracker)
      .insert_resource(Seed::default())
      .insert_resource(SpawnPoints::default());

    // Add system and run one update to execute it
    app.add_systems(Update, generate_valid_spawn_points_system);
    app.update();

    // Validate spawn points
    let spawn_points = app.world().get_resource::<SpawnPoints>().expect("SpawnPoints missing");
    assert_eq!(spawn_points.data.len(), 10, "Expected 10 spawn points to be generated");

    // Ensure points are at least EDGE_MARGIN apart from each other
    let min_distance_sq = EDGE_MARGIN * EDGE_MARGIN;
    for i in 0..spawn_points.data.len() {
      for j in (i + 1)..spawn_points.data.len() {
        let (x1, y1, _) = spawn_points.data[i];
        let (x2, y2, _) = spawn_points.data[j];
        let dx = x1 - x2;
        let dy = y1 - y2;
        let dist_sq = dx * dx + dy * dy;
        assert!(
          dist_sq >= min_distance_sq,
          "Spawn points too close: {} and {} (dist_sq = {})",
          i,
          j,
          dist_sq
        );
      }
    }
  }

  #[test]
  fn initialise_available_player_configurations_system_populates_configs() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Prepare resources
    let mut tracker = InitialisationTracker::default();
    tracker.reset(vec![InitialisationStep::InitialiseAvailablePlayerConfigs]);
    app.insert_resource(tracker);
    app.insert_resource(AvailablePlayerConfigs::default());

    // Add system and run one update to execute it
    app.add_systems(Update, initialise_available_player_configurations_system);
    app.update();

    // Validate available player configs
    let available_configs = app
      .world()
      .get_resource::<AvailablePlayerConfigs>()
      .expect("AvailablePlayerConfigs missing");
    assert_eq!(
      available_configs.configs.len(),
      5,
      "Expected 5 available player configurations"
    );

    // Validate that player IDs are unique
    let mut player_ids: Vec<usize> = available_configs.configs.iter().map(|c| c.id.0 as usize).collect();
    player_ids.sort_unstable();
    player_ids.dedup();
    assert_eq!(
      player_ids.len(),
      available_configs.configs.len(),
      "Player ids are not unique"
    );

    // Validate that each config has a unique colour assigned
    let mut seen = HashSet::new();
    for (idx, available_config) in available_configs.configs.iter().enumerate() {
      let key = (
        (available_config.colour.to_srgba().red * 10000.) as u32,
        (available_config.colour.to_srgba().green * 10000.) as u32,
        (available_config.colour.to_srgba().blue * 10000.) as u32,
        (available_config.colour.to_srgba().alpha * 10000.) as u32,
      );
      assert!(seen.insert(key), "Player colour duplicated at config {}", idx);
    }
  }
}
