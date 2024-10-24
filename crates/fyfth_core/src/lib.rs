use bevy::prelude::*;
use bevy_component::BevyComponentRegistry;
use interpreter::FyfthInterpreter;

pub mod bevy_component;
pub mod interpreter;
pub mod language;
pub mod lexer;
pub mod util;

#[derive(Component)]
pub struct FyfthIgnoreEntity;

#[derive(Debug, Default)]
pub struct FyfthPlugin {
    preludes: Vec<String>,
}

impl FyfthPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_prelude_paths(paths: &[&str]) -> Self {
        Self {
            preludes: paths.iter().map(|&p| p.to_string()).collect(),
        }
    }

    pub fn with_prelude(&mut self, path: &str) -> &mut Self {
        self.preludes.push(path.to_string());
        self
    }
}

impl Plugin for FyfthPlugin {
    fn build(&self, app: &mut App) {
        let world = app.world_mut();
        let mut interpreter = FyfthInterpreter::new();

        // Parse all provided preludes into the interpreter
        for path in self.preludes.iter() {
            // TODO: Don't panic when we can't find the file?
            let prelude = std::fs::read_to_string(path).unwrap();
            interpreter.parse_code(&prelude);
            let (_, res) = interpreter.run(world);
            res.unwrap();
        }

        world.insert_resource(interpreter);
        world.init_non_send_resource::<BevyComponentRegistry>();
    }
}
