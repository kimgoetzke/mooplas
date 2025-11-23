use bevy::camera::visibility::RenderLayers;

// General and rendering
// --------------------------------//

/// The target resolution width for the pixel-perfect camera.
pub(crate) const RESOLUTION_WIDTH: u32 = 640;

/// The target resolution height for the pixel-perfect camera.
pub(crate) const RESOLUTION_HEIGHT: u32 = 360;

/// Render layer for high-resolution elements (UI, effects, etc.).
pub(crate) const HIGH_RES_LAYER: RenderLayers = RenderLayers::layer(0);

/// Render layer for pixel-perfect elements (game world, sprites, etc.).
pub(crate) const PIXEL_PERFECT_LAYER: RenderLayers = RenderLayers::layer(1);

/// The path to the default font used in the game.
pub(crate) const DEFAULT_FONT: &str = "fonts/Tiny5.ttf";

// Game world
// --------------------------------//

/// The number of tiles along the X axis in the game world grid.
pub(crate) const GRID_TILES_X: f32 = 5.;

/// The number of tiles along the Y axis in the game world grid.
pub(crate) const GRID_TILES_Y: f32 = 5.;

/// The margin between tiles and towards the screen edges in the game world grid.
pub(crate) const GRID_MARGIN: f32 = 2.;

/// The margin from the screen edges for spawn points.
pub(crate) const EDGE_MARGIN: f32 = 75.0;

// Controls and movement
// --------------------------------//

/// The movement speed of the player-controlled snake (units per second).
pub(crate) const MOVEMENT_SPEED: f32 = 60.0;

/// The rotation speed of the player-controlled snake.
pub(crate) const ROTATION_SPEED: f32 = 120.0;

// Snake and gameplay constants
// --------------------------------//

/// The margin beyond the screen edges for wraparound behavior to occur. Allows adjusting the point at which wraparound
/// happens. The larger the margin, the further the wraparound entity can travel off-screen before reappearing on the
/// opposite side.
pub(crate) const WRAPAROUND_MARGIN: f32 = 2.; // Must be divisible by 2

/// The length (in pixel) to which the snake tail needs to grow before certain logic is applied, such as introducing
/// gaps.
pub(crate) const SNAKE_TAIL_POSITION_SAMPLE_DISTANCE: f32 = 5.0;

/// The maximum continuous length of the snake body before a gap is introduced, measured in "samples".
pub(crate) const SNAKE_LENGTH_MAX_CONTINUOUS: usize = 100;

/// Half of the width of the snake body mesh. The full width will be double this value.
pub(crate) const SNAKE_BODY_WIDTH: f32 = 2.0;

/// The size of the gap in the snake body mesh, measured in "samples".
pub(crate) const SNAKE_GAP_LENGTH: usize = 15;

/// The radius of the snake head collider.
pub(crate) const SNAKE_HEAD_SIZE: f32 = 3.5;

/// Number of most-recent sampled positions (closest to the head) that are *not* included in the tail collider polyline.
/// This creates a "safe" buffer directly behind the head so that the head does not immediately collide with its own
/// tail.
pub(crate) const TAIL_COLLIDER_SKIP_RECENT: usize = 2;

// UI and touch controls
// --------------------------------//

pub(crate) const BUTTON_ALPHA_DEFAULT: f32 = 0.3;
pub(crate) const BUTTON_ALPHA_PRESSED: f32 = 0.8;
pub(crate) const BUTTON_WIDTH: f32 = 60.0;
pub(crate) const BUTTON_HEIGHT: f32 = 50.0;
pub(crate) const BUTTON_MARGIN: f32 = 15.0;
pub(crate) const BUTTON_BORDER_WIDTH: f32 = 2.0;
