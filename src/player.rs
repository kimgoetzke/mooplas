use crate::constants::*;
use avian2d::math::{AdjustPrecision, Scalar, Vector};
use avian2d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::tailwind;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<InputAction>()
      .add_systems(Startup, spawn_player_system)
      .add_systems(
        Update,
        (
          keyboard_input_system,
          movement_system,
          wraparound_system,
          render_gizmos_system,
          (update_snake_body_system, refresh_snake_mesh_system).chain(),
        ),
      );
  }
}

/// A bundle that contains the components needed for a basic kinematic character controller.
#[derive(Bundle)]
struct Controller {
  body: RigidBody,
  collider: Collider,
  ground_caster: ShapeCaster,
  locked_axes: LockedAxes,
}

impl Controller {
  pub fn new(collider: Collider) -> Self {
    let mut caster_shape = collider.clone();
    caster_shape.set_scale(Vector::ONE * 0.99, 10);

    Self {
      body: RigidBody::Dynamic,
      collider,
      ground_caster: ShapeCaster::new(caster_shape, Vector::ZERO, 0.0, Dir2::NEG_Y).with_max_distance(10.0),
      locked_axes: LockedAxes::ROTATION_LOCKED,
    }
  }
}

/// A marker component for the player entity.
#[derive(Component)]
struct Player;

/// A marker component for entities that should wrap around the screen edges.
#[derive(Component)]
struct WrapAroundEntity;

/// A [`Message`] written for an input action.
#[derive(Message)]
enum InputAction {
  Move(Scalar),
  Action,
}

#[derive(Component)]
struct SnakeBody {
  positions: Vec<Vec2>,
  distance_since_last_sample: f32,
  segment_entities: Vec<Entity>,
}

#[derive(Component)]
struct SnakeBodySegment;

impl Default for SnakeBody {
  fn default() -> Self {
    Self {
      positions: Vec::with_capacity(10000),
      distance_since_last_sample: 0.0,
      segment_entities: Vec::with_capacity(10000),
    }
  }
}

/// A marker component for the snake body mesh.
#[derive(Component)]
struct SnakeTail;

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
  let snake_head_handle = asset_server.load("player.png");
  commands.spawn((
    Name::new("Snake Head"),
    Player,
    WrapAroundEntity,
    Sprite::from_image(snake_head_handle),
    SnakeBody::default(),
    Controller::new(Collider::circle(5.)),
    Transform::default(),
    Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
    Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
    ColliderDensity::default(),
    CollisionLayers::new(GameLayer::Head, [GameLayer::Default, GameLayer::Tail]),
    PIXEL_PERFECT_LAYER,
  ));
  commands.spawn((
    Player,
    Name::new("Snake Tail"),
    SnakeTail,
    Mesh2d(meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default()))),
    MeshMaterial2d(materials.add(Color::from(BASE_BODY_COLOUR))),
    Transform::default(),
    CollisionLayers::new(GameLayer::Tail, [GameLayer::Head]),
    PIXEL_PERFECT_LAYER,
  ));
}

fn render_gizmos_system(mut gizmos: Gizmos, player_query: Query<&Transform, With<SnakeBody>>) {
  for transform in player_query.iter() {
    gizmos.circle_2d(
      Isometry2d::from_translation(Vector::new(transform.translation.x, transform.translation.y)),
      1.,
      Color::from(tailwind::AMBER_400),
    );
  }
}

/// Sends [`InputAction`] events based on keyboard input.
fn keyboard_input_system(
  mut input_action_writer: MessageWriter<InputAction>,
  keyboard_input: Res<ButtonInput<KeyCode>>,
) {
  let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
  let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
  let horizontal = right as i8 - left as i8;
  let direction = horizontal as Scalar;
  if direction != 0.0 {
    input_action_writer.write(InputAction::Move(direction));
  }
  if keyboard_input.just_pressed(KeyCode::Space) {
    input_action_writer.write(InputAction::Action);
  }
}

