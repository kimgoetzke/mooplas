use crate::app_states::AppState;
use crate::prelude::constants::{DEFAULT_FONT, NORMAL_FONT, PIXEL_PERFECT_LAYER, RESOLUTION_HEIGHT, RESOLUTION_WIDTH};
use crate::shared::CustomInteraction;
use crate::ui::spawn_button;
use bevy::app::{App, Plugin};
use bevy::color::palettes::tailwind;
use bevy::log::*;
use bevy::prelude::*;

/// Plugin that provides and manages the main menu UI.
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(OnEnter(AppState::Preparing), spawn_main_menu_system)
      .add_systems(
        Update,
        handle_main_menu_buttons_system.run_if(in_state(AppState::Preparing)),
      )
      .add_systems(OnExit(AppState::Preparing), despawn_main_menu_system);
  }
}

/// Marker component for the root of the main menu. Used for despawning.
#[derive(Component)]
struct MainMenuRoot;

/// Marker component for the Play Online button in the main menu.
#[derive(Component)]
struct PlayOnlineButton;

/// Marker component for the Play Local button in the main menu.
#[derive(Component)]
struct PlayLocalButton;

/// Marker component for the Exit button in the main menu.
#[derive(Component)]
struct ExitButton;

/// System to spawn the main menu UI including the background image.
fn spawn_main_menu_system(mut commands: Commands, asset_server: Res<AssetServer>) {
  let font = asset_server.load(DEFAULT_FONT);
  let heading_font = font.clone();
  let background_image = asset_server.load("images/background_menu_main.png");

  // Background
  commands.spawn((
    Name::new("Main Menu Background"),
    MainMenuRoot,
    Sprite {
      image: background_image.clone(),
      custom_size: Some(Vec2::new(RESOLUTION_WIDTH as f32, RESOLUTION_HEIGHT as f32)),
      ..default()
    },
    Transform::from_xyz(0., 0., -1.),
    PIXEL_PERFECT_LAYER,
  ));

  // Main Menu UI
  commands
    .spawn((
      Name::new("Main Menu"),
      MainMenuRoot,
      Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        position_type: PositionType::Relative,
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
    ))
    .with_children(|parent| {
      parent
        .spawn(Node {
          width: Val::Percent(100.0),
          height: Val::Percent(100.0),
          flex_direction: FlexDirection::Column,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          row_gap: Val::Px(20.),
          ..default()
        })
        .with_children(|parent| {
          // Title
          parent.spawn((
            Text::new("Mooplas"),
            TextFont {
              font: heading_font.clone(),
              font_size: 110.,
              ..default()
            },
            TextColor(Color::from(tailwind::AMBER_300)),
            TextShadow::default(),
          ));

          // Buttons
          parent
            .spawn(Node {
              flex_direction: FlexDirection::Column,
              justify_content: JustifyContent::Center,
              align_items: AlignItems::Center,
              row_gap: Val::Px(20.),
              ..default()
            })
            .with_children(|parent| {
              spawn_button(parent, &asset_server, PlayLocalButton, "Play Local", 300, NORMAL_FONT);
              spawn_button(parent, &asset_server, PlayOnlineButton, "Play Online", 300, NORMAL_FONT);
              spawn_button(parent, &asset_server, ExitButton, "Exit", 300, NORMAL_FONT);
            });
        });
    });
}

/// System to handle all main menu button interactions.
fn handle_main_menu_buttons_system(
  mut commands: Commands,
  mut exit_button_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<ExitButton>)>,
  mut play_local_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<PlayLocalButton>)>,
  mut play_online_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<PlayOnlineButton>)>,
  menu_root_query: Query<Entity, With<MainMenuRoot>>,
  mut next_state: ResMut<NextState<AppState>>,
) {
  for interaction in &mut exit_button_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Exit\"");
      std::process::exit(0);
    }
  }

  for interaction in &mut play_local_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Play Local\"");
      next_state.set(AppState::Registering);
      for root in &menu_root_query {
        commands.entity(root).despawn();
      }
    }
  }

  for interaction in &mut play_online_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Play Online\" -> No-op for now");
    }
  }
}

/// Despawns all elements with the [`MainMenuRoot`] component.
fn despawn_main_menu_system(mut commands: Commands, menu_root_query: Query<Entity, With<MainMenuRoot>>) {
  for root in &menu_root_query {
    commands.entity(root).despawn();
  }
}
