use bevy::app::{App, Plugin};
use bevy::log::*;

// TODO: Implement a transport for WASM targets
#[allow(dead_code)]
/// Plugin that adds online multiplayer capabilities for WASM targets to the game.
pub struct WasmOnlinePlugin;

impl Plugin for WasmOnlinePlugin {
  fn build(&self, _: &mut App) {
    info!("Online multiplayer for WebAssembly builds is enabled");
  }
}
