use crate::constants::SNAKE_HEAD_SIZE;
use crate::player::SnakeBody;
use avian2d::math::Vector;
use bevy::app::{App, Plugin, Update};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::math::Isometry2d;
use bevy::prelude::{Gizmos, Query, Transform, With};

/// A plugin that renders gizmos for debugging purposes.
pub struct GizmosPlugin;

impl Plugin for GizmosPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, (render_gizmos_system,));
  }
}

fn render_gizmos_system(mut gizmos: Gizmos, player_query: Query<&Transform, With<SnakeBody>>) {
  for transform in player_query.iter() {
    gizmos.circle_2d(
      Isometry2d::from_translation(Vector::new(transform.translation.x, transform.translation.y)),
      SNAKE_HEAD_SIZE / 2.,
      Color::from(tailwind::AMBER_400),
    );

    gizmos.circle_2d(
      Isometry2d::from_translation(Vector::new(transform.translation.x, transform.translation.y)),
      0.5,
      Color::from(tailwind::AMBER_400),
    );
  }
}
