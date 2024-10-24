use bevy::prelude::*;

pub mod bevy_component;
pub mod interpreter;
pub mod language;
pub mod lexer;
pub mod util;

#[derive(Component)]
pub struct FyfthIgnoreEntity;
