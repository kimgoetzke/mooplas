use crate::constants::*;
use crate::shared::{Player, WrapAroundEntity};
use avian2d::math::Vector;
use avian2d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, spawn_player_system).add_systems(
      Update,
      ((
        wraparound_system,
        update_snake_tail_segments_system,
        update_active_segment_collider_system,
        update_active_segment_mesh_system,
      )
        .chain(),),
    );
  }
}

/// A bundle that contains the components needed for a basic kinematic character controller.
#[derive(Bundle)]
struct Controller {
  collider: Collider,
  body: RigidBody,
  ground_caster: ShapeCaster,
  locked_axes: LockedAxes,
}

impl Controller {
  pub fn new(collider: Collider) -> Self {
    let mut caster_shape = collider.clone();
    caster_shape.set_scale(Vector::ONE * 0.99, 10);

    Self {
      collider,
      body: RigidBody::Dynamic,
      ground_caster: ShapeCaster::new(caster_shape, Vector::ZERO, 0.0, Dir2::NEG_Y).with_max_distance(10.0),
      locked_axes: LockedAxes::ROTATION_LOCKED,
    }
  }
}

#[derive(Component)]
struct SnakeSegment {
  positions: Vec<Vec2>,
  mesh_entity: Option<Entity>,
  collider_entity: Option<Entity>,
}

impl Default for SnakeSegment {
  fn default() -> Self {
    Self {
      positions: Vec::with_capacity(MAX_CONTINUOUS_SNAKE_LENGTH),
      mesh_entity: None,
      collider_entity: None,
    }
  }
}

#[derive(Component)]
struct SnakeTail {
  segments: Vec<SnakeSegment>,
  distance_since_last_sample: f32,
  gap_samples_remaining: usize,
}

impl Default for SnakeTail {
  fn default() -> Self {
    Self {
      segments: vec![SnakeSegment::default()],
      distance_since_last_sample: 0.0,
      gap_samples_remaining: 0,
    }
  }
}

#[derive(PhysicsLayer, Default)]
enum GameLayer {
  #[default]
  Default,
  Head,
  Tail,
}

// TODO: Spawn all player related entities under a parent entity for better organisation
/// Spawns the player.
fn spawn_player_system(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
) {
  let starting_position = Vec2::ZERO;
  let snake_head_handle = asset_server.load("player.png");
  commands.spawn((
    Name::new("Snake Head"),
    Player,
    WrapAroundEntity,
    Sprite::from_image(snake_head_handle),
    Controller::new(Collider::circle(SNAKE_HEAD_SIZE)),
    Transform::from_xyz(starting_position.x, starting_position.y, 0.),
    Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
    Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
    CollisionLayers::new(GameLayer::Head, [GameLayer::Default, GameLayer::Tail]),
    PIXEL_PERFECT_LAYER,
  ));
  commands.spawn((
    Name::new("Snake Tail"),
    SnakeTail::default(),
    Transform::from_xyz(starting_position.x, starting_position.y, 0.),
    PIXEL_PERFECT_LAYER,
  ));
  // Spawning round tail for visual reasons; consider replacing with more fancy tail mesh later
  commands.spawn((
    Name::new("Snake Tail End"),
    Mesh2d(meshes.add(Circle::new(BODY_WIDTH))),
    MeshMaterial2d(materials.add(Color::from(BASE_BODY_COLOUR))),
    Transform::from_xyz(starting_position.x, starting_position.y - (BODY_WIDTH * 2.), 1.),
    CollisionLayers::new(GameLayer::Tail, [GameLayer::Head]),
    Collider::circle(SNAKE_HEAD_SIZE / 2.),
    RigidBody::Static,
    PIXEL_PERFECT_LAYER,
  ));
}

