use core::any::TypeId;
use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};

pub(crate) struct BevyComponentInfo {
    pub(crate) full_path: String,
    pub(crate) type_id: TypeId,
}

#[derive(Default)]
pub(crate) struct BevyComponentRegistry {
    pub(crate) registered_components: Vec<BevyComponentInfo>,
    pub(crate) registered_components_map: HashMap<TypeId, RegisteredBevyComponent>,
}

pub struct DynBevyComponent(pub(crate) Box<dyn FyfthCompatibleBevyComponent>);

impl std::fmt::Display for DynBevyComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.debug(f)
    }
}

impl std::fmt::Debug for DynBevyComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.debug(f)
    }
}

impl PartialEq for DynBevyComponent {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .reflect_partial_eq(other.0.as_reflect())
            .unwrap_or(false)
    }
}

impl Clone for DynBevyComponent {
    fn clone(&self) -> Self {
        self.0.shell_clone_self()
    }
}

pub(crate) struct RegisteredBevyComponent {
    pub(crate) from_reflect: Arc<dyn Fn(&dyn Reflect) -> Result<DynBevyComponent, ()>>,
    pub(crate) extract: Arc<dyn Fn(Entity, &World) -> Option<DynBevyComponent>>,
    pub(crate) insert: Arc<dyn Fn(Entity, &mut World, DynBevyComponent)>,
}

pub trait FyfthRegisterBevyComponent {
    fn fyfth_register_bevy_component<T>(&mut self) -> &mut Self
    where
        T: FyfthCompatibleBevyComponent + Component + Default + Clone;
}

impl FyfthRegisterBevyComponent for App {
    fn fyfth_register_bevy_component<T>(&mut self) -> &mut Self
    where
        T: FyfthCompatibleBevyComponent + Component + Default + Clone,
    {
        let world = self.world_mut();
        let mut register = world.non_send_resource_mut::<BevyComponentRegistry>();

        register.registered_components_map.insert(
            TypeId::of::<T>(),
            RegisteredBevyComponent {
                from_reflect: Arc::new(|refl| {
                    if refl.type_id() == TypeId::of::<T>() {
                        Ok(DynBevyComponent(Box::new(
                            refl.downcast_ref::<T>().unwrap().clone(),
                        )))
                    } else {
                        Err(())
                    }
                }),
                extract: Arc::new(|entity, world| {
                    world
                        .entity(entity)
                        .get::<T>()
                        .cloned()
                        .map(|comp| DynBevyComponent(Box::new(comp)))
                }),
                insert: Arc::new(|entity, world, value| {
                    world
                        .entity_mut(entity)
                        .insert(value.0.as_any().downcast_ref::<T>().unwrap().clone());
                }),
            },
        );
        let temp = T::default();
        register.registered_components.push(BevyComponentInfo {
            full_path: temp.reflect_type_path().to_string(),
            type_id: TypeId::of::<T>(),
        });

        self
    }
}

pub trait FyfthCompatibleBevyComponent: Reflect + std::fmt::Debug {
    fn shell_type_path(&self) -> String;
    fn shell_underlying_component_type_id(&self) -> core::any::TypeId;
    fn shell_clone_self(&self) -> DynBevyComponent;
}

impl<T> FyfthCompatibleBevyComponent for T
where
    T: Reflect + Clone + PartialEq + TypePath + std::fmt::Debug,
{
    fn shell_type_path(&self) -> String {
        T::type_path().to_string()
    }

    fn shell_underlying_component_type_id(&self) -> core::any::TypeId {
        TypeId::of::<T>()
    }

    fn shell_clone_self(&self) -> DynBevyComponent {
        DynBevyComponent(Box::new(self.clone()))
    }
}
