use std::fmt::Write;

use fyfth_core::interpreter::{FyfthContext, FyfthVariant};

/// `val: Entity`
pub(crate) fn fyfth_func_focus(
    ctx: FyfthContext,
    args: &[FyfthVariant],
) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        &FyfthVariant::Entity(entity) => {
            if let Some(mut entity) = ctx.world.get_entity_mut(entity) {
                entity.insert(crate::FyfthFocusObject);
                Ok(None)
            } else {
                write!(ctx.output, "Error: entity ({entity}) no longer exists.").unwrap();
                Err(())
            }
        }
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

// `val: Entity`
pub(crate) fn fyfth_func_unfocus(
    ctx: FyfthContext,
    args: &[FyfthVariant],
) -> Result<Option<FyfthVariant>, ()> {
    let [val] = args else {
        panic!("received the wrong number of arguments")
    };
    match val {
        &FyfthVariant::Entity(entity) => {
            if let Some(mut entity) = ctx.world.get_entity_mut(entity) {
                entity.remove::<crate::FyfthFocusObject>();
                Ok(None)
            } else {
                write!(ctx.output, "Error: entity ({entity}) no longer exists.").unwrap();
                Err(())
            }
        }
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

pub(crate) fn fyfth_func_focused(
    ctx: FyfthContext,
    args: &[FyfthVariant],
) -> Result<Option<FyfthVariant>, ()> {
    let [] = args else {
        panic!("received the wrong number of arguments")
    };
    let mut query = ctx.world.query::<&crate::FocusObjectAvatar>();
    let mut ent_vec = vec![];

    for av in query.iter(ctx.world) {
        ent_vec.push(FyfthVariant::Entity(av.0));
    }

    Ok(Some(FyfthVariant::Iter(ent_vec)))
}
