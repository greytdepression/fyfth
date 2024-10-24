use core::any::TypeId;
use std::{collections::VecDeque, fmt::Write, sync::Arc};

use bevy::{prelude::*, utils::HashMap};

use crate::{
    bevy_component::{BevyComponentRegistry, DynBevyComponent},
    language::{FnInfo, FyfthBroadcastBehavior, FyfthLanguageExtension},
    lexer::{FyfthLexer, FyfthWord},
};

#[derive(Clone, Resource)]
pub struct FyfthInterpreter {
    stack: Vec<FyfthVariant>,
    queue: VecDeque<FyfthVariant>,
    vars: HashMap<String, FyfthVariant>,
    lang: Arc<FyfthLanguageExtension>,
}

impl FyfthInterpreter {
    pub(crate) fn new_with_prelude(path: &str, world: &mut World) -> Self {
        let mut output = Self {
            stack: default(),
            queue: default(),
            vars: default(),
            lang: Arc::new(FyfthLanguageExtension::new()),
        };

        let prelude = std::fs::read_to_string(path).unwrap();
        output.parse_code(&prelude);

        let (_, res) = FyfthVariant::run(&mut output, world);

        res.unwrap();

        output
    }

    pub(crate) fn pretty_print_stack(
        &self,
        world: &World,
        delimiter: &str,
        lang: &FyfthLanguageExtension,
    ) -> String {
        let mut buffer = String::new();

        let mut first = true;
        for val in self.stack.iter() {
            if !first {
                write!(&mut buffer, "{delimiter}").unwrap();
            }
            val.pretty_print(&mut buffer, world, lang);
            first = false;
        }

        buffer
    }

    pub(crate) fn parse_code(&mut self, code: &str) {
        let lexer = FyfthLexer::iter(code, self.lang.clone());
        for res in lexer {
            let word = res.unwrap();
            FyfthVariant::parse(self, word);
        }
    }
}

pub struct FyfthContext<'a> {
    pub output: &'a mut String,
    pub world: &'a mut World,
    pub vars: &'a mut HashMap<String, FyfthVariant>,
    pub lang: &'a FyfthLanguageExtension,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FyfthVariant {
    // Non-executable
    Nil,
    Bool(bool),
    Num(f32),
    Literal(String),
    Iter(Vec<FyfthVariant>),

    // bevy specific
    Entity(Entity),
    Vec2(Vec2),
    Vec3(Vec3),
    Quat(Quat),
    Component(DynBevyComponent),

    // Executable
    FnIter,
    Macro,
    LineEnd,
    FnQueue,
    FnPush,
    FnDup,
    FnSwap,
    FnSwapN,
    FnRotRN,
    FnRotLN,
    LangFunc(u32),
}

impl FyfthVariant {
    pub(crate) fn parse(interpreter: &mut FyfthInterpreter, word: FyfthWord) {
        let FyfthInterpreter { queue, lang, .. } = interpreter;
        let lang = Arc::clone(lang);

        if let Some(prefix_index) = word.maybe_prefix {
            // TODO: handle errors gracefully
            let res = (lang.prefixes[prefix_index as usize].fn_ptr)(&word.word, &lang).unwrap();

            queue.extend(res);
            return;
        }

        if word.in_quotes {
            queue.push_back(Self::Literal(word.word));
            return;
        }

        let command = word.word;

        // check if it's a number
        if let Ok(val) = command.parse() {
            queue.push_back(Self::Num(val));
            return;
        }

        match command.as_str() {
            "nil" => queue.push_back(Self::Nil),
            "iter" => queue.push_back(Self::FnIter),
            "true" => queue.push_back(Self::Bool(true)),
            "false" => queue.push_back(Self::Bool(false)),
            "macro" => queue.push_back(Self::Macro),
            "queue" => queue.push_back(Self::FnQueue),
            "dup" => queue.push_back(Self::FnDup),
            "swap" => queue.push_back(Self::FnSwap),
            ";" => queue.push_back(Self::LineEnd),
            "swap_n" => queue.push_back(Self::FnSwapN),
            "rotr" => queue.push_back(Self::FnRotRN),
            "rotl" => queue.push_back(Self::FnRotLN),
            "push" => queue.push_back(Self::FnPush),

            _ if lang.keywords.contains_key(&command) => {
                let index = *lang.keywords.get(&command).unwrap();
                queue.push_back(Self::LangFunc(index));
            }
            _ => queue.push_back(Self::Literal(command)),
        }
    }

