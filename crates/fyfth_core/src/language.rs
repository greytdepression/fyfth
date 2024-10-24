use std::fmt::Write;

use bevy::prelude::*;
use bevy::utils::HashMap;
use regex::Regex;

use crate::{
    bevy_component::BevyComponentRegistry,
    interpreter::{FyfthContext, FyfthVariant},
    util, FyfthIgnoreEntity,
};

#[derive(Debug, Clone)]
pub struct FyfthLanguageExtension {
    pub(crate) keywords: HashMap<String, u32>,
    pub(crate) functions: Vec<FnInfo>,
    pub(crate) prefixes: Vec<PrefixInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct FnInfo {
    pub(crate) keyword: String,
    pub(crate) simple_function: SimpleFunc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FyfthBroadcastBehavior {
    IgnoreIter,
    MayIter,
}

#[derive(Debug, Clone)]
pub(crate) struct PrefixInfo {
    pub(crate) ch: char,
    pub(crate) fn_ptr: FyfthPrefixParserFnPtr,
}

pub type FyfthFuncFnPtr = fn(FyfthContext, &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()>;
pub type FyfthPrefixParserFnPtr =
    fn(&str, &FyfthLanguageExtension) -> Result<Vec<FyfthVariant>, ()>;

#[derive(Debug, Clone)]
pub(crate) struct SimpleFunc {
    pub(crate) fn_ptr: FyfthFuncFnPtr,
    pub(crate) broadcast_behaviors: Box<[FyfthBroadcastBehavior]>,
}

//--------------------------------------------------
// Standard Language Extension
//--------------------------------------------------
impl FyfthLanguageExtension {
    pub fn new_empty() -> Self {
        Self {
            keywords: default(),
            functions: default(),
            prefixes: default(),
        }
    }

    pub fn get_command_id(&self, keyword: &str) -> Option<u32> {
        self.keywords.get(keyword).copied()
    }

    pub fn merge(&mut self, other: Self) -> Result<(), ()> {
        // Make sure the other language extension does not conflict with this one
        if other.keywords.keys().any(|k| self.keywords.contains_key(k)) {
            // Colliding keywords!
            return Err(());
        }
        if other
            .prefixes
            .iter()
            .any(|op| self.prefixes.iter().any(|sp| op.ch == sp.ch))
        {
            // Colliding prefixes!
            return Err(());
        }

        let FyfthLanguageExtension {
            keywords,
            functions,
            prefixes,
        } = other;

        // We can safely merge the two languages
        let keyword_index_offset = self.functions.len() as u32;
        self.functions.extend_from_slice(&functions);
        keywords.iter().for_each(|(kw, &id)| {
            self.keywords.insert(kw.clone(), id + keyword_index_offset);
        });
        self.prefixes.extend_from_slice(&prefixes);

        Ok(())
    }

    pub fn with_command(
        &mut self,
        kw: &str,
        fn_ptr: FyfthFuncFnPtr,
        broadcast_behaviors: &[FyfthBroadcastBehavior],
    ) -> &mut Self {
        let index = self.functions.len() as u32;
        self.functions.push(FnInfo {
            keyword: kw.to_string(),
            simple_function: SimpleFunc {
                fn_ptr,
                broadcast_behaviors: broadcast_behaviors
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            },
        });
        self.keywords.insert(kw.to_string(), index);
        self
    }

    pub fn with_prefix(&mut self, prefix_char: char, fn_ptr: FyfthPrefixParserFnPtr) -> &mut Self {
        // make sure this prefix is not yet in use
        if prefix_char == '"' {
            panic!("FyfthLanguageExtension::with_prefix: prefix cannot be `\"`");
        }

        if self.prefixes.iter().any(|pi| pi.ch == prefix_char) {
            panic!("FyfthLanguageExtension::with_prefix: `{prefix_char}` already in use");
        }

        self.prefixes.push(PrefixInfo {
            ch: prefix_char,
            fn_ptr,
        });

        self
    }

    pub fn base_fyfth() -> Self {
        let mut lang = Self {
            keywords: default(),
            functions: default(),
            prefixes: default(),
        };

        lang.with_command("entities", fyfth_func_entities, &[])
            .with_command(
                "get",
                fyfth_func_get,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "set",
                fyfth_func_set,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "add",
                fyfth_func_add,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "sub",
                fyfth_func_sub,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "mul",
                fyfth_func_mul,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "div",
                fyfth_func_div,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "print",
                fyfth_func_print,
                &[FyfthBroadcastBehavior::IgnoreIter],
            )
            .with_command(
                "store",
                fyfth_func_store,
                &[
                    FyfthBroadcastBehavior::IgnoreIter,
                    FyfthBroadcastBehavior::IgnoreIter,
                ],
            )
            .with_command("load", fyfth_func_load, &[FyfthBroadcastBehavior::MayIter])
            .with_command("print_vars", fyfth_func_print_vars, &[])
            .with_command(
                "geq",
                fyfth_func_geq,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "leq",
                fyfth_func_leq,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "eq",
                fyfth_func_eq,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "eqq",
                fyfth_func_eq,
                &[
                    FyfthBroadcastBehavior::IgnoreIter,
                    FyfthBroadcastBehavior::IgnoreIter,
                ],
            )
            .with_command("not", fyfth_func_not, &[FyfthBroadcastBehavior::MayIter])
            .with_command("name", fyfth_func_name, &[FyfthBroadcastBehavior::MayIter])
            .with_command("pop", fyfth_func_pop, &[FyfthBroadcastBehavior::IgnoreIter])
            .with_command(
                "index",
                fyfth_func_index,
                &[
                    FyfthBroadcastBehavior::IgnoreIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "enum",
                fyfth_func_enum,
                &[FyfthBroadcastBehavior::IgnoreIter],
            )
            .with_command("len", fyfth_func_len, &[FyfthBroadcastBehavior::IgnoreIter])
            .with_command(
                "type",
                fyfth_func_type,
                &[FyfthBroadcastBehavior::IgnoreIter],
            )
            .with_command(
                "append",
                fyfth_func_append,
                &[
                    FyfthBroadcastBehavior::IgnoreIter,
                    FyfthBroadcastBehavior::IgnoreIter,
                ],
            )
            .with_command(
                "extend",
                fyfth_func_extend,
                &[
                    FyfthBroadcastBehavior::IgnoreIter,
                    FyfthBroadcastBehavior::IgnoreIter,
                ],
            )
            .with_command(
                "reverse",
                fyfth_func_reverse,
                &[FyfthBroadcastBehavior::IgnoreIter],
            )
            .with_command(
                "filter",
                fyfth_func_filter,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "select",
                fyfth_func_select,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "mod",
                fyfth_func_mod,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "vec2",
                fyfth_func_vec2,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "vec3",
                fyfth_func_vec3,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "quat",
                fyfth_func_quat,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "fuzzy",
                fyfth_func_fuzzy,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command(
                "regex",
                fyfth_func_regex,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            )
            .with_command("sin", fyfth_func_sin, &[FyfthBroadcastBehavior::MayIter])
            .with_command("cos", fyfth_func_cos, &[FyfthBroadcastBehavior::MayIter])
            .with_command("tan", fyfth_func_tan, &[FyfthBroadcastBehavior::MayIter])
            .with_command("atan", fyfth_func_atan, &[FyfthBroadcastBehavior::MayIter])
            .with_command(
                "atan2",
                fyfth_func_atan2,
                &[
                    FyfthBroadcastBehavior::MayIter,
                    FyfthBroadcastBehavior::MayIter,
                ],
            );

        // prefixes
        lang.with_prefix('*', fyfth_prefix_load);
        lang.with_prefix('$', fyfth_prefix_queue_macro);
        lang.with_prefix('@', fyfth_prefix_fuzzy_entity);

        lang
    }
}

//--------------------------------------------------
// Command Implementations
//--------------------------------------------------

fn fyfth_func_entities(
    ctx: FyfthContext,
    args: &[FyfthVariant],
) -> Result<Option<FyfthVariant>, ()> {
    let [] = args else {
        panic!("received the wrong number of arguments")
    };

    let mut entity_query = ctx
        .world
        .query_filtered::<Entity, Without<FyfthIgnoreEntity>>();
    let entities = entity_query
        .iter(ctx.world)
        .map(|ent| FyfthVariant::Entity(ent))
        .collect();

    Ok(Some(FyfthVariant::Iter(entities)))
}

fn fyfth_func_set(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, mhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };

    match (lhs, mhs, rhs) {
        (FyfthVariant::Iter(vec), &FyfthVariant::Num(index), val) => {
            let mut vec = vec.clone();
            let index = if index >= 0.0 {
                index as i32
            } else {
                vec.len() as i32 + index as i32
            };

            if 0 <= index && index < vec.len() as i32 {
                vec[index as usize] = val.clone();
                Ok(Some(FyfthVariant::Iter(vec)))
            } else {
                write!(
                    ctx.output,
                    "Error: `set` tried setting index {index} of an iter of length {}",
                    vec.len()
                )
                .unwrap();
                Err(())
            }
        }
        (FyfthVariant::Component(dyn_comp), FyfthVariant::Literal(field_name), val) => {
            let mut dyn_comp = dyn_comp.clone();
            match dyn_comp.0.reflect_mut() {
                bevy::reflect::ReflectMut::Struct(strct) => {
                    if strct.field(&field_name).is_none() {
                        write!(
                            ctx.output,
                            "Error: component `{}` does not have a field `{}`",
                            dyn_comp.0.reflect_type_path(),
                            &field_name,
                        )
                        .unwrap();
                        return Err(());
                    }

                    if let Ok(_) = val.try_set_reflect_field(strct, &field_name) {
                        Ok(Some(FyfthVariant::Component(dyn_comp)))
                    } else {
                        write!(
                            ctx.output,
                            "Error: failed to set field `{}` of component `{}` to value `",
                            &field_name,
                            dyn_comp.0.reflect_type_path(),
                        )
                        .unwrap();
                        val.pretty_print(ctx.output, ctx.world, ctx.lang);
                        ctx.output.push('`');
                        Err(())
                    }
                }
                _ => {
                    write!(
                        ctx.output,
                        "Error: component `{}` is not a struct",
                        dyn_comp.0.reflect_type_path(),
                    )
                    .unwrap();
                    Err(())
                }
            }
        }
        (lhs, mhs, rhs) => {
            ctx.output
                .push_str("Syntax error: the operation `set` is incompatible with types `");
            lhs.pretty_print_type(ctx.output);
            ctx.output.push(' ');
            mhs.pretty_print_type(ctx.output);
            ctx.output.push(' ');
            rhs.pretty_print_type(ctx.output);
            ctx.output.push_str(" `.");
            Err(())
        }
    }
}

fn fyfth_func_get(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (FyfthVariant::Iter(vec), &FyfthVariant::Num(index)) => {
            let index = if index >= 0.0 {
                index as i32
            } else {
                vec.len() as i32 + index as i32
            };

            if 0 <= index && index < vec.len() as i32 {
                Ok(Some(vec[index as usize].clone()))
            } else {
                write!(
                    ctx.output,
                    "Error: `get` tried getting index {index} of an iter of length {}",
                    vec.len()
                )
                .unwrap();
                Err(())
            }
        }
        (&FyfthVariant::Vec2(vec), FyfthVariant::Literal(comp)) => match comp.as_str() {
            "x" => Ok(Some(FyfthVariant::Num(vec.x))),
            "y" => Ok(Some(FyfthVariant::Num(vec.y))),
            _ => {
                write!(ctx.output, "Error: vec2 has no `{comp}` component",).unwrap();
                Err(())
            }
        },
        (&FyfthVariant::Vec3(vec), FyfthVariant::Literal(comp)) => match comp.as_str() {
            "x" => Ok(Some(FyfthVariant::Num(vec.x))),
            "y" => Ok(Some(FyfthVariant::Num(vec.y))),
            "z" => Ok(Some(FyfthVariant::Num(vec.z))),
            _ => {
                write!(ctx.output, "Error: vec3 has no `{comp}` component",).unwrap();
                Err(())
            }
        },
        (&FyfthVariant::Quat(quat), FyfthVariant::Literal(comp)) => match comp.as_str() {
            "x" => Ok(Some(FyfthVariant::Num(quat.x))),
            "y" => Ok(Some(FyfthVariant::Num(quat.y))),
            "z" => Ok(Some(FyfthVariant::Num(quat.z))),
            "w" => Ok(Some(FyfthVariant::Num(quat.w))),
            _ => {
                write!(ctx.output, "Error: quat has no `{comp}` component",).unwrap();
                Err(())
            }
        },
        (&FyfthVariant::Entity(entity), FyfthVariant::Literal(val)) => {
            let registry = ctx.world.non_send_resource::<BevyComponentRegistry>();

            let fuzzy_matches: Vec<_> = registry
                .registered_components
                .iter()
                .filter_map(|info| {
                    if util::fuzzy_match(&info.full_path, &val) {
                        Some(info)
                    } else {
                        None
                    }
                })
                .collect();

            match fuzzy_matches.len() {
                1 => {
                    let info = fuzzy_matches.first().unwrap();
                    let component_type_id = info.type_id;
                    let maybe_component_dyn = (registry
                        .registered_components_map
                        .get(&component_type_id)
                        .unwrap()
                        .extract)(entity, ctx.world);

                    if let Some(component_dyn) = maybe_component_dyn {
                        Ok(Some(FyfthVariant::Component(component_dyn)))
                    } else {
                        write!(
                            ctx.output,
                            "Error: entity ({}) does not contain component `{}`",
                            entity, info.full_path,
                        )
                        .unwrap();
                        Err(())
                    }
                }
                0 => {
                    write!(
                        ctx.output,
                        "Error: could not find component `{}` in registry. Make sure it is registered using `app.register_shell_component::<T>()`.",
                        &val
                    )
                    .unwrap();
                    Err(())
                }
                _ => {
                    write!(
                        ctx.output,
                        "Error: multiple components fit the name '{}'. Specify the name more clearly avoid ambiguity. The matching components are:\n",
                        &val
                    )
                    .unwrap();

                    for &info in fuzzy_matches.iter() {
                        writeln!(ctx.output, "  {}", &info.full_path).unwrap();
                    }

                    Err(())
                }
            }
        }
        (FyfthVariant::Component(dyn_comp), FyfthVariant::Literal(field_name)) => {
            match dyn_comp.0.reflect_ref() {
                bevy::reflect::ReflectRef::Struct(val) => {
                    if let Some(field) = val.field(&field_name) {
                        let registry = ctx.world.non_send_resource::<BevyComponentRegistry>();
                        if let Some(shell_value) =
                            FyfthVariant::try_reflect_from_type_id(field, registry)
                        {
                            Ok(Some(shell_value))
                        } else {
                            write!(
                                ctx.output,
                                "Error: field `{}` of component `{}` has an unsupported type",
                                &field_name,
                                dyn_comp.0.reflect_type_path(),
                            )
                            .unwrap();
                            Err(())
                        }
                    } else {
                        write!(
                            ctx.output,
                            "Error: component `{}` does not have a field `{}`",
                            dyn_comp.0.reflect_type_path(),
                            &field_name,
                        )
                        .unwrap();
                        Err(())
                    }
                }
                _ => {
                    write!(
                        ctx.output,
                        "Error: component `{}` is not a struct",
                        dyn_comp.0.reflect_type_path(),
                    )
                    .unwrap();
                    Err(())
                }
            }
        }
        (lhs, rhs) => {
            ctx.output
                .push_str("Syntax error: the operation `get` is incompatible with types `");
            lhs.pretty_print_type(ctx.output);
            ctx.output.push(' ');
            rhs.pretty_print_type(ctx.output);
            ctx.output.push_str(" `.");
            Err(())
        }
    }
}

fn fyfth_func_add(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Num(lhs + rhs)))
        }
        (&FyfthVariant::Vec2(lhs), &FyfthVariant::Vec2(rhs)) => {
            Ok(Some(FyfthVariant::Vec2(lhs + rhs)))
        }
        (&FyfthVariant::Vec3(lhs), &FyfthVariant::Vec3(rhs)) => {
            Ok(Some(FyfthVariant::Vec3(lhs + rhs)))
        }
        (FyfthVariant::Literal(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Literal(format!("{lhs}{rhs}"))))
        }
        (FyfthVariant::Literal(lhs), FyfthVariant::Literal(rhs)) => {
            Ok(Some(FyfthVariant::Literal(format!("{lhs}{rhs}"))))
        }
        (&FyfthVariant::Entity(entity), FyfthVariant::Component(dyn_comp)) => {
            let registry = ctx.world.non_send_resource::<BevyComponentRegistry>();
            let inserter = registry
                .registered_components_map
                .get(&dyn_comp.0.shell_underlying_component_type_id())
                .expect("The shell component registry has been corrupted!")
                .insert
                .clone();

            (*inserter)(entity, ctx.world, dyn_comp.clone());

            Ok(None)
        }
        (lhs, rhs) => {
            ctx.output
                .push_str("Syntax error: the operation `add` is incompatible with types `");
            lhs.pretty_print_type(ctx.output);
            ctx.output.push(' ');
            rhs.pretty_print_type(ctx.output);
            ctx.output.push_str(" `.");
            Err(())
        }
    }
}

