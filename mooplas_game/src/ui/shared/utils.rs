use bevy::prelude::{Children, Commands, Component, Entity, Query, With};

/// Despawns the provided children.
pub(crate) fn despawn_children(commands: &mut Commands, children: &Children) {
  for child in children.iter() {
    commands.entity(*child).despawn();
  }
}

/// Despawns the component for the provided menu.
pub(crate) fn despawn_menu(commands: &mut Commands, marker_component_query: &Query<Entity, With<impl Component>>) {
  for root in marker_component_query {
    commands.entity(root).despawn();
  }
}
