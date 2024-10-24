use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use fyfth_core::interpreter::FyfthInterpreter;

pub struct FyfthTerminalPlugin;

impl Plugin for FyfthTerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TerminalDisplayEvent>()
            .add_event::<TerminalInteractionEvent>()
            .configure_sets(Update, FyfthTerminalSystemSet)
            .add_systems(
                Update,
                (display_terminal, shell)
                    .chain()
                    .in_set(FyfthTerminalSystemSet),
            );
    }
}

#[derive(Debug, SystemSet, PartialEq, Eq, Hash, Clone)]
pub struct FyfthTerminalSystemSet;

#[derive(Debug, Clone, Event)]
pub enum TerminalDisplayEvent {
    Print(String),
    SetState(String),
}

#[derive(Debug, Clone, Event)]
pub enum TerminalInteractionEvent {
    Submit(String),
}

fn display_terminal(
    mut contexts: EguiContexts,
    mut current_string: Local<String>,
    mut history: Local<Vec<String>>,
    mut state: Local<String>,
    mut display_event_reader: EventReader<TerminalDisplayEvent>,
    mut interaction_event_writer: EventWriter<TerminalInteractionEvent>,
) {
    for display_event in display_event_reader.read() {
        match display_event {
            TerminalDisplayEvent::Print(value) => {
                history.push(value.clone());
            }
            TerminalDisplayEvent::SetState(value) => {
                *state = value.clone();
            }
        }
    }

    egui::Window::new("Terminal")
        .max_height(500.0)
        .show(contexts.ctx_mut(), |ui| {
            if ui.button("Clear").is_pointer_button_down_on() {
                history.clear();
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for line in history.iter() {
                    ui.label(line);
                }
            });

            ui.separator();
            // state label
            ui.label("Stack:");
            ui.label(&*state);
            // input field
            let response = ui.text_edit_singleline(&mut *current_string);

            if response.lost_focus() && !current_string.is_empty() {
                interaction_event_writer
                    .send(TerminalInteractionEvent::Submit(current_string.clone()));
                history.push(current_string.clone());
                *current_string = String::new();
            }
        });
}

fn shell(
    mut commands: Commands,
    mut interaction_event_reader: EventReader<TerminalInteractionEvent>,
) {
    let mut submitted_commands = vec![];
    for interaction in interaction_event_reader.read() {
        match interaction {
            TerminalInteractionEvent::Submit(value) => {
                submitted_commands.push(value.clone());
            }
        }
    }

    if !submitted_commands.is_empty() {
        commands.add(|world: &mut World| {
            let mut interpreter = world.resource::<FyfthInterpreter>().clone();

            for command in submitted_commands {
                interpreter.parse_code(&command);
            }

            let (output, res) = interpreter.run(world);

            if !output.is_empty() {
                world.send_event(TerminalDisplayEvent::Print(output));
            }

            // if we didn't encounter an error, update the interpreters value
            if res.is_ok() {
                world.send_event(TerminalDisplayEvent::SetState(
                    interpreter.pretty_print_stack(&world, " "),
                ));

                *world.resource_mut::<FyfthInterpreter>() = interpreter;
            }
        });
    }
}