/// Samples the player's position and updates the [`SnakeTail`] segments accordingly. Creates mesh and collider
/// entities as needed.
fn update_snake_tail_segments_system(
  mut commands: Commands,
  mut snake_tail_query: Query<&mut SnakeTail, Without<Player>>,
  player_query: Query<&mut Transform, With<Player>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
) {
  let transform = player_query.single().expect("There should be a single player");
  for mut snake_tail in &mut snake_tail_query {
    let gap_samples_remaining = snake_tail.gap_samples_remaining;
    let active_segment_index = snake_tail.segments.len() - 1;
    let is_active_segment_positions_empty = snake_tail.segments[active_segment_index].positions.is_empty();
    let current_position = transform.translation.truncate() - (transform.rotation * Vec3::Y * 5.).truncate();

    // Add the first point and, if required, the mesh if the active segment has no positions yet and there are no gap
    // samples remaining
    if is_active_segment_positions_empty && gap_samples_remaining == 0 {
      snake_tail.distance_since_last_sample = 0.0;
      let active_segment = &mut snake_tail.segments[active_segment_index];
      active_segment.positions.push(current_position);
      create_segment_mesh_if_none_exist(&mut commands, &mut meshes, &mut materials, active_segment);
      continue;
    }

    // Update distance since last sample based distance between last point and current position
    update_distance_since_last_sample(&mut snake_tail, active_segment_index, current_position);

    // Handle logic for when sample distance is reached
    handle_sample_distance_reached(
      &mut commands,
      &mut meshes,
      &mut materials,
      &mut snake_tail,
      active_segment_index,
      current_position,
    );
  }
}

/// Creates a mesh entity for the active segment if none exists.
fn create_segment_mesh_if_none_exist(
  commands: &mut Commands,
  meshes: &mut ResMut<Assets<Mesh>>,
  materials: &mut ResMut<Assets<ColorMaterial>>,
  active_segment: &mut SnakeSegment,
) {
  if active_segment.mesh_entity.is_none() {
    let mesh_entity = commands
      .spawn((
        Name::new("Snake Tail Segment Mesh"),
        Mesh2d(meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default()))),
        MeshMaterial2d(materials.add(Color::from(BASE_BODY_COLOUR))),
        Transform::default(),
        PIXEL_PERFECT_LAYER,
      ))
      .id();
    active_segment.mesh_entity = Some(mesh_entity);
  }
}

/// Updates the distance since the last sample for the active segment based on the distance from the last recorded
/// position to the current position.
fn update_distance_since_last_sample(
  snake_tail: &mut Mut<SnakeTail>,
  active_segment_index: usize,
  current_position: Vec2,
) {
  let last_position = snake_tail.segments[active_segment_index]
    .positions
    .last()
    .copied()
    .unwrap_or(current_position);
  let distance = current_position.distance(last_position);
  snake_tail.distance_since_last_sample += distance;
}

/// Handles the logic for when the distance since the last sample exceeds the defined threshold.
fn handle_sample_distance_reached(
  mut commands: &mut Commands,
  mut meshes: &mut ResMut<Assets<Mesh>>,
  mut materials: &mut ResMut<Assets<ColorMaterial>>,
  snake_tail: &mut Mut<SnakeTail>,
  active_segment_index: usize,
  current_position: Vec2,
) {
  if snake_tail.distance_since_last_sample < POSITION_SAMPLE_DISTANCE {
    return;
  }

  // Reset distance since last sample
  snake_tail.distance_since_last_sample = 0.0;

  // Handle gap samples by decrementing counter and starting new segment if gap is over
  if snake_tail.gap_samples_remaining > 0 {
    snake_tail.gap_samples_remaining -= 1;
    if snake_tail.gap_samples_remaining == 0 {
      // Start a fresh segment after the gap
      snake_tail.segments.push(SnakeSegment::default());
    }
    return;
  }

  // Add current position to active segment and create mesh if needed
  let active_segment = &mut snake_tail.segments[active_segment_index];
  active_segment.positions.push(current_position);
  create_segment_mesh_if_none_exist(&mut commands, &mut meshes, &mut materials, active_segment);

  // Create collider entity in the active segment once we have two points if none exists
  if active_segment.collider_entity.is_none() && active_segment.positions.len() >= 2 {
    let collider = commands
      .spawn((
        Name::new("Snake Tail Segment Collider"),
        RigidBody::Static,
        Collider::polyline(active_segment.positions.clone(), None),
        Transform::default(),
        CollisionLayers::new([GameLayer::Tail], [GameLayer::Head]),
      ))
      .id();
    active_segment.collider_entity = Some(collider);
  }

  // If this segment reached max continuous length, start gap samples
  if active_segment.positions.len() >= MAX_CONTINUOUS_SNAKE_LENGTH {
    snake_tail.gap_samples_remaining = SNAKE_GAP_LENGTH;
  }
}

