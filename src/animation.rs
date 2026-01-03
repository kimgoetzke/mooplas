use crate::prelude::{AnimationIndices, AnimationTimer};
use bevy::prelude::{App, Plugin, Query, Res, Sprite, Time, Update};

/// Plugin that provides sprite animation functionality.
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, animate_sprite_system);
  }
}

fn animate_sprite_system(time: Res<Time>, mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>) {
  for (indices, mut timer, mut sprite) in &mut query {
    timer.tick(time.delta());
    if timer.just_finished()
      && let Some(atlas) = &mut sprite.texture_atlas
    {
      atlas.index = if atlas.index == indices.last {
        indices.first
      } else {
        atlas.index + 1
      };
    }
  }
}