fn fyfth_func_sub(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Num(lhs - rhs)))
        }
        (lhs, rhs) => {
            ctx.output
                .push_str("Syntax error: the operation `add` is incompatible with types `");
            lhs.pretty_print_type(ctx.output);
            ctx.output.push(' ');
            rhs.pretty_print_type(ctx.output);
            ctx.output.push_str(" `.");
            Err(())
        }
    }
}

fn fyfth_func_print(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    val.pretty_print(ctx.output, ctx.world, ctx.lang);
    Ok(None)
}

fn fyfth_func_store(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (val, FyfthVariant::Literal(ident)) => {
            ctx.vars.insert(ident.clone(), val.clone());
            Ok(None)
        }
        _ => {
            ctx.output
                .push_str("Syntax error: the operation `store` needs to operate on `X literal`");
            Err(())
        }
    }
}

fn fyfth_func_load(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        FyfthVariant::Literal(ident) => {
            if let Some(val) = ctx.vars.get(ident) {
                Ok(Some(val.clone()))
            } else {
                write!(
                    ctx.output,
                    "Error: no variable of the name `{}` found",
                    &ident
                )
                .unwrap();
                Err(())
            }
        }
        _ => {
            ctx.output
                .push_str("Syntax error: the operation `load` needs to operate on `literal`");
            Err(())
        }
    }
}