// Replaces the [`Collider`] of the active (last) [`SnakeSegment`] every time the [`SnakeTail`] changes.
fn update_active_segment_collider_system(
  mut commands: Commands,
  snake_tail_query: Query<&SnakeTail, Changed<SnakeTail>>,
) {
  for snake_tail in &snake_tail_query {
    if snake_tail.segments.is_empty() {
      continue;
    }
    if let Some(active_segment) = snake_tail.segments.last() {
      if active_segment.positions.len() < 2 {
        continue;
      }
      if let Some(collider_entity) = active_segment.collider_entity {
        let vertices: Vec<Vector> = active_segment.positions.iter().map(|p| Vector::new(p.x, p.y)).collect();
        commands
          .entity(collider_entity)
          .insert(Collider::polyline(vertices, None));
      }
    }
  }
}

/// Updates the mesh of the active (last) [`SnakeSegment`] every time the [`SnakeTail`] changes.
fn update_active_segment_mesh_system(
  snake_tail_query: Query<&SnakeTail, (Without<Player>, Changed<SnakeTail>)>,
  mut mesh_query: Query<&mut Mesh2d>,
  mut meshes: ResMut<Assets<Mesh>>,
) {
  for snake_tail in &snake_tail_query {
    if snake_tail.segments.is_empty() {
      continue;
    }
    if let Some(active_segment) = snake_tail.segments.last() {
      if active_segment.positions.len() < 2 {
        continue;
      }
      if let Some(mesh_entity) = active_segment.mesh_entity {
        if let Ok(mesh2d) = mesh_query.get_mut(mesh_entity) {
          if let Some(m) = meshes.get_mut(&mesh2d.0) {
            *m = create_snake_tail_mesh(&active_segment.positions);
          }
        }
      }
    }
  }
}

fn create_snake_tail_mesh(positions: &[Vec2]) -> Mesh {
  if positions.len() < 2 {
    return Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
  }

  let mut vertices = Vec::with_capacity(positions.len() * 2);
  let mut indices = Vec::with_capacity((positions.len() - 1) * 6);

  // Generate vertices along the path
  for (i, &position) in positions.iter().enumerate() {
    let tangent = if i == 0 {
      (positions[1] - positions[0]).normalize_or_zero()
    } else if i == positions.len() - 1 {
      (positions[i] - positions[i - 1]).normalize_or_zero()
    } else {
      ((positions[i + 1] - position) + (position - positions[i - 1])).normalize_or_zero()
    };

    let normal = Vec2::new(-tangent.y, tangent.x);
    vertices.push([
      position.x + normal.x * BODY_WIDTH,
      position.y + normal.y * BODY_WIDTH,
      0.0,
    ]);
    vertices.push([
      position.x - normal.x * BODY_WIDTH,
      position.y - normal.y * BODY_WIDTH,
      0.0,
    ]);
  }

  // Generate indices for triangles
  for i in 0..(positions.len() - 1) {
    let base = (i * 2) as u32;
    indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
  }

  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(Indices::U32(indices))
}

/// Wraps the relevant entities around the screen edges, making them reappear on the opposite side.
fn wraparound_system(
  mut entities: Query<(Entity, &mut Transform, Option<&Player>), With<WrapAroundEntity>>,
  mut snake_tail_query: Query<&mut SnakeTail, Without<Player>>,
) {
  let extents = Vec3::new(RESOLUTION_WIDTH as f32 / 2., RESOLUTION_HEIGHT as f32 / 2., 0.);
  for (_entity, mut transform, player) in entities.iter_mut() {
    let mut was_wrapped = false;

    // Move entity to opposite side if it goes out of bounds and set flag
    if transform.translation.x > (extents.x + WRAPAROUND_MARGIN) {
      transform.translation.x = -extents.x - WRAPAROUND_MARGIN;
      was_wrapped = true;
    } else if transform.translation.x < (-extents.x - WRAPAROUND_MARGIN) {
      transform.translation.x = extents.x + WRAPAROUND_MARGIN;
      was_wrapped = true;
    }
    if transform.translation.y > (extents.y + WRAPAROUND_MARGIN) {
      transform.translation.y = -extents.y - WRAPAROUND_MARGIN;
      was_wrapped = true;
    } else if transform.translation.y < (-extents.y - WRAPAROUND_MARGIN) {
      transform.translation.y = extents.y + WRAPAROUND_MARGIN;
      was_wrapped = true;
    }

    // If flagged, and this entity is a player, stop tail growth and start a gap
    if was_wrapped && player.is_some() {
      for mut snake_tail in &mut snake_tail_query {
        snake_tail.gap_samples_remaining = SNAKE_GAP_LENGTH / 2;
        snake_tail.distance_since_last_sample = 0.0;
      }
    }
  }
}