/// Responds to [`InputAction`] events and moves character controllers accordingly.
fn movement_system(
  time: Res<Time>,
  mut input_action_messages: MessageReader<InputAction>,
  mut controllers: Query<(&Transform, &mut LinearVelocity, &mut AngularVelocity), With<Player>>,
) {
  let delta_time = time.delta_secs_f64().adjust_precision();
  for (transform, mut linear_velocity, mut angular_velocity) in &mut controllers {
    let mut has_movement_input = false;
    let direction = (transform.rotation * Vec3::Y).normalize_or_zero();
    let velocity = direction * MOVEMENT_SPEED;
    linear_velocity.x = velocity.x;
    linear_velocity.y = velocity.y;

    for event in input_action_messages.read() {
      has_movement_input = true;
      match event {
        InputAction::Move(direction) => {
          angular_velocity.0 = -*direction * ROTATION_SPEED * delta_time;
        }
        InputAction::Action => {
          debug!("[Not implemented] Action received");
        }
      }
    }
    if !has_movement_input {
      angular_velocity.0 = 0.;
    }
  }
}

/// Wraps the relevant entities around the screen edges, making them reappear on the opposite side.
fn wraparound_system(mut entities: Query<&mut Transform, With<WrapAroundEntity>>) {
  let extents = Vec3::new(RESOLUTION_WIDTH as f32 / 2., RESOLUTION_HEIGHT as f32 / 2., 0.);
  for mut transform in entities.iter_mut() {
    if transform.translation.x > (extents.x + WRAPAROUND_MARGIN) {
      transform.translation.x = -extents.x - WRAPAROUND_MARGIN;
    } else if transform.translation.x < (-extents.x - WRAPAROUND_MARGIN) {
      transform.translation.x = extents.x + WRAPAROUND_MARGIN;
    }
    if transform.translation.y > (extents.y + WRAPAROUND_MARGIN) {
      transform.translation.y = -extents.y - WRAPAROUND_MARGIN;
    } else if transform.translation.y < (-extents.y - WRAPAROUND_MARGIN) {
      transform.translation.y = extents.y + WRAPAROUND_MARGIN;
    }
  }
}

fn update_snake_body_system(
  mut commands: Commands,
  mut player_query: Query<(&Transform, &mut SnakeBody), With<Player>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
) {
  for (transform, mut body) in &mut player_query {
    let current_position = transform.translation.truncate() - (transform.rotation * Vec3::Y * 10.).truncate();

    if body.positions.is_empty() {
      body.positions.push(current_position);
      return;
    }

    let last_position = *body.positions.last().unwrap();
    let distance = current_position.distance(last_position);
    body.distance_since_last_sample += distance;

    if body.distance_since_last_sample >= POSITION_SAMPLE_DISTANCE {
      body.positions.push(current_position);
      body.distance_since_last_sample = 0.0;

      let segment = commands
        .spawn((
          Name::new("Snake Body Segment"),
          SnakeBodySegment,
          Transform::from_translation(current_position.extend(0.0)),
          RigidBody::Static,
          Collider::circle(BODY_WIDTH / 2.),
          CollisionLayers::new(GameLayer::Tail, [GameLayer::Head]),
          Mesh2d(meshes.add(Circle::new(BODY_WIDTH))),
          MeshMaterial2d(materials.add(Color::from(BASE_BODY_COLOUR))),
          PIXEL_PERFECT_LAYER,
        ))
        .id();

      body.segment_entities.push(segment);
    }
  }
}

fn refresh_snake_mesh_system(
  player_query: Query<&SnakeBody, (With<Player>, Changed<SnakeBody>)>,
  mut mesh_query: Query<&mut Mesh2d, With<SnakeTail>>,
  mut meshes: ResMut<Assets<Mesh>>,
) {
  // for body in &player_query {
  //   if body.positions.len() < 2 {
  //     continue;
  //   }
  //
  //   for mesh_handle in &mut mesh_query {
  //     if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
  //       *mesh = create_snake_body_mesh(&body.positions);
  //     }
  //   }
  // }
}

fn create_snake_body_mesh(positions: &[Vec2]) -> Mesh {
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