fn fyfth_func_print_vars(
    ctx: FyfthContext,
    args: &[FyfthVariant],
) -> Result<Option<FyfthVariant>, ()> {
    let [] = args else {
        panic!("received the wrong number of arguments")
    };
    for (ident, val) in ctx.vars.iter() {
        write!(ctx.output, "\"{ident}\" : ").unwrap();
        val.pretty_print(ctx.output, ctx.world, ctx.lang);
        ctx.output.push('\n');
    }
    Ok(None)
}

/// `lhs: Num`, `rhs: Num`
fn fyfth_func_geq(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Bool(lhs >= rhs)))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `geq` needs to operate on two `bool` types."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `lhs: Num`, `rhs: Num`
fn fyfth_func_leq(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Bool(lhs <= rhs)))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `leq` needs to operate on two `bool` types."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `lhs: X`, `rhs: Y`
fn fyfth_func_eq(_ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    Ok(Some(FyfthVariant::Bool(lhs == rhs)))
}

/// `val: bool`
fn fyfth_func_not(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        &FyfthVariant::Bool(cond) => Ok(Some(FyfthVariant::Bool(!cond))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `not` needs to operate on `bool`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `val: Entity`
fn fyfth_func_name(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        &FyfthVariant::Entity(entity) => {
            let mut query = ctx
                .world
                .query_filtered::<&Name, Without<FyfthIgnoreEntity>>();

            if let Some(name) = query
                .get(ctx.world, entity)
                .ok()
                .map(|n| FyfthVariant::Literal(n.as_str().to_string()))
            {
                Ok(Some(name))
            } else {
                Ok(Some(FyfthVariant::Nil))
            }
        }
        FyfthVariant::Component(dyn_comp) => Ok(Some(FyfthVariant::Literal(
            dyn_comp.0.reflect_type_path().to_string(),
        ))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `name` needs to operate on `Entity`."
            )
            .unwrap();
            Err(())
        }
    }
}

