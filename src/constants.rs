use bevy::camera::visibility::RenderLayers;
use bevy::color::Srgba;

pub(crate) const DEFAULT_FONT: &str = "fonts/bulkypix.ttf";
pub(crate) const TILES_X: f32 = 5.; // Must result in a whole number when dividing by WORLD_SIZE
pub(crate) const TILES_Y: f32 = 5.; // Must result in a whole number when dividing by WORLD_SIZE
pub(crate) const GRID_MARGIN: f32 = 2.;
pub(crate) const WRAPAROUND_MARGIN: f32 = 0.; // Must be divisible by 2
pub(crate) const HIGH_RES_LAYER: RenderLayers = RenderLayers::layer(0);
pub(crate) const PIXEL_PERFECT_LAYER: RenderLayers = RenderLayers::layer(1);
pub(crate) const RESOLUTION_WIDTH: u32 = 640;
pub(crate) const RESOLUTION_HEIGHT: u32 = 360;
pub(crate) const MOVEMENT_SPEED: f32 = 60.0;
pub(crate) const ROTATION_SPEED: f32 = 120.0;
pub(crate) const BASE_BODY_COLOUR: Srgba = Srgba::new(0.0, 0.419607, 0.4239524, 1.0);
pub(crate) const BODY_WIDTH: f32 = 3.0;
pub(crate) const POSITION_SAMPLE_DISTANCE: f32 = 5.0; // Record position every 5 pixels
pub(crate) const MAX_CONTINUOUS_SNAKE_LENGTH: usize = 100;
pub(crate) const SNAKE_GAP_LENGTH: usize = 15;
pub(crate) const SNAKE_HEAD_SIZE: f32 = 5.0;
