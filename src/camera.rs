use crate::constants::*;
use bevy::app::{App, Plugin, Startup, Update};
use bevy::asset::Assets;
use bevy::camera::RenderTarget;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::WindowResized;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, setup_camera_system)
      .add_systems(Update, fit_canvas_system);
  }
}

#[derive(Component)]
struct Canvas;

#[derive(Component)]
struct InGameCamera; // Cameras rendering on `PIXEL_PERFECT_LAYER`

#[derive(Component)]
struct OuterCamera; // Camera rendering `HIGH_RES_LAYER`

fn setup_camera_system(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
  let canvas_size = Extent3d {
    width: RESOLUTION_WIDTH,
    height: RESOLUTION_HEIGHT,
    ..default()
  };

  // This image serves as a canvas representing the low-resolution game screen
  let mut canvas = Image {
    texture_descriptor: TextureDescriptor {
      label: None,
      size: canvas_size,
      dimension: TextureDimension::D2,
      format: TextureFormat::Bgra8UnormSrgb,
      mip_level_count: 1,
      sample_count: 1,
      usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT,
      view_formats: &[],
    },
    ..default()
  };

  canvas.resize(canvas_size);
  let image_handle = images.add(canvas);
  commands.spawn((
    Camera2d,
    Camera {
      // Render before the "main pass" camera
      order: -1,
      target: RenderTarget::Image(image_handle.clone().into()),
      clear_color: ClearColorConfig::Custom(Color::from(tailwind::NEUTRAL_950)),
      ..default()
    },
    Msaa::Off,
    InGameCamera,
    PIXEL_PERFECT_LAYER,
  ));

  commands.spawn((Sprite::from_image(image_handle), Canvas, HIGH_RES_LAYER));
  commands.spawn((Camera2d, Msaa::Off, OuterCamera, HIGH_RES_LAYER));
}

// Scales camera projection to fit the window (integer multiples only for pixel-perfect rendering)
fn fit_canvas_system(
  mut resize_messages: MessageReader<WindowResized>,
  mut projection: Single<&mut Projection, With<OuterCamera>>,
) {
  let Projection::Orthographic(projection) = &mut **projection else {
    return;
  };
  for window_resized in resize_messages.read() {
    let h_scale = window_resized.width / RESOLUTION_WIDTH as f32;
    let v_scale = window_resized.height / RESOLUTION_HEIGHT as f32;
    projection.scale = 1. / h_scale.min(v_scale).round();
  }
}