// `val: X`
fn fyfth_func_pop(_ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [_val] = args else {
        panic!("received the wrong number of arguments")
    };
    Ok(None)
}

/// `lhs: iter`, `rhs: num`
fn fyfth_func_index(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (FyfthVariant::Iter(vec), &FyfthVariant::Num(index)) => {
            let index = if index >= 0.0 {
                index as i32
            } else {
                vec.len() as i32 + index as i32
            };

            if 0 <= index && index < vec.len() as i32 {
                Ok(Some(vec[index as usize].clone()))
            } else {
                write!(
                    ctx.output,
                    "Error: index `{index}` out of range for an iterator of length {}.",
                    vec.len()
                )
                .unwrap();
                Err(())
            }
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `index` needs to operate on `iter num`."
            )
            .unwrap();
            Err(())
        }
    }
}

fn fyfth_func_enum(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        FyfthVariant::Iter(vec) => Ok(Some(FyfthVariant::Iter(
            (0..vec.len())
                .map(|i| FyfthVariant::Num(i as f32))
                .collect(),
        ))),
        &FyfthVariant::Num(num) => {
            if 0.0 <= num && num <= 1_000_000.0 {
                Ok(Some(FyfthVariant::Iter(
                    (0..(num as i32))
                        .map(|i| FyfthVariant::Num(i as f32))
                        .collect(),
                )))
            } else {
                write!(ctx.output, "Error: {num} is not a valid `enum` range").unwrap();
                Err(())
            }
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `enum` needs to operate on `iter` or `num`."
            )
            .unwrap();
            Err(())
        }
    }
}

