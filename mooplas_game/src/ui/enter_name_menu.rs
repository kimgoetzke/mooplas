#![cfg(feature = "online")]

use crate::app_state::AppState;
use crate::prelude::constants::{ACCENT_COLOUR, BUTTON_ALPHA_DEFAULT, DEFAULT_FONT, NORMAL_FONT, TEXT_COLOUR};
use crate::prelude::{CustomInteraction, MenuName, PlayerName};
use crate::shared::ToggleMenuMessage;
use crate::shared::constants::SMALL_FONT;
use crate::ui::shared::{
  BackgroundRoot, despawn_menu, menu_base_node, spawn_background_if_not_exists, spawn_button, spawn_logo,
};
use bevy::app::{App, Plugin};
use bevy::asset::{AssetServer, Assets};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::image::TextureAtlasLayout;
use bevy::input_focus::AutoFocus;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::log::debug;
use bevy::prelude::{
  AlignItems, Alpha, BackgroundColor, BorderColor, BorderRadius, Changed, Commands, Component, Entity, FlexDirection,
  IntoScheduleConfigs, Justify, JustifyContent, MessageReader, MessageWriter, Name, Node, OnExit, Query, Res, ResMut,
  Text, TextColor, TextFont, TextLayout, TextShadow, UiRect, Update, With, default, in_state, percent, px,
};
use bevy::text::{EditableText, FontSize, LineHeight, TextCursorStyle};

/// A plugin to manage the name entry menu UI. Players choose their name before entering
/// the play online menu.
pub struct EnterNameMenuPlugin;

impl Plugin for EnterNameMenuPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        (handle_toggle_menu_message, handle_button_interactions_system)
          .chain()
          .run_if(in_state(AppState::Preparing)),
      )
      .add_systems(OnExit(AppState::Preparing), despawn_menu_system);
  }
}

/// Marker component for the root of the menu. Used for despawning.
#[derive(Component)]
struct EnterNameMenuRoot;

/// Marker component for the continue button.
#[derive(Component)]
struct ContinueButton;

/// Marker component for the back button.
#[derive(Component)]
struct BackButton;

/// Marker component for the name input field.
#[derive(Component)]
struct NameInputField;

/// System to handle toggling the enter name menu based on received messages.
fn handle_toggle_menu_message(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut messages: MessageReader<ToggleMenuMessage>,
  menu_root_query: Query<Entity, With<EnterNameMenuRoot>>,
  background_root_query: Query<Entity, With<BackgroundRoot>>,
  mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
  player_name: Res<PlayerName>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::EnterNameMenu => spawn_menu(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        background_root_query,
        &player_name,
      ),
      _ => despawn_menu(&mut commands, &menu_root_query),
    }
  }
}

fn spawn_menu(
  commands: &mut Commands,
  asset_server: &AssetServer,
  texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
  background_root_query: Query<Entity, With<BackgroundRoot>>,
  player_name: &PlayerName,
) {
  let font = asset_server.load(DEFAULT_FONT);
  let heading_font = font.clone();
  let background_image = asset_server.load("images/background.png");
  let logo_image = asset_server.load("images/logo_animated.png");

  // Background & logo
  spawn_background_if_not_exists(
    commands,
    BackgroundRoot,
    background_image,
    texture_atlas_layouts,
    background_root_query,
  );
  spawn_logo(commands, EnterNameMenuRoot, logo_image, texture_atlas_layouts);

  let initial_name = player_name.get().to_string();
  let mut name_input = EditableText::new(initial_name);
  name_input.max_characters = Some(8);
  name_input.visible_width = Some(8.);

  // Enter name UI
  commands
    .spawn(menu_base_node(EnterNameMenuRoot, "Enter Name Menu".to_string()))
    .with_children(|parent| {
      parent
        .spawn(Node {
          flex_direction: FlexDirection::Column,
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          width: percent(75),
          row_gap: px(20.),
          ..default()
        })
        .with_children(|parent| {
          // Prompt text
          parent.spawn((
            Text::new("Choose your name (max. 8 characters):"),
            TextFont {
              font: heading_font.clone().into(),
              font_size: FontSize::Px(SMALL_FONT),
              ..default()
            },
            TEXT_COLOUR,
            TextShadow::default(),
          ));

          // Name text input field, pre-populated with random name
          parent.spawn((
            Name::new("Name Input Field"),
            NameInputField,
            name_input,
            AutoFocus,
            TabIndex(0),
            TextLayout::no_wrap().with_justify(Justify::Center),
            TextFont {
              font: font.into(),
              font_size: FontSize::Px(NORMAL_FONT),
              ..Default::default()
            },
            TextColor(Color::from(ACCENT_COLOUR)),
            TextCursorStyle::default(),
            BorderColor::all(Color::from(tailwind::SLATE_500)),
            BackgroundColor(Color::from(tailwind::SLATE_500.with_alpha(BUTTON_ALPHA_DEFAULT))),
            Node {
              width: percent(100),
              height: px(55.),
              padding: UiRect::all(px(10.)),
              align_items: AlignItems::Center,
              justify_content: JustifyContent::Center,
              border_radius: BorderRadius::all(px(10)),
              ..default()
            },
          ));

          // Info note
          parent.spawn((
            Text::new("To change your name later, you need to restart Mooplas."),
            TextFont {
              font: heading_font.into(),
              font_size: FontSize::Px(SMALL_FONT),
              ..default()
            },
            LineHeight::RelativeToFont(2.),
            TEXT_COLOUR,
            TextShadow::default(),
          ));

          // Continue button
          spawn_button(parent, asset_server, ContinueButton, "Continue", 300, NORMAL_FONT);

          // Back button
          spawn_button(parent, asset_server, BackButton, "Back", 300, NORMAL_FONT);
        });
    });
}

