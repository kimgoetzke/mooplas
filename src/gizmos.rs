use crate::constants::SNAKE_HEAD_SIZE;
use crate::shared::{Settings, SnakeHead, SpawnPoints};
use avian2d::math::Vector;
use bevy::app::{App, Plugin, Update};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::math::{Isometry2d, Vec2};
use bevy::prelude::{Gizmos, GlobalTransform, Query, Res, With};

/// A plugin that renders gizmos for debugging purposes.
pub struct GizmosPlugin;

impl Plugin for GizmosPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, (render_gizmos_system,));
  }
}

fn render_gizmos_system(
  mut gizmos: Gizmos,
  settings: Res<Settings>,
  spawn_points: Res<SpawnPoints>,
  snake_head_query: Query<&GlobalTransform, With<SnakeHead>>,
) {
  if !settings.general.display_player_gizmos {
    return;
  }

  // Spawn points
  for (x, y) in spawn_points.points.iter() {
    gizmos.circle_2d(
      Isometry2d::from_translation(Vec2::new(*x, *y)),
      SNAKE_HEAD_SIZE,
      Color::from(tailwind::CYAN_300),
    );
  }

  // Players
  for transform in snake_head_query.iter() {
    let translation = transform.translation();

    // Head collider
    gizmos.circle_2d(
      Isometry2d::from_translation(Vector::new(translation.x, translation.y)),
      SNAKE_HEAD_SIZE,
      Color::from(tailwind::AMBER_400),
    );

    // Head center point
    gizmos.circle_2d(
      Isometry2d::from_translation(Vector::new(translation.x, translation.y)),
      0.5,
      Color::from(tailwind::AMBER_400),
    );
  }
}