fn fyfth_func_len(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        FyfthVariant::Iter(vec) => Ok(Some(FyfthVariant::Num(vec.len() as f32))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `len` needs to operate on `iter`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `val: X`
fn fyfth_func_type(_ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    let mut type_name = String::new();
    val.pretty_print_type(&mut type_name);
    Ok(Some(FyfthVariant::Literal(type_name)))
}

fn fyfth_func_append(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (FyfthVariant::Iter(vec), val) => {
            let mut new_vec = Vec::with_capacity(vec.len() + 1);
            new_vec.extend_from_slice(&vec);
            new_vec.push(val.clone());
            Ok(Some(FyfthVariant::Iter(new_vec)))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `append` needs to operate on `iter X`."
            )
            .unwrap();
            Err(())
        }
    }
}

fn fyfth_func_extend(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (FyfthVariant::Iter(lhs), FyfthVariant::Iter(rhs)) => {
            let mut new_vec = Vec::with_capacity(lhs.len() + rhs.len());
            new_vec.extend_from_slice(&lhs);
            new_vec.extend_from_slice(&rhs);
            Ok(Some(FyfthVariant::Iter(new_vec)))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `extend` needs to operate on two `iter` types."
            )
            .unwrap();
            Err(())
        }
    }
}

fn fyfth_func_reverse(
    ctx: FyfthContext,
    args: &[FyfthVariant],
) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        FyfthVariant::Iter(vec) => Ok(Some(FyfthVariant::Iter(
            vec.iter().rev().cloned().collect(),
        ))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `reverse` needs to operate on `iter`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `lhs: X`, `rhs: bool`
fn fyfth_func_filter(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (val, &FyfthVariant::Bool(cond)) => {
            if cond {
                Ok(Some(val.clone()))
            } else {
                Ok(None)
            }
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `filter` needs to operate on `X bool`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `cond: bool` `then: X`, `else: Y`
fn fyfth_func_select(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, mhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, mhs, rhs) {
        (&FyfthVariant::Bool(cond), then_val, else_val) => {
            if cond {
                Ok(Some(then_val.clone()))
            } else {
                Ok(Some(else_val.clone()))
            }
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `select` needs to operate on `bool X Y`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `lhs: num`, `rhs: num`
fn fyfth_func_mod(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(lhs), &FyfthVariant::Num(rhs)) => Ok(Some(FyfthVariant::Num(
            ((lhs as i32) % (rhs as i32)) as f32,
        ))),
        (_, FyfthVariant::Num(_)) => Ok(Some(FyfthVariant::Nil)),
        _ => {
            write!(
                    ctx.output,
                    "Syntax error: the operation `mod` needs to operate on `X num`. If X is not num it will return nil."
                )
                .unwrap();
            Err(())
        }
    }
}

/// `lhs: num`, `rhs: num`
fn fyfth_func_vec2(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(x), &FyfthVariant::Num(y)) => {
            Ok(Some(FyfthVariant::Vec2(Vec2::new(x, y))))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `vec2` needs to operate on `num num`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `num num num`
fn fyfth_func_vec3(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, mhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, mhs, rhs) {
        (&FyfthVariant::Num(x), &FyfthVariant::Num(y), &FyfthVariant::Num(z)) => {
            Ok(Some(FyfthVariant::Vec3(Vec3::new(x, y, z))))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `vec3` needs to operate on `num num num`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `num num num num`
fn fyfth_func_quat(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [x, y, z, w] = args else {
        panic!("received the wrong number of arguments")
    };
    match (x, y, z, w) {
        (
            &FyfthVariant::Num(x),
            &FyfthVariant::Num(y),
            &FyfthVariant::Num(z),
            &FyfthVariant::Num(w),
        ) => Ok(Some(FyfthVariant::Quat(
            Quat::from_xyzw(x, y, z, w).normalize(),
        ))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `quat` needs to operate on `num num num num`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `lhs: literal`, `rhs: literal`
fn fyfth_func_fuzzy(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (FyfthVariant::Literal(lhs), FyfthVariant::Literal(rhs)) => {
            Ok(Some(FyfthVariant::Bool(util::fuzzy_match(&lhs, &rhs))))
        }
        (_, FyfthVariant::Literal(_)) => Ok(Some(FyfthVariant::Bool(false))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `fuzzy` needs to operate on `X literal`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `lhs: literal`, `rhs: literal`
fn fyfth_func_regex(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (FyfthVariant::Literal(lhs), FyfthVariant::Literal(rhs)) => {
            match Regex::new(&rhs) {
                Ok(re) => {
                    if re.is_match(&lhs) {
                        // add the captures as variables
                        for capture_name in re.capture_names().filter_map(|n| n) {
                            let captures: Vec<_> = re
                                .captures_iter(&lhs)
                                .filter_map(|c| c.name(capture_name))
                                .map(|m| m.as_str())
                                .collect();

                            match captures.len() {
                                // don't do anything
                                0 => {}
                                // put it in a var as a literal
                                1 => {
                                    ctx.vars.insert(
                                        capture_name.to_string(),
                                        FyfthVariant::Literal(captures[0].to_string()),
                                    );
                                }
                                // put an iterator of all of them into the var
                                _ => {
                                    ctx.vars.insert(
                                        capture_name.to_string(),
                                        FyfthVariant::Iter(
                                            captures
                                                .iter()
                                                .map(|&c| FyfthVariant::Literal(c.to_string()))
                                                .collect(),
                                        ),
                                    );
                                }
                            }
                        }
                        Ok(Some(FyfthVariant::Bool(true)))
                    } else {
                        Ok(Some(FyfthVariant::Bool(false)))
                    }
                }
                Err(err) => {
                    write!(ctx.output, "Error: failed to parse regex: {err}").unwrap();
                    Err(())
                }
            }
            // Ok(Some(FyfthVariant::Bool(matcher.fuzzy_match(&lhs, &rhs).is_some())))
        }
        (_, FyfthVariant::Literal(_)) => Ok(Some(FyfthVariant::Bool(false))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `regex` needs to operate on `X literal`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `val: num`
fn fyfth_func_sin(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        &FyfthVariant::Num(val) => Ok(Some(FyfthVariant::Num(val.sin()))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `sin` needs to operate on `num`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `val: num`
fn fyfth_func_cos(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        &FyfthVariant::Num(val) => Ok(Some(FyfthVariant::Num(val.cos()))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `cos` needs to operate on `num`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `val: num`
fn fyfth_func_tan(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        &FyfthVariant::Num(val) => Ok(Some(FyfthVariant::Num(val.tan()))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `tan` needs to operate on `num`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `val: num`
fn fyfth_func_atan(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        &FyfthVariant::Num(val) => Ok(Some(FyfthVariant::Num(val.atan()))),
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `atan` needs to operate on `num`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `lhs: num, rhs: num`
fn fyfth_func_atan2(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Num(lhs.atan2(rhs))))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `atan` needs to operate on `num num`."
            )
            .unwrap();
            Err(())
        }
    }
}

/// `lhs: num, rhs: num`
fn fyfth_func_mul(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Num(lhs * rhs)))
        }
        (&FyfthVariant::Vec2(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Vec2(lhs * rhs)))
        }
        (&FyfthVariant::Vec3(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Vec3(lhs * rhs)))
        }
        (&FyfthVariant::Num(lhs), &FyfthVariant::Vec2(rhs)) => {
            Ok(Some(FyfthVariant::Vec2(lhs * rhs)))
        }
        (&FyfthVariant::Num(lhs), &FyfthVariant::Vec3(rhs)) => {
            Ok(Some(FyfthVariant::Vec3(lhs * rhs)))
        }
        (&FyfthVariant::Vec2(lhs), &FyfthVariant::Vec2(rhs)) => {
            Ok(Some(FyfthVariant::Vec2(lhs * rhs)))
        }
        (&FyfthVariant::Vec3(lhs), &FyfthVariant::Vec3(rhs)) => {
            Ok(Some(FyfthVariant::Vec3(lhs * rhs)))
        }
        (&FyfthVariant::Quat(lhs), &FyfthVariant::Quat(rhs)) => {
            Ok(Some(FyfthVariant::Quat(lhs * rhs)))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `mul` cannot work on types `"
            )
            .unwrap();
            lhs.pretty_print_type(ctx.output);
            ctx.output.push_str(" ");
            rhs.pretty_print_type(ctx.output);
            ctx.output.push_str("`.");
            Err(())
        }
    }
}

/// `lhs: num, rhs: num`
fn fyfth_func_div(ctx: FyfthContext, args: &[FyfthVariant]) -> Result<Option<FyfthVariant>, ()> {
    let [lhs, rhs] = args else {
        panic!("received the wrong number of arguments")
    };
    match (lhs, rhs) {
        (&FyfthVariant::Num(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Num(lhs / rhs)))
        }
        (&FyfthVariant::Vec2(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Vec2(lhs / rhs)))
        }
        (&FyfthVariant::Vec3(lhs), &FyfthVariant::Num(rhs)) => {
            Ok(Some(FyfthVariant::Vec3(lhs / rhs)))
        }
        (&FyfthVariant::Vec2(lhs), &FyfthVariant::Vec2(rhs)) => {
            Ok(Some(FyfthVariant::Vec2(lhs / rhs)))
        }
        (&FyfthVariant::Vec3(lhs), &FyfthVariant::Vec3(rhs)) => {
            Ok(Some(FyfthVariant::Vec3(lhs / rhs)))
        }
        _ => {
            write!(
                ctx.output,
                "Syntax error: the operation `div` cannot work on types `"
            )
            .unwrap();
            lhs.pretty_print_type(ctx.output);
            ctx.output.push_str(" ");
            rhs.pretty_print_type(ctx.output);
            ctx.output.push_str("`.");
            Err(())
        }
    }
}

//--------------------------------------------------
// Prefix Implementations
//--------------------------------------------------

fn fyfth_prefix_load(word: &str, lang: &FyfthLanguageExtension) -> Result<Vec<FyfthVariant>, ()> {
    Ok(vec![
        FyfthVariant::Literal(word.to_string()),
        FyfthVariant::LangFunc(lang.get_command_id("load").ok_or(())?),
    ])
}

fn fyfth_prefix_queue_macro(
    word: &str,
    lang: &FyfthLanguageExtension,
) -> Result<Vec<FyfthVariant>, ()> {
    Ok(vec![
        FyfthVariant::Literal(word.to_string()),
        FyfthVariant::LangFunc(lang.get_command_id("load").ok_or(())?),
        FyfthVariant::FnQueue,
    ])
}

fn fyfth_prefix_fuzzy_entity(
    word: &str,
    lang: &FyfthLanguageExtension,
) -> Result<Vec<FyfthVariant>, ()> {
    use FyfthVariant::*;
    Ok(vec![
        Literal(word.to_string()),
        Literal("fuzzent".to_string()),
        LangFunc(lang.get_command_id("load").ok_or(())?),
        FnQueue,
        Num(0.0),
        LangFunc(lang.get_command_id("index").ok_or(())?),
    ])
}
