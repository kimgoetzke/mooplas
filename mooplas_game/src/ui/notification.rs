use crate::prelude::UiNotification;
use crate::prelude::constants::{DEFAULT_FONT, SMALL_FONT};
use crate::ui::shared::default_shadow;
use bevy::app::{App, Plugin, Update};
use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::picking::Pickable;
use bevy::prelude::{
  AlignItems, Alpha, Commands, Component, Entity, FontSize, Justify, JustifyContent, LineBreak, MessageReader, Name,
  Node, PositionType, Query, Res, Text, TextBackgroundColor, TextColor, TextFont, TextLayout, Time, Timer, TimerMode,
  With, default, percent, px,
};
use bevy::text::LineHeight;

// A plugin that handles transient UI notifications (e.g. errors or player joined/left messages).
pub struct NotificationPlugin;

impl Plugin for NotificationPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(
      Update,
      (handle_ui_notification_messages, clear_in_game_notification_system),
    );
  }
}

const IN_GAME_NOTIFICATION_DURATION_SECONDS: f32 = 3.0;

/// Marker component for an in-game transient notification.
#[derive(Component)]
struct InGameNotificationRoot;

/// Timer controlling in-game transient notification lifetime.
#[derive(Component)]
struct InGameNotificationTimer(Timer);

fn handle_ui_notification_messages(
  mut commands: Commands,
  mut messages: MessageReader<UiNotification>,
  asset_server: Res<AssetServer>,
  mut notification_query: Query<
    (&mut Text, &mut TextColor, &mut InGameNotificationTimer),
    With<InGameNotificationRoot>,
  >,
) {
  for notification in messages.read() {
    let mut updated_existing_notification = false;
    for (mut text, mut text_colour, mut timer) in &mut notification_query {
      text.0 = notification.text.clone();
      text_colour.0 = notification.colour();
      timer.0.reset();
      updated_existing_notification = true;
    }

    if !updated_existing_notification {
      commands.spawn((
        InGameNotificationRoot,
        Name::new("In-game notification"),
        Node {
          width: percent(100),
          height: px(60),
          bottom: px(24),
          position_type: PositionType::Absolute,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
        },
        Pickable::IGNORE,
        Text::new(format!(" {} ", notification.text.clone())),
        TextFont {
          font: asset_server.load(DEFAULT_FONT).into(),
          font_size: FontSize::Px(SMALL_FONT),
          ..default()
        },
        LineHeight::RelativeToFont(1.5),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        TextColor(notification.colour()),
        TextBackgroundColor::from(Color::BLACK.with_alpha(0.5)),
        default_shadow(),
        InGameNotificationTimer(Timer::from_seconds(
          IN_GAME_NOTIFICATION_DURATION_SECONDS,
          TimerMode::Once,
        )),
      ));
    }
  }
}

// Advances the timer for any in-game notifications and despawns them when the timer has expired.
fn clear_in_game_notification_system(
  mut commands: Commands,
  time: Res<Time>,
  mut notification_query: Query<(Entity, &mut InGameNotificationTimer), With<InGameNotificationRoot>>,
) {
  for (entity, mut timer) in &mut notification_query {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
      commands.entity(entity).despawn();
    }
  }
}

#[cfg(all(test, feature = "online"))]
mod tests {
  use super::*;
  use crate::prelude::UiNotification;
  use bevy::asset::AssetPlugin;
  use bevy::prelude::*;
  use bevy::text::TextPlugin;
  use std::time::Duration;

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default(), TextPlugin));
    app.init_resource::<Time>();
    app.add_message::<UiNotification>();
    app
  }

  #[test]
  fn handle_ui_notification_messages_spawns_in_game_notification() {
    let mut app = setup();
    app.add_systems(Update, handle_ui_notification_messages);

    app
      .world_mut()
      .write_message(UiNotification::info("A player joined the game".to_string()))
      .expect("Failed to write UiNotification");
    app.update();

    let mut query = app.world_mut().query_filtered::<&Text, With<InGameNotificationRoot>>();
    let texts: Vec<_> = query.iter(app.world()).map(|text| text.0.as_str()).collect();
    assert_eq!(texts, vec![" A player joined the game "]);
  }

  #[test]
  fn handle_ui_notification_messages_updates_existing_in_game_notification() {
    let mut app = setup();
    app.add_systems(Update, handle_ui_notification_messages);

    app
      .world_mut()
      .write_message(UiNotification::info("A player joined the game".to_string()))
      .expect("Failed to write UiNotification");
    app.update();
    app
      .world_mut()
      .write_message(UiNotification::info("A player left the game".to_string()))
      .expect("Failed to write UiNotification");
    app.update();

    let mut query = app.world_mut().query_filtered::<&Text, With<InGameNotificationRoot>>();
    let texts: Vec<_> = query.iter(app.world()).map(|text| text.0.as_str()).collect();
    assert_eq!(texts, vec!["A player left the game"]);
  }

  #[test]
  fn tick_in_game_notification_timer_system_despawns_expired_notification() {
    let mut app = setup();
    app.add_systems(
      Update,
      (handle_ui_notification_messages, clear_in_game_notification_system),
    );

    app
      .world_mut()
      .write_message(UiNotification::info("A player joined the game".to_string()))
      .expect("Failed to write UiNotification");
    app.update();
    app
      .world_mut()
      .resource_mut::<Time>()
      .advance_by(Duration::from_secs(4));
    app.update();
    app.update();

    let mut query = app.world_mut().query_filtered::<Entity, With<InGameNotificationRoot>>();
    assert_eq!(query.iter(app.world()).count(), 0);
  }
}
