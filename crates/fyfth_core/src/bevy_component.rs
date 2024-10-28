use core::any::TypeId;

use bevy::{prelude::*, utils::HashMap};

use crate::util;

pub(crate) struct BevyComponentInfo {
    pub(crate) full_path: String,
    pub(crate) type_ident: Option<String>,
    pub(crate) type_id: TypeId,
}

#[derive(Default, Resource)]
pub(crate) struct BevyComponentRegistry {
    pub(crate) registered_components: Vec<BevyComponentInfo>,
    pub(crate) registered_components_map: HashMap<TypeId, RegisteredBevyComponentHandler>,
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

#[derive(Clone)]
pub(crate) struct RegisteredBevyComponentHandler {
    func_from_world: unsafe fn(*mut World) -> DynBevyComponent,
    func_from_reflect: unsafe fn(*const dyn Reflect) -> Result<DynBevyComponent, ()>,
    func_extract: unsafe fn(Entity, *const World) -> Option<DynBevyComponent>,
    func_insert: unsafe fn(Entity, *mut World, DynBevyComponent),
}

impl RegisteredBevyComponentHandler {
    pub(crate) fn from_world(&self, world: &mut World) -> DynBevyComponent {
        unsafe {
            // SAFETY: We have a mutable reference to world guaranteeing that the
            // pointer we produce is valid and also that there are no other mutable references
            // to world.
            (self.func_from_world)(core::ptr::from_mut(world))
        }
    }

    pub(crate) fn from_reflect(&self, reflect: &dyn Reflect) -> Result<DynBevyComponent, ()> {
        unsafe { (self.func_from_reflect)(core::ptr::from_ref(reflect)) }
    }

    pub(crate) fn extract(&self, entity: Entity, world: &World) -> Option<DynBevyComponent> {
        unsafe {
            // SAFETY: We have a reference to world guaranteeing that the
            // pointer we produce is valid and also that there are no mutable references
            // to world.
            (self.func_extract)(entity, core::ptr::from_ref(world))
        }
    }

