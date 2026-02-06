use bevy::prelude::{Deref, DerefMut, Resource};
use renet_visualizer::{RenetClientVisualizer, RenetServerVisualizer};

/// Whether to show the renet visualisers by default.
pub(crate) const SHOW_VISUALISERS_BY_DEFAULT: bool = true;

/// The number of values to display in the renet visualiser graphs.
pub(crate) const VISUALISER_DISPLAY_VALUES: usize = 200;

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenetClientVisualiser(RenetClientVisualizer<{ VISUALISER_DISPLAY_VALUES }>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenetServerVisualiser(RenetServerVisualizer<{ VISUALISER_DISPLAY_VALUES }>);
