use crate::prelude::SnakeTail;
use crate::prelude::constants::SNAKE_HEAD_SIZE;
use crate::prelude::constants::TAIL_COLLIDER_SKIP_RECENT;
use crate::prelude::{Settings, SnakeHead, SpawnPoints};
use avian2d::math::Vector;
use bevy::app::{App, Plugin, Update};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::math::{Isometry2d, Vec2};
use bevy::prelude::{Alpha, Gizmos, GlobalTransform, Query, Res, With};

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
  snake_tail_query: Query<(&GlobalTransform, &SnakeTail)>,
) {
  if !settings.general.display_player_gizmos {
    return;
  }

  // Spawn points
  for (x, y) in spawn_points.points.iter() {
    gizmos.circle_2d(Isometry2d::from_translation(Vec2::new(*x, *y)), 1.0, Color::WHITE);
  }

  // Players
  let colour = Color::WHITE;
  draw_snake_head_gizmos(&mut gizmos, snake_head_query, colour);
  draw_snake_tail_gizmos(&mut gizmos, snake_tail_query, colour);
}

fn draw_snake_head_gizmos(
  gizmos: &mut Gizmos,
  snake_head_query: Query<&GlobalTransform, With<SnakeHead>>,
  colour: Color,
) {
  for transform in snake_head_query.iter() {
    let translation = transform.translation();

    // Head collider
    gizmos.circle_2d(
      Isometry2d::from_translation(Vector::new(translation.x, translation.y)),
      SNAKE_HEAD_SIZE,
      colour.clone(),
    );

    // Head center point
    gizmos.circle_2d(
      Isometry2d::from_translation(Vector::new(translation.x, translation.y)),
      0.5,
      colour,
    );
  }
}

fn draw_snake_tail_gizmos(gizmos: &mut Gizmos, snake_tail_query: Query<(&GlobalTransform, &SnakeTail)>, colour: Color) {
  for (global_transform, snake_tail) in snake_tail_query.iter() {
    let translation = global_transform.translation();
    for segment in &snake_tail.segments {
      let positions = segment.positions();
      if positions.is_empty() {
        continue;
      }

      // Sampled tail positions
      for position in positions.iter() {
        gizmos.circle_2d(
          Isometry2d::from_translation(Vec2::new(translation.x + position.x, translation.y + position.y)),
          0.35,
          colour.with_alpha(0.6),
        );
      }

      // Collider endpoints
      let collider_end_index = positions.len().saturating_sub(TAIL_COLLIDER_SKIP_RECENT);
      if collider_end_index >= 1 {
        let start_index = 0;
        let end_index = collider_end_index - 1;

        // Start of the tail collider
        if let Some(start) = positions.get(start_index) {
          gizmos.circle_2d(
            Isometry2d::from_translation(Vec2::new(translation.x + start.x, translation.y + start.y)),
            1.0,
            Color::from(tailwind::RED_400),
          );
        }

        // End of the tail collider
        if let Some(end) = positions.get(end_index) {
          gizmos.circle_2d(
            Isometry2d::from_translation(Vec2::new(translation.x + end.x, translation.y + end.y)),
            1.4,
            Color::from(tailwind::RED_400),
          );
        }
      }
    }
  }
}
