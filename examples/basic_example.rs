use bevy::{prelude::*, render::view::RenderLayers};
use bevy_egui::{EguiPlugin, EguiSettings};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins((
            fyfth::core::FyfthPlugin::new_from_prelude_paths(&["assets/fyfth/prelude.fy"]),
            fyfth::focus::FyfthFocusCameraPlugin,
            fyfth::terminal::FyfthTerminalPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut egui_settings: ResMut<EguiSettings>,
) {
    egui_settings.scale_factor = 1.25;

    commands.spawn((
        Name::new("Main Camera"),
        Camera3dBundle {
            camera: Camera {
                order: 0,
                is_active: true,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 1.5, 5.0)),
            ..default()
        },
        RenderLayers::layer(0),
        PanOrbitCamera::default(),
        fyfth::focus::FyfthFocusMainCameraTag,
    ));

    // add a directional light
    commands.spawn((
        Name::new("Directional Light (Sun)"),
        DirectionalLightBundle {
            transform: Transform::from_translation(Vec3::new(-2.0, 5.0, 1.0))
                .looking_at(Vec3::ZERO, Vec3::Z),
            ..default()
        },
    ));

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for i in -2..3 {
        // spawn a debug cube
        let cube = commands
            .spawn((
                Name::new(format!("debug cube {i}")),
                PbrBundle {
                    mesh: cube_mesh.clone(),
                    material: materials.add(Color::WHITE),
                    transform: Transform::from_translation(Vec3::new(2.0 * (i as f32), 0.0, 0.0)),
                    ..default()
                },
            ))
            .id();

        if i % 2 == 0 {
            commands.entity(cube).insert(fyfth::focus::FyfthFocusObject);
        }
    }
}
