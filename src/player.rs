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
        update_snake_tail_system,
        update_snake_tail_collider_system,
        update_snake_tail_mesh_system,
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
struct SnakeTail {
  positions: Vec<Vec2>,
  distance_since_last_sample: f32,
  collider_entity: Option<Entity>,
}

impl SnakeTail {
  fn len(&self) -> usize {
    self.positions.len()
  }
}

impl Default for SnakeTail {
  fn default() -> Self {
    Self {
      positions: Vec::with_capacity(10000),
      distance_since_last_sample: 0.0,
      collider_entity: None,
    }
  }
}

#[derive(Component)]
struct SnakeBodyCollider;

#[derive(PhysicsLayer, Default)]
enum GameLayer {
  #[default]
  Default,
  Head,
  Tail,
}

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
    Mesh2d(meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default()))),
    MeshMaterial2d(materials.add(Color::from(BASE_BODY_COLOUR))),
    Transform::from_xyz(starting_position.x, starting_position.y, 0.),
    RigidBody::Static,
    CollisionLayers::new(GameLayer::Tail, [GameLayer::Head]),
    PIXEL_PERFECT_LAYER,
  ));
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

fn update_snake_tail_system(
  mut commands: Commands,
  mut snake_tail_query: Query<&mut SnakeTail, Without<Player>>,
  player_query: Query<&Transform, With<Player>>,
) {
  let transform = player_query.single().expect("There should be a single player");
  for mut snake_tail in &mut snake_tail_query {
    if snake_tail.len() >= MAX_CONTINUOUS_SNAKE_LENGTH + SNAKE_GAP_LENGTH {
      // TODO: Create new body
    }

    if snake_tail.len() >= MAX_CONTINUOUS_SNAKE_LENGTH {
      continue;
    }

    let current_position = transform.translation.truncate() - (transform.rotation * Vec3::Y * 5.).truncate();
    if snake_tail.positions.is_empty() {
      snake_tail.positions.push(current_position);
      return;
    }

    let last_position = *snake_tail
      .positions
      .last()
      .expect("There should be at least one position");
    let distance = current_position.distance(last_position);
    snake_tail.distance_since_last_sample += distance;

    if snake_tail.distance_since_last_sample >= POSITION_SAMPLE_DISTANCE {
      snake_tail.positions.push(current_position);
      snake_tail.distance_since_last_sample = 0.0;

      if snake_tail.collider_entity.is_none() && snake_tail.positions.len() >= 2 {
        debug!(
          "Creating snake body collider with {} positions",
          snake_tail.positions.len()
        );
        let collider = commands
          .spawn((
            Name::new("Snake Body Collider"),
            SnakeBodyCollider,
            RigidBody::Static,
            Collider::polyline(snake_tail.positions.clone(), None),
            Transform::default(),
            CollisionLayers::new([GameLayer::Tail], [GameLayer::Head]),
          ))
          .id();
        snake_tail.collider_entity = Some(collider);
      }
    }
  }
}

fn update_snake_tail_collider_system(mut commands: Commands, player_query: Query<&SnakeTail, Changed<SnakeTail>>) {
  for snake_tail in &player_query {
    if snake_tail.positions.len() < 2 {
      continue;
    }

    if let Some(collider_entity) = snake_tail.collider_entity {
      let vertices: Vec<Vector> = snake_tail.positions.iter().map(|p| Vector::new(p.x, p.y)).collect();
      commands
        .entity(collider_entity)
        .insert(Collider::polyline(vertices, None));
    }
  }
}

fn update_snake_tail_mesh_system(
  player_query: Query<(&mut Mesh2d, &SnakeTail), (Without<Player>, Changed<SnakeTail>)>,
  mut meshes: ResMut<Assets<Mesh>>,
) {
  for (mesh, snake_tail) in &player_query {
    if snake_tail.positions.len() < 2 {
      continue;
    }

    if let Some(mesh) = meshes.get_mut(&mesh.0) {
      *mesh = create_snake_tail_mesh(&snake_tail.positions);
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
