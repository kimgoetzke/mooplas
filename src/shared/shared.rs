use bevy::prelude::Component;

/// A marker component for the player entity.
#[derive(Component)]
pub struct Player;

/// A marker component for the player entity.
#[derive(Component)]
pub struct SnakeHead;

/// A marker component for entities that should wrap around the screen edges.
#[derive(Component)]
pub struct WrapAroundEntity;

/// A component identifying which player an entity belongs to. Used to route input and per-player logic, etc.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlayerId(pub u8);
