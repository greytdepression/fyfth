mod language_extension;

use bevy::{
    prelude::*,
    render::{
        render_resource::{AsBindGroup, ShaderRef},
        view::RenderLayers,
    },
};
use fyfth_core::{
    interpreter::FyfthInterpreter,
    language::{FyfthBroadcastBehavior, FyfthLanguageExtension},
    FyfthIgnoreEntity,
};

pub const FOCUS_RENDER_LAYER: RenderLayers = RenderLayers::layer(31);

pub struct FyfthFocusCameraPlugin;

#[derive(Resource)]
struct FocusAvatarMaterialHandle(Handle<FocusAvatarMaterial>);

#[derive(Debug, Component)]
pub struct FyfthFocusMainCameraTag;

impl Plugin for FyfthFocusCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FocusAvatarMaterial>::default())
            .add_systems(
                PostUpdate,
                (focus_camera_copy_camera_settings, focus_camera_follow)
                    .after(TransformSystem::TransformPropagate),
            )
            .add_systems(
                PostUpdate,
                (focus_object_mesh_changes, focus_object_follow)
                    .chain()
                    .after(TransformSystem::TransformPropagate),
            )
            .add_systems(Startup, setup);

        let mut language_extension = FyfthLanguageExtension::new_empty();
        language_extension
            .with_command(
                "focus",
                language_extension::fyfth_func_focus,
                &[FyfthBroadcastBehavior::MayIter],
            )
            .with_command(
                "unfocus",
                language_extension::fyfth_func_unfocus,
                &[FyfthBroadcastBehavior::MayIter],
            )
            .with_command("focused", language_extension::fyfth_func_focused, &[]);

        let mut interpreter = app.world_mut().get_resource_mut::<FyfthInterpreter>()
            .expect("Make sure to register the `FyfthPlugin` before registering the `FyfthFocusMainCameraPlugin`.");
        interpreter.add_language_extension(language_extension);
    }
}

fn setup(world: &mut World) {
    // Spawn the focus camera
    world.spawn((
        Name::new("Fyfth Focus Camera"),
        Camera3dBundle {
            camera: Camera {
                order: 10,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            ..default()
        },
        FyfthFocusCamera,
        FOCUS_RENDER_LAYER,
        FyfthIgnoreEntity,
    ));

    // Load the focus material
    let mut focus_mat = world.resource_mut::<Assets<FocusAvatarMaterial>>();
    let focus_mat_handle = focus_mat.add(FocusAvatarMaterial {});
    world.insert_resource(FocusAvatarMaterialHandle(focus_mat_handle));

    // Register component hooks
    world
        .register_component_hooks::<FyfthFocusObject>()
        .on_add(|mut world, ent, _comp_id| {
            // Make sure this entity has a mesh
            let Some(mesh) = world
                .entity(ent)
                .get::<Handle<Mesh>>()
                .map(|m| m.clone_weak())
            else {
                return;
            };

            world.commands().add(move |world: &mut World| {
                // Make sure there does not yet exist a focus avatar for this entity
                let mut query = world.query::<&FocusObjectAvatar>();
                if query.iter(world).any(|fa| fa.0 == ent) {
                    return;
                }

                let target_entity = world.entity(ent);
                let mat_handle = world.resource::<FocusAvatarMaterialHandle>().0.clone_weak();

                let avatar_name = if let Some(target_name) = target_entity.get::<Name>() {
                    format!("Focus Avatar for Entity '{}'", target_name.as_str())
                } else {
                    format!("Focus Avatar for Entity ({ent})")
                };
                world.spawn((
                    Name::from(avatar_name),
                    FocusObjectAvatar(ent),
                    mat_handle,
                    mesh,
                    SpatialBundle::default(),
                    FOCUS_RENDER_LAYER,
                    FyfthIgnoreEntity,
                ));
            });
        })
        .on_remove(|mut world, ent, _comp_id| {
            world.commands().add(move |world: &mut World| {
                let mut query = world.query::<(Entity, &FocusObjectAvatar)>();

                let mut entities_to_despawn = vec![];
                for (av_ent, avatar) in query.iter(&world) {
                    if avatar.0 == ent {
                        entities_to_despawn.push(av_ent);
                    }
                }

                for av_ent in entities_to_despawn {
                    world.despawn(av_ent);
                }
            });
        });
}

#[derive(Component)]
pub struct FyfthFocusCamera;

#[derive(Component)]
pub struct FyfthFocusObject;

#[derive(Component)]
pub(crate) struct FocusObjectAvatar(pub(crate) Entity);

fn focus_camera_follow(
    other_camera_query: Query<
        &GlobalTransform,
        (Without<FyfthFocusCamera>, With<FyfthFocusMainCameraTag>),
    >,
    mut focus_camera_query: Query<&mut GlobalTransform, With<FyfthFocusCamera>>,
) {
    for mut focus_cam_transform in focus_camera_query.iter_mut() {
        if let Ok(target_transform) = other_camera_query.get_single() {
            *focus_cam_transform = target_transform.clone();
        } else {
            warn_once!("More than one `FyfthFocusMainCameraTag` in world!")
        }
    }
}

fn focus_camera_copy_camera_settings(
    other_camera_query: Query<
        (&Projection, &Camera),
        (
            With<FyfthFocusMainCameraTag>,
            Without<FyfthFocusCamera>,
            Or<(Changed<Projection>, Changed<Camera>)>,
        ),
    >,
    mut focus_camera_query: Query<(&mut Projection, &mut Camera), With<FyfthFocusCamera>>,
) {
    for (mut focus_cam_proj, mut focus_cam_cam) in focus_camera_query.iter_mut() {
        if let Ok((proj, cam)) = other_camera_query.get_single() {
            *focus_cam_proj.bypass_change_detection() = proj.clone();
            focus_cam_cam.bypass_change_detection().viewport = cam.viewport.clone();
            focus_cam_cam.bypass_change_detection().target = cam.target.clone();
        }
    }
}

fn focus_object_follow(
    focus_object_query: Query<
        &GlobalTransform,
        (With<FyfthFocusObject>, Without<FocusObjectAvatar>),
    >,
    mut avatar_query: Query<(&FocusObjectAvatar, &mut GlobalTransform), Without<FyfthFocusObject>>,
) {
    for (avatar, mut av_global_trans) in avatar_query.iter_mut() {
        if let Ok(target_global_transform) = focus_object_query.get(avatar.0) {
            *av_global_trans.bypass_change_detection() = target_global_transform.clone()
        }
    }
}

fn focus_object_mesh_changes(
    mut commands: Commands,
    focus_object_query: Query<
        &Handle<Mesh>,
        (
            With<FyfthFocusObject>,
            Without<FocusObjectAvatar>,
            Changed<Handle<Mesh>>,
        ),
    >,
    mut avatar_query: Query<(Entity, &FocusObjectAvatar), Without<FyfthFocusObject>>,
) {
    for (avatar_ent, avatar) in avatar_query.iter_mut() {
        if let Ok(target_new_mesh) = focus_object_query.get(avatar.0) {
            commands
                .entity(avatar_ent)
                .insert(target_new_mesh.clone_weak());
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct FocusAvatarMaterial {}

impl Material for FocusAvatarMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/focus.wgsl".into()
    }
}
