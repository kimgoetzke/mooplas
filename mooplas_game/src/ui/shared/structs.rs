use bevy::prelude::Component;

/// Marker component for button animations.
#[derive(Component)]
pub(crate) struct ButtonAnimation;

#[derive(Component)]
pub(crate) struct BackgroundRoot;

/// Marker component for the lobby UI call-to-action (CTA) at the bottom of the player list.
#[derive(Component)]
pub(crate) struct LobbyUiCta;