/// System to handle button interactions in the enter name menu.
fn handle_button_interactions_system(
  mut continue_button_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<ContinueButton>)>,
  mut back_button_query: Query<&CustomInteraction, (Changed<CustomInteraction>, With<BackButton>)>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
  name_input_query: Query<&EditableText, With<NameInputField>>,
  mut player_name: ResMut<PlayerName>,
) {
  for interaction in &mut continue_button_query {
    if *interaction == CustomInteraction::Released {
      if let Ok(input_buffer) = name_input_query.single() {
        let name = input_buffer.value().to_string().trim().to_string();
        if !name.is_empty() {
          debug!("[Menu] Player name set to \"{}\"", name);
          player_name.set(name);
        }
      }
      player_name.confirm();
      toggle_menu_message.write(ToggleMenuMessage::set(MenuName::PlayOnlineMenu));
    }
  }

  for interaction in &mut back_button_query {
    if *interaction == CustomInteraction::Released {
      debug!("[Menu] Selected \"Back\"");
      toggle_menu_message.write(ToggleMenuMessage::set(MenuName::MainMenu));
    }
  }
}

/// Despawns all elements with the [`EnterNameMenuRoot`] component.
fn despawn_menu_system(mut commands: Commands, menu_root_query: Query<Entity, With<EnterNameMenuRoot>>) {
  despawn_menu(&mut commands, &menu_root_query);
}

#[cfg(test)]
mod tests {
  use super::*;
  use bevy::prelude::{App, Messages, MinimalPlugins, Mut, Update};

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<ToggleMenuMessage>();
    let mut player_name = PlayerName::default();
    player_name.set("OldName".to_string());
    app.insert_resource(player_name);
    app
  }

  #[test]
  fn handle_button_interactions_system_saves_trimmed_editable_text_and_opens_play_online_menu() {
    let mut app = setup();
    app.add_systems(Update, handle_button_interactions_system);
    app.world_mut().spawn((ContinueButton, CustomInteraction::Released));
    app.world_mut().spawn((BackButton, CustomInteraction::None));
    app.world_mut().spawn((NameInputField, EditableText::new("  Moop  ")));

    app.update();

    let player_name = app.world().resource::<PlayerName>();
    assert_eq!(player_name.get(), "Moop");
    assert!(player_name.is_confirmed());

    let toggle_menu_messages: Mut<Messages<ToggleMenuMessage>> = app
      .world_mut()
      .get_resource_mut::<Messages<ToggleMenuMessage>>()
      .expect("Messages<ToggleMenuMessage> missing");
    let toggle_menu_messages: Vec<_> = toggle_menu_messages.iter_current_update_messages().collect();
    assert_eq!(toggle_menu_messages.len(), 1);
    assert_eq!(toggle_menu_messages[0].active, MenuName::PlayOnlineMenu);
  }

  #[test]
  fn handle_button_interactions_system_keeps_existing_name_when_editable_text_is_empty() {
    let mut app = setup();
    app.add_systems(Update, handle_button_interactions_system);
    app.world_mut().spawn((ContinueButton, CustomInteraction::Released));
    app.world_mut().spawn((BackButton, CustomInteraction::None));
    app.world_mut().spawn((NameInputField, EditableText::new("   ")));

    app.update();

    let player_name = app.world().resource::<PlayerName>();
    assert_eq!(player_name.get(), "OldName");
    assert!(player_name.is_confirmed());
  }

  #[test]
  fn handle_button_interactions_system_opens_main_menu_when_back_button_released() {
    let mut app = setup();
    app.add_systems(Update, handle_button_interactions_system);
    app.world_mut().spawn((ContinueButton, CustomInteraction::None));
    app.world_mut().spawn((BackButton, CustomInteraction::Released));
    app.world_mut().spawn((NameInputField, EditableText::new("Moop")));

    app.update();

    let toggle_menu_messages = app
      .world_mut()
      .get_resource_mut::<Messages<ToggleMenuMessage>>()
      .expect("Messages<ToggleMenuMessage> missing");
    let toggle_menu_messages: Vec<_> = toggle_menu_messages.iter_current_update_messages().collect();
    assert_eq!(toggle_menu_messages.len(), 1);
    assert_eq!(toggle_menu_messages[0].active, MenuName::MainMenu);
  }
}
