use crate::prelude::constants::{
  BUTTON_ALPHA_PRESSED, BUTTON_BORDER_WIDTH, DEFAULT_FONT, PIXEL_PERFECT_LAYER, RESOLUTION_HEIGHT, RESOLUTION_WIDTH,
  TEXT_COLOUR,
};
use crate::prelude::{AnimationIndices, AnimationTimer, CustomInteraction, RegularButton};
use crate::ui;
use crate::ui::ButtonAnimation;
use crate::ui::shared::BackgroundRoot;
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::color::palettes::tailwind;
use bevy::color::{Alpha, Color};
use bevy::ecs::children;
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::image::{Image, TextureAtlas, TextureAtlasLayout};
use bevy::math::{UVec2, Vec2};
use bevy::prelude::{
  AlignItems, BackgroundColor, BorderColor, BorderGradient, BorderRadius, Bundle, ChildOf, Component, Entity,
  FlexDirection, ImageNode, JustifyContent, LinearGradient, Name, Node, NodeImageMode, PositionType, Query,
  SpriteImageMode, Text, TextFont, TextShadow, Timer, TimerMode, UiRect, With, default, percent, px,
};
use bevy::prelude::{Commands, SpawnRelated, Sprite, Transform};
use bevy::sprite::ScalingMode;

/// Creates a node with the given marker component and name. This node serves as the base for every menu.
pub fn menu_base_node(marker_component: impl Component, name: String) -> impl Bundle {
  (
    Name::new(name),
    marker_component,
    Node {
      width: percent(100),
      height: percent(100),
      position_type: PositionType::Relative,
      flex_direction: FlexDirection::Column,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      row_gap: px(20),
      ..default()
    },
  )
}

/// Spawn the logo in an absolute positioned node.
pub fn spawn_logo<T: Component>(
  commands: &mut Commands,
  marker_component: T,
  logo_image: Handle<Image>,
  texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
) {
  let layout = TextureAtlasLayout::from_grid(UVec2::new(99, 25), 10, 1, None, None);
  let texture_atlas_layout = texture_atlas_layouts.add(layout);
  let animation_indices = AnimationIndices { first: 1, last: 9 };

  commands.spawn((
    Name::new("Logo Node"),
    marker_component,
    Node {
      width: percent(100),
      height: percent(25),
      position_type: PositionType::Absolute,
      flex_direction: FlexDirection::Row,
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },
    children![(
      Name::new("Logo"),
      Node {
        width: px(500.),
        height: px(125.),
        ..default()
      },
      ImageNode {
        image: logo_image,
        image_mode: NodeImageMode::Stretch,
        texture_atlas: Some(TextureAtlas {
          layout: texture_atlas_layout,
          index: 0,
        }),
        ..default()
      },
      animation_indices,
      AnimationTimer(Timer::from_seconds(0.25, TimerMode::Repeating)),
    )],
  ));
}

/// Spawns the background image for the given menu if it doesn't exist already.
pub fn spawn_background_if_not_exists<T: Component>(
  commands: &mut Commands,
  marker_component: T,
  background_image: Handle<Image>,
  texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
  background_root_query: Query<Entity, With<BackgroundRoot>>,
) {
  if !background_root_query.is_empty() {
    return;
  }

  let layout = TextureAtlasLayout::from_grid(UVec2::new(320, 160), 20, 1, None, None);
  let texture_atlas_layout = texture_atlas_layouts.add(layout);
  let animation_indices = AnimationIndices { first: 1, last: 19 };

  commands.spawn((
    Name::new(format!("Background for {}", std::any::type_name::<T>())),
    marker_component,
    Sprite {
      image: background_image,
      texture_atlas: Some(TextureAtlas {
        layout: texture_atlas_layout,
        index: 0,
      }),
      custom_size: Some(Vec2::new(RESOLUTION_WIDTH as f32, RESOLUTION_HEIGHT as f32)),
      image_mode: SpriteImageMode::Scale(ScalingMode::FillCenter),
      ..default()
    },
    Transform::from_xyz(0., 0., -2.),
    PIXEL_PERFECT_LAYER,
    animation_indices,
    AnimationTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
  ));
}

/// Spawns a [`RegularButton`] with the given parameters. Standard interaction observers are attached.
pub fn spawn_button(
  parent: &mut RelatedSpawnerCommands<ChildOf>,
  asset_server: &AssetServer,
  button_type: impl Component,
  button_text: &str,
  button_width: i32,
  font_size: f32,
) -> Entity {
  parent
    .spawn(button(button_type, asset_server, button_text, button_width, font_size))
    .observe(ui::set_interaction_on_hover)
    .observe(ui::set_interaction_on_hover_exit)
    .observe(ui::set_interaction_on_press)
    .observe(ui::set_interaction_on_release)
    .observe(ui::set_interaction_on_cancel)
    .id()
}

fn button(
  button_type: impl Component,
  asset_server: &AssetServer,
  button_text: &str,
  button_width: i32,
  font_size: f32,
) -> impl Bundle {
  (
    Node {
      width: px(button_width),
      height: px(65),
      border: UiRect::all(px(BUTTON_BORDER_WIDTH)),
      justify_content: JustifyContent::Center, // Horizontally centre child text
      align_items: AlignItems::Center,         // Vertically centre child text
      padding: UiRect::all(px(2)),
      ..default()
    },
    Name::new(format!("Button: {}", button_text)),
    RegularButton,
    CustomInteraction::default(),
    ButtonAnimation,
    button_type,
    BorderRadius::all(px(10)),
    BorderColor::all(Color::from(tailwind::SLATE_500)),
    BackgroundColor(Color::from(tailwind::SLATE_500.with_alpha(BUTTON_ALPHA_PRESSED))),
    default_gradient(0.),
    children![(
      Text::new(button_text),
      TextFont {
        font: asset_server.load(DEFAULT_FONT),
        font_size,
        ..default()
      },
      TEXT_COLOUR,
      TextShadow::default(),
    )],
  )
}

pub fn default_gradient(transparency: f32) -> BorderGradient {
  BorderGradient::from(LinearGradient {
    stops: vec![
      tailwind::YELLOW_400.with_alpha(transparency).into(),
      tailwind::YELLOW_50.with_alpha(transparency).into(),
      tailwind::YELLOW_400.with_alpha(transparency).into(),
    ],
    ..default()
  })
}

pub fn despawn_menu(commands: &mut Commands, marker_component_query: &Query<Entity, With<impl Component>>) {
  for root in marker_component_query {
    commands.entity(root).despawn();
  }
}