    pub(crate) fn insert(&self, entity: Entity, world: &mut World, value: DynBevyComponent) {
        unsafe {
            // SAFETY: We have a mutable reference to world guaranteeing that the
            // pointer we produce is valid and also that there are no other mutable references
            // to world.
            (self.func_insert)(entity, core::ptr::from_mut(world), value);
        }
    }
}

pub trait FyfthRegisterBevyComponent {
    fn fyfth_register_bevy_component<T>(&mut self) -> &mut Self
    where
        T: FyfthCompatibleBevyComponent + Component + FromWorld + Clone;
}

/// **SAFETY**: Pointer must be valid and there must not be any other mutable references to world.
unsafe fn from_world_impl<'w, T>(world: *mut World) -> DynBevyComponent
where
    T: FyfthCompatibleBevyComponent + FromWorld,
{
    DynBevyComponent(Box::new(T::from_world(world.as_mut().unwrap())))
}

/// **SAFETY**: Pointer must be valid.
unsafe fn from_reflect_impl<'a, T>(reflect: *const dyn Reflect) -> Result<DynBevyComponent, ()>
where
    T: FyfthCompatibleBevyComponent + Clone,
{
    let reflect = reflect.as_ref().unwrap();
    if reflect.type_id() == TypeId::of::<T>() {
        Ok(DynBevyComponent(Box::new(
            reflect.downcast_ref::<T>().unwrap().clone(),
        )))
    } else {
        Err(())
    }
}

/// **SAFETY**: Pointer must be valid and there must not be any mutable references to world.
unsafe fn extract_impl<'w, T>(entity: Entity, world: *const World) -> Option<DynBevyComponent>
where
    T: FyfthCompatibleBevyComponent + Component + Clone,
{
    world
        .as_ref()
        .unwrap()
        .entity(entity)
        .get::<T>()
        .cloned()
        .map(|comp| DynBevyComponent(Box::new(comp)))
}

/// **SAFETY**: Pointer must be valid and there must not be any other mutable references to world.
unsafe fn insert_impl<'w, T>(entity: Entity, world: *mut World, value: DynBevyComponent)
where
    T: Component + FromWorld + Clone,
{
    world
        .as_mut()
        .unwrap()
        .entity_mut(entity)
        .insert(value.0.as_any().downcast_ref::<T>().unwrap().clone());
}

impl FyfthRegisterBevyComponent for App {
    fn fyfth_register_bevy_component<T>(&mut self) -> &mut Self
    where
        T: FyfthCompatibleBevyComponent + Component + FromWorld + Clone,
    {
        let world = self.world_mut();
        let temp = T::from_world(world);
        let mut register = world.resource_mut::<BevyComponentRegistry>();

        register.registered_components_map.insert(
            TypeId::of::<T>(),
            RegisteredBevyComponentHandler {
                func_from_world: from_world_impl::<T>,
                func_from_reflect: from_reflect_impl::<T>,
                func_extract: extract_impl::<T>,
                func_insert: insert_impl::<T>,
            },
        );
        register.registered_components.push(BevyComponentInfo {
            full_path: temp.reflect_type_path().to_string(),
            type_ident: temp.reflect_type_ident().map(|name| name.to_string()),
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

pub(crate) enum BevyComponentRegistryError {
    NoMatchingComponent,
    MultipleMatchingComponents(Vec<usize>),
}

impl BevyComponentRegistry {
    pub(crate) fn try_find_component_by_name(
        &self,
        component_name: &str,
    ) -> Result<TypeId, BevyComponentRegistryError> {
        for comp_path in self.registered_components.iter() {
            println!("Registered Component: {}", comp_path.full_path);
        }

        let component_name_full_matches: Vec<usize> = self
            .registered_components
            .iter()
            .enumerate()
            .filter_map(|(index, ci)| {
                ci.type_ident.as_ref().and_then(|ident| {
                    util::case_ignored_match(&ident, component_name).then_some(index)
                })
            })
            .collect();

        match component_name_full_matches.len() {
            // Good! We have exactly one full match! That's our component
            1 => Ok(self.registered_components[component_name_full_matches[0]].type_id),
            // There are multiple components whose type ident fully matches the query string.
            // We cannot infer which the user might have meant. Return an error.
            2.. => Err(BevyComponentRegistryError::MultipleMatchingComponents(
                component_name_full_matches,
            )),
            // There are no components that match exactly. Try fuzzy matching their entire type paths
            0 => {
                let component_name_fuzzy_matches: Vec<usize> = self
                    .registered_components
                    .iter()
                    .enumerate()
                    .filter_map(|(index, ci)| {
                        ci.type_ident.as_ref().and_then(|ident| {
                            util::fuzzy_match(&ident, component_name).then_some(index)
                        })
                    })
                    .collect();

                match component_name_fuzzy_matches.len() {
                    // Good! We have exactly one full match! That's our component
                    1 => Ok(self.registered_components[component_name_fuzzy_matches[0]].type_id),
                    // There are multiple components whose type ident fully matches the query string.
                    // We cannot infer which the user might have meant. Return an error.
                    2.. => Err(BevyComponentRegistryError::MultipleMatchingComponents(
                        component_name_fuzzy_matches,
                    )),
                    // We still haven't found anything. Try fuzzy matching the entire type path
                    0 => {
                        let component_type_path_fuzzy_matches: Vec<usize> = self
                            .registered_components
                            .iter()
                            .enumerate()
                            .filter_map(|(index, ci)| {
                                util::fuzzy_match(&ci.full_path, component_name).then_some(index)
                            })
                            .collect();

                        match component_type_path_fuzzy_matches.len() {
                            // Good! We have exactly one full match! That's our component
                            1 => Ok(self.registered_components
                                [component_type_path_fuzzy_matches[0]]
                                .type_id),
                            // There are multiple components whose type ident fully matches the query string.
                            // We cannot infer which the user might have meant. Return an error.
                            2.. => Err(BevyComponentRegistryError::MultipleMatchingComponents(
                                component_type_path_fuzzy_matches,
                            )),
                            // Okay, now we really don't have anything.
                            0 => Err(BevyComponentRegistryError::NoMatchingComponent),
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn get_info(&self, type_id: TypeId) -> Option<&BevyComponentInfo> {
        self.registered_components
            .iter()
            .filter_map(|ci| (ci.type_id == type_id).then_some(ci))
            .next()
    }
}
