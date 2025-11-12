use crate::constants::*;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy::text::FontSmoothing;

/// Plugin that creates the game world. Only has a visual effect.
pub struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, create_world_system);
  }
}

/// Creates the game world as a grid of tiles with labels.
fn create_world_system(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
  asset_server: Res<AssetServer>,
) {
  let tile_size_x = (RESOLUTION_WIDTH as f32 - GRID_MARGIN) / TILES_X;
  let tile_size_y = (RESOLUTION_HEIGHT as f32 - GRID_MARGIN) / TILES_Y;
  let adjusted_tile_size_x = tile_size_x - GRID_MARGIN;
  let adjusted_tile_size_y = tile_size_y - GRID_MARGIN;
  let half_world_x = RESOLUTION_WIDTH as f32 / 2.;
  let half_world_y = RESOLUTION_HEIGHT as f32 / 2.;
  let half_margin = GRID_MARGIN / 2.;
  let parent = commands.spawn((Name::new("Game World"), Transform::default())).id();

  for i in 0..TILES_X as i32 {
    for j in (0..TILES_Y as i32).rev() {
      let x = (i as f32 * tile_size_x) - half_world_x + (tile_size_x / 2.);
      let y = (j as f32 * tile_size_y) - half_world_y + (tile_size_y / 2.);
      let letter = (b'A' + i as u8) as char;
      let description = format!("{}{}", letter, TILES_Y as i32 - j);
      let tile_color = determine_tile_colour(i, j);

      commands.entity(parent).with_children(|parent| {
        parent
          .spawn((
            Mesh2d(meshes.add(Rectangle::new(adjusted_tile_size_x, adjusted_tile_size_y))),
            MeshMaterial2d(materials.add(tile_color)),
            Transform::from_xyz(x + half_margin, y + half_margin, -999.),
            Name::new(description.clone()),
            PIXEL_PERFECT_LAYER,
          ))
          .with_children(|builder| {
            builder.spawn((
              Name::new("Text"),
              Text2d::new(description),
              TextFont {
                font_size: 18.,
                font: asset_server.load(DEFAULT_FONT),
                font_smoothing: FontSmoothing::None,
                ..default()
              },
              TextColor(Color::from(tailwind::NEUTRAL_600)),
              TextLayout::new(Justify::Center, LineBreak::AnyCharacter),
              PIXEL_PERFECT_LAYER,
            ));
          });
      });
    }
  }
  debug!("✅  Game world creation completed");
}

/// Determines the colour of a tile based on its grid coordinates. Used to create a checkerboard pattern.
fn determine_tile_colour(i: i32, j: i32) -> Color {
  if ((i + j) % 2) == 0 {
    Color::from(tailwind::NEUTRAL_800)
  } else {
    Color::from(tailwind::NEUTRAL_700)
  }
}
