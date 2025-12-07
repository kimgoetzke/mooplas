use crate::online::lib::SerialisableInputAction;
use crate::prelude::{InputAction, PlayerId};
use crate::shared::Player;
use bevy::log::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::{
  Add, App, Entity, IntoScheduleConfigs, IntoSystem, MessageReader, MessageWriter, Name, Observer, On, Plugin, Query,
  Remove, ResMut, Resource, Update, With,
};
use bevy_renet::client_connected;

/// A plugin that acts as an interface between local and online functionalities.
pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<PlayerIndex>()
      .add_systems(Update, handle_input_action_message.run_if(client_connected))
      .world_mut()
      .spawn_batch([
        (
          Observer::new(IntoSystem::into_system(on_add_player_trigger)),
          Name::new("Observer: Add Player"),
        ),
        (
          Observer::new(IntoSystem::into_system(on_remove_player_trigger)),
          Name::new("Observer: Remove Player"),
        ),
      ]);
  }
}

/// Contains all players that currently exists in the world. This index is kept up-to-date by observing the [`OnAdd`]
/// and [`OnRemove`] triggers.
#[derive(Resource, Default)]
pub struct PlayerIndex {
  map: HashMap<Entity, PlayerId>,
}

impl PlayerIndex {
  pub fn get(&self, entity: &Entity) -> Option<&PlayerId> {
    if let Some(entity) = self.map.get(entity) {
      Some(entity)
    } else {
      None
    }
  }

  pub fn size(&self) -> usize {
    self.map.len()
  }
}

fn on_add_player_trigger(
  trigger: On<Add, Player>,
  query: Query<(Entity, &PlayerId), With<Player>>,
  mut index: ResMut<PlayerIndex>,
) {
  let (entity, player_id) = query.get(trigger.entity).expect("Failed to fetch player from index");
  index.map.insert(entity, *player_id);
  debug!("PlayerIndex <- Added [{:?}] with key [{}]", player_id, entity);
}

fn on_remove_player_trigger(
  trigger: On<Remove, Player>,
  query: Query<(Entity, &PlayerId), With<Player>>,
  mut index: ResMut<PlayerIndex>,
) {
  let (entity, player_id) = query.get(trigger.entity).expect("Failed to fetch player from index");
  let result = index.map.remove(&entity);
  if result.is_none() {
    warn!(
      "PlayerIndex -> Tried to remove [{:?}] with key [{}] but it did not exist in the index",
      player_id, entity
    );
  }
  debug!("PlayerIndex -> Removed [{:?}] with key [{}]", player_id, entity);
}

fn handle_input_action_message(
  mut messages: MessageReader<InputAction>,
  mut serialisable_input_action_writer: MessageWriter<SerialisableInputAction>,
) {
  for message in messages.read() {
    serialisable_input_action_writer.write(message.into());
  }
}
