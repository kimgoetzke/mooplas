use crate::online::lib::SerialisableInputActionMessage;
use crate::prelude::InputMessage;
use bevy::prelude::{App, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Update};
use bevy_renet::client_connected;

/// A plugin that acts as an interface between local and online functionalities.
pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, handle_input_message.run_if(client_connected));
  }
}

fn handle_input_message(
  mut messages: MessageReader<InputMessage>,
  mut serialisable_input_message: MessageWriter<SerialisableInputActionMessage>,
) {
  for message in messages.read() {
    serialisable_input_message.write(message.into());
  }
}