    pub(crate) fn as_num(&self) -> f32 {
        match self {
            FyfthVariant::Num(val) => *val,
            _ => unreachable!(),
        }
    }

    pub(crate) fn run(
        interpreter: &mut FyfthInterpreter,
        world: &mut World,
    ) -> (String, Result<(), ()>) {
        let mut output = String::new();

        let FyfthInterpreter {
            stack,
            queue,
            vars,
            lang,
        } = interpreter;

        let lang = Arc::clone(lang);

        let mut result = Ok(());

        let mut iterations = 0;

        let vars = &mut interpreter.vars;

        let mut print_out = String::new();

        let debug_print_state = |print_out: &mut String,
                                 i: i32,
                                 stack: &[FyfthVariant],
                                 queue: &VecDeque<FyfthVariant>,
                                 vars: &HashMap<String, FyfthVariant>,
                                 world: &World| {
            // write!(print_out, "{i: >5}:\n").unwrap();
            // print_out.push_str("  Stack: ");
            // for val in stack.iter() {
            //     val.pretty_print(print_out, world);
            //     print_out.push(' ');
            // }
            // print_out.push_str("\n  Queue: ");
            // for val in queue.iter() {
            //     val.pretty_print(print_out, world);
            //     print_out.push(' ');
            // }
            // print_out.push_str("\n  Variables:\n");
            // for (name, val) in vars.iter() {
            //     write!(print_out, "\"{name}\": ").unwrap();
            //     val.pretty_print(print_out, world);
            //     print_out.push('\n');
            // }
            // print_out.push_str("\n\n");
        };

        while !queue.is_empty() && result.is_ok() {
            debug_print_state(&mut print_out, iterations, stack, queue, vars, world);
            if iterations > 100_000 {
                write!(&mut output, "Error: reached iteration limit").unwrap();
                result = Err(());
                break;
            }
            iterations += 1;

            let current = queue.pop_front().unwrap();
            match &current {
                // These are all the values which cannot be executed. Simply push them onto the stack.
                FyfthVariant::Bool(_)
                | FyfthVariant::Num(_)
                | FyfthVariant::Literal(_)
                | FyfthVariant::Entity(_)
                | FyfthVariant::Iter(_)
                | FyfthVariant::Nil => {
                    stack.push(current);
                    continue;
                }

                FyfthVariant::LineEnd => continue,

                // All other values can be executed and so we continue below
                _ => {}
            }

            result = match current {
                FyfthVariant::FnIter => {
                    let mut iter_vec = vec![];
                    std::mem::swap(&mut iter_vec, stack);

                    stack.push(Self::Iter(iter_vec));
                    Ok(())
                }
                FyfthVariant::Macro => {
                    if let Some(Self::Literal(name)) = queue.pop_front() {
                        let mut counter = 1;
                        let mut end = 0;
                        for val in queue.iter() {
                            match val {
                                FyfthVariant::Macro => counter += 1,
                                FyfthVariant::LineEnd => {
                                    counter -= 1;
                                    if counter <= 0 {
                                        assert_eq!(counter, 0);
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            end += 1;
                        }

                        vars.insert(name, Self::Iter(queue.drain(0..end).collect()));
                        Ok(())
                    } else {
                        write!(
                            &mut output,
                            "Syntax error: `macro` needs to be followed by a name for the macro"
                        )
                        .unwrap();
                        Err(())
                    }
                }
                FyfthVariant::FnQueue => {
                    if let Some(Self::Iter(vals)) = stack.pop() {
                        for val in vals.iter().rev() {
                            queue.push_front(val.clone());
                        }
                        Ok(())
                    } else {
                        write!(
                            &mut output,
                            "Syntax error: `run` expects the top of the stack to be `iter`"
                        )
                        .unwrap();
                        Err(())
                    }
                }
                FyfthVariant::FnPush => {
                    if let Some(Self::Iter(vals)) = stack.pop() {
                        stack.extend_from_slice(&vals);
                        Ok(())
                    } else {
                        write!(
                            &mut output,
                            "Syntax error: `push` expects the top of the stack to be `iter`"
                        )
                        .unwrap();
                        Err(())
                    }
                }
                FyfthVariant::FnDup => {
                    if let Some(val) = stack.last() {
                        stack.push(val.clone());
                    }
                    Ok(())
                }
                FyfthVariant::FnSwap => {
                    let first = stack.pop();
                    let second = stack.pop();

                    match (first, second) {
                        (Some(first), Some(second)) => {
                            stack.push(first);
                            stack.push(second);
                            Ok(())
                        }
                        _ => {
                            write!(
                                &mut output,
                                "Syntax error: `swap` expects two items on the stack"
                            )
                            .unwrap();
                            Err(())
                        }
                    }
                }
                FyfthVariant::FnSwapN => {
                    if let Some(Self::Num(index)) = stack.pop() {
                        let index = index as i32;
                        if index < 0 || index as usize + 1 >= stack.len() {
                            output
                                .push_str("Error: not enough items on the stack to apply `swap_n`");
                            Err(())
                        } else if index == 0 {
                            Ok(())
                        } else {
                            assert!(!stack.is_empty());
                            let rhs_index = stack.len() - 1;
                            let lhs_index = stack.len() - 1 - index as usize;

                            stack.swap(lhs_index, rhs_index);
                            Ok(())
                        }
                    } else {
                        output.push_str("Syntax error: `swap_n` must follow a number");
                        Err(())
                    }
                }
                FyfthVariant::FnRotRN => {
                    if let Some(Self::Num(index)) = stack.pop() {
                        let size = index as i32;
                        if size < 0 || size as usize > stack.len() {
                            output.push_str("Error: not enough items on the stack to apply `rotr`");
                            Err(())
                        } else if size <= 1 {
                            Ok(())
                        } else {
                            assert!(!stack.is_empty());
                            let lhs_index = stack.len() - size as usize;
                            let last = stack.pop().unwrap();
                            stack.insert(lhs_index, last);
                            Ok(())
                        }
                    } else {
                        output.push_str("Syntax error: `rotr` must follow a number");
                        Err(())
                    }
                }
                FyfthVariant::FnRotLN => {
                    if let Some(Self::Num(index)) = stack.pop() {
                        let size = index as i32;
                        if size < 0 || size as usize > stack.len() {
                            output.push_str("Error: not enough items on the stack to apply `rotl`");
                            Err(())
                        } else if size <= 1 {
                            Ok(())
                        } else {
                            assert!(!stack.is_empty());
                            let lhs_index = stack.len() - size as usize;
                            let temp = stack.remove(lhs_index);
                            stack.push(temp);
                            Ok(())
                        }
                    } else {
                        output.push_str("Syntax error: `rotl` must follow a number");
                        Err(())
                    }
                }
                FyfthVariant::LangFunc(index) => Self::try_call_func(
                    FyfthContext {
                        output: &mut output,
                        world,
                        vars,
                        lang: &lang,
                    },
                    &lang.functions[index as usize],
                    stack,
                ),
                _ => todo!(),
            };
        }

        debug_print_state(&mut print_out, iterations, stack, queue, vars, world);
        // println!("{}", print_out);

        (output, result)
    }

    pub(crate) fn try_call_func(
        ctx: FyfthContext,
        func: &FnInfo,
        stack: &mut Vec<FyfthVariant>,
    ) -> Result<(), ()> {
        let arity = func.simple_function.broadcast_behaviors.len();

        if stack.len() < arity {
            write!(
                ctx.output,
                "Syntax error: function `{}` expects {} arguments but stack has only {} items",
                &func.keyword,
                arity,
                stack.len(),
            )
            .unwrap();
            return Err(());
        }

        let args = stack.split_off(stack.len() - arity);
        let wants_to_iter = func
            .simple_function
            .broadcast_behaviors
            .iter()
            .enumerate()
            .any(|(index, &beh)| match (beh, &args[index]) {
                (FyfthBroadcastBehavior::MayIter, Self::Iter(_)) => true,
                (FyfthBroadcastBehavior::MayIter, _) => false,
                (FyfthBroadcastBehavior::IgnoreIter, _) => false,
            });

        let maybe_output = if !wants_to_iter {
            (func.simple_function.fn_ptr)(ctx, &args)?
        } else {
            // make sure all iters have the same length
            let min_len = func
                .simple_function
                .broadcast_behaviors
                .iter()
                .zip(args.iter())
                .filter_map(|(beh, arg)| match (beh, arg) {
                    (FyfthBroadcastBehavior::MayIter, Self::Iter(v)) => Some(v.len()),
                    _ => None,
                })
                .min()
                .unwrap();
            let max_len = func
                .simple_function
                .broadcast_behaviors
                .iter()
                .zip(args.iter())
                .filter_map(|(beh, arg)| match (beh, arg) {
                    (FyfthBroadcastBehavior::MayIter, Self::Iter(v)) => Some(v.len()),
                    _ => None,
                })
                .max()
                .unwrap();

            if min_len != max_len {
                write!(
                    ctx.output,
                    "Error: function `{}` cannot combine iterators of differing lenths.",
                    &func.keyword,
                )
                .unwrap();
                return Err(());
            }

            let len = min_len;

            let mut output_vec = Vec::with_capacity(len);
            let mut temp_args = Vec::with_capacity(arity);

            let FyfthContext {
                output,
                world,
                vars,
                lang,
            } = ctx;

            for i in 0..len {
                temp_args.clear();
                for j in 0..arity {
                    let arg = &args[j];
                    let beh = &func.simple_function.broadcast_behaviors[j];

                    temp_args.push(match (beh, arg) {
                        (FyfthBroadcastBehavior::MayIter, Self::Iter(v)) => v[i].clone(),
                        _ => arg.clone(),
                    });
                }

                let maybe_value = (func.simple_function.fn_ptr)(
                    FyfthContext {
                        output,
                        world,
                        vars,
                        lang,
                    },
                    &temp_args,
                )?;

                if let Some(val) = maybe_value {
                    output_vec.push(val);
                }
            }

            Some(Self::Iter(output_vec))
        };

        if let Some(result_value) = maybe_output {
            stack.push(result_value);
        }

        Ok(())
    }

    pub(crate) fn pretty_print(
        &self,
        output: &mut String,
        world: &World,
        lang: &FyfthLanguageExtension,
    ) {
        match self {
            FyfthVariant::Nil => write!(output, "nil").unwrap(),
            FyfthVariant::Bool(val) => write!(output, "{val}").unwrap(),
            FyfthVariant::Num(val) => write!(output, "{val}").unwrap(),
            FyfthVariant::Literal(val) => write!(output, "\"{val}\"").unwrap(),
            FyfthVariant::Entity(entity) => {
                let maybe_name = (|| world.get_entity(*entity)?.get::<Name>())();

                if let Some(name) = maybe_name {
                    write!(output, "({entity} - \"{}\")", name.as_str()).unwrap()
                } else {
                    write!(output, "({entity})").unwrap()
                }
            }
            FyfthVariant::Iter(vec) => {
                write!(output, "[{} items; ", vec.len()).unwrap();
                let mut first = true;
                for item in vec {
                    if !first {
                        write!(output, ", ").unwrap();
                    }
                    item.pretty_print(output, world, lang);
                    first = false;
                }
                write!(output, "]").unwrap();
            }
            FyfthVariant::Vec2(val) => write!(output, "vec2({} {})", val.x, val.y).unwrap(),
            FyfthVariant::Vec3(val) => {
                write!(output, "vec3({} {} {})", val.x, val.y, val.z).unwrap()
            }
            FyfthVariant::Quat(val) => {
                write!(output, "quat({} {} {} {})", val.x, val.y, val.z, val.w).unwrap()
            }
            FyfthVariant::Component(comp) => write!(output, "{comp}").unwrap(),
            FyfthVariant::FnIter => write!(output, "iter").unwrap(),
            FyfthVariant::Macro => write!(output, "macro").unwrap(),
            FyfthVariant::LineEnd => write!(output, ";").unwrap(),
            FyfthVariant::FnQueue => write!(output, "queue").unwrap(),
            FyfthVariant::FnDup => write!(output, "dup").unwrap(),
            FyfthVariant::FnSwap => write!(output, "swap").unwrap(),
            FyfthVariant::FnSwapN => write!(output, "swap_n").unwrap(),
            FyfthVariant::FnRotRN => write!(output, "rotr").unwrap(),
            FyfthVariant::FnRotLN => write!(output, "rotl").unwrap(),
            FyfthVariant::FnPush => write!(output, "push").unwrap(),
            // TODO: print the keyword of the function
            //       requires some API changes for this function
            FyfthVariant::LangFunc(index) => {
                write!(output, "{}", &lang.functions[*index as usize].keyword).unwrap()
            }
        }
    }

    pub(crate) fn pretty_print_type(&self, output: &mut String) {
        match self {
            FyfthVariant::Nil => write!(output, "nil").unwrap(),
            FyfthVariant::Bool(_) => write!(output, "bool").unwrap(),
            FyfthVariant::Num(_) => write!(output, "num").unwrap(),
            FyfthVariant::Literal(_) => write!(output, "literal").unwrap(),
            FyfthVariant::Entity(_) => write!(output, "Entity").unwrap(),
            FyfthVariant::Iter(_) => write!(output, "iter").unwrap(),
            FyfthVariant::Vec2(_) => write!(output, "vec2").unwrap(),
            FyfthVariant::Vec3(_) => write!(output, "vec3").unwrap(),
            FyfthVariant::Quat(_) => write!(output, "quat").unwrap(),
            FyfthVariant::Component(comp) => write!(
                output,
                "{}",
                comp.0.reflect_type_ident().unwrap_or("anonymous component")
            )
            .unwrap(),
            FyfthVariant::FnIter => write!(output, "func").unwrap(),
            FyfthVariant::Macro => write!(output, "special").unwrap(),
            FyfthVariant::LineEnd => write!(output, "special").unwrap(),
            FyfthVariant::FnQueue => write!(output, "func").unwrap(),
            FyfthVariant::FnDup => write!(output, "func").unwrap(),
            FyfthVariant::FnSwap => write!(output, "func").unwrap(),
            FyfthVariant::FnSwapN => write!(output, "func").unwrap(),
            FyfthVariant::FnRotRN => write!(output, "func").unwrap(),
            FyfthVariant::FnRotLN => write!(output, "func").unwrap(),
            FyfthVariant::FnPush => write!(output, "func").unwrap(),
            FyfthVariant::LangFunc(_) => write!(output, "func").unwrap(),
        }
    }

    pub(crate) fn try_reflect_from_type_id(
        value: &dyn Reflect,
        registry: &BevyComponentRegistry,
    ) -> Option<Self> {
        match value.type_id() {
            id if id == TypeId::of::<bool>() => {
                return Some(Self::Bool(value.downcast_ref::<bool>().unwrap().clone()));
            }
            id if id == TypeId::of::<f32>() => {
                return Some(Self::Num(value.downcast_ref::<f32>().unwrap().clone()));
            }
            id if id == TypeId::of::<f64>() => {
                return Some(Self::Num(
                    value.downcast_ref::<f64>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<i8>() => {
                return Some(Self::Num(value.downcast_ref::<i8>().unwrap().clone() as f32));
            }
            id if id == TypeId::of::<i16>() => {
                return Some(Self::Num(
                    value.downcast_ref::<i16>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<i32>() => {
                return Some(Self::Num(
                    value.downcast_ref::<i32>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<i64>() => {
                return Some(Self::Num(
                    value.downcast_ref::<i64>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<isize>() => {
                return Some(Self::Num(
                    value.downcast_ref::<isize>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<u8>() => {
                return Some(Self::Num(value.downcast_ref::<u8>().unwrap().clone() as f32));
            }
            id if id == TypeId::of::<u16>() => {
                return Some(Self::Num(
                    value.downcast_ref::<u16>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<u32>() => {
                return Some(Self::Num(
                    value.downcast_ref::<u32>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<u64>() => {
                return Some(Self::Num(
                    value.downcast_ref::<u64>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<usize>() => {
                return Some(Self::Num(
                    value.downcast_ref::<usize>().unwrap().clone() as f32
                ));
            }
            id if id == TypeId::of::<String>() => {
                return Some(Self::Literal(
                    value.downcast_ref::<String>().unwrap().clone(),
                ));
            }
            id if id == TypeId::of::<&str>() => {
                return Some(Self::Literal(
                    value.downcast_ref::<&str>().unwrap().to_string(),
                ));
            }
            id if id == TypeId::of::<Entity>() => {
                return Some(Self::Entity(
                    value.downcast_ref::<Entity>().unwrap().clone(),
                ));
            }
            id if id == TypeId::of::<Vec2>() => {
                return Some(Self::Vec2(value.downcast_ref::<Vec2>().unwrap().clone()));
            }
            id if id == TypeId::of::<Vec3>() => {
                return Some(Self::Vec3(value.downcast_ref::<Vec3>().unwrap().clone()));
            }
            id if id == TypeId::of::<Quat>() => {
                return Some(Self::Quat(value.downcast_ref::<Quat>().unwrap().clone()));
            }
            _ => {}
        }

        for info in registry.registered_components.iter() {
            if info.type_id == value.type_id() {
                let from_reflect_func = &registry
                    .registered_components_map
                    .get(&info.type_id)
                    .unwrap()
                    .from_reflect;

                return Some(Self::Component((from_reflect_func)(value).unwrap()));
            }
        }

        None
    }

    pub(crate) fn try_set_reflect_field(
        &self,
        value: &mut dyn Struct,
        field_name: &str,
    ) -> Result<(), ()> {
        match self {
            FyfthVariant::Bool(val) => {
                *value.get_field_mut(field_name).ok_or(())? = *val;
                return Ok(());
            }
            FyfthVariant::Literal(val) => {
                *value.get_field_mut(field_name).ok_or(())? = val.clone();
                return Ok(());
            }
            FyfthVariant::Entity(val) => {
                *value.get_field_mut(field_name).ok_or(())? = *val;
                return Ok(());
            }
            FyfthVariant::Vec2(val) => {
                *value.get_field_mut(field_name).ok_or(())? = *val;
                return Ok(());
            }
            FyfthVariant::Vec3(val) => {
                *value.get_field_mut(field_name).ok_or(())? = *val;
                return Ok(());
            }
            FyfthVariant::Quat(val) => {
                *value.get_field_mut(field_name).ok_or(())? = *val;
                return Ok(());
            }
            FyfthVariant::Num(val) => {
                return match value.field(field_name).ok_or(())?.type_id() {
                    id if id == TypeId::of::<f32>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as f32;
                        Ok(())
                    }
                    id if id == TypeId::of::<f64>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as f64;
                        Ok(())
                    }
                    id if id == TypeId::of::<i8>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as i8;
                        Ok(())
                    }
                    id if id == TypeId::of::<i16>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as i16;
                        Ok(())
                    }
                    id if id == TypeId::of::<i32>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as i32;
                        Ok(())
                    }
                    id if id == TypeId::of::<i64>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as i64;
                        Ok(())
                    }
                    id if id == TypeId::of::<isize>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as isize;
                        Ok(())
                    }
                    id if id == TypeId::of::<u8>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as u8;
                        Ok(())
                    }
                    id if id == TypeId::of::<u16>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as u16;
                        Ok(())
                    }
                    id if id == TypeId::of::<u32>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as u32;
                        Ok(())
                    }
                    id if id == TypeId::of::<u64>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as u64;
                        Ok(())
                    }
                    id if id == TypeId::of::<usize>() => {
                        *value.get_field_mut(field_name).unwrap() = *val as usize;
                        Ok(())
                    }
                    _ => Err(()),
                }
            }
            FyfthVariant::Component(dyn_shell_component) => todo!(),
            FyfthVariant::Iter(vec) => Err(())?,
            FyfthVariant::Nil => panic!("Cannot set field to nil"),
            _ => panic!("Cannot set field to non-value type `FyfthVariant`"),
        }

        Err(())
    }
}
