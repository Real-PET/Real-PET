use super::errors::{component_not_found, entity_not_found};
use super::{BattleLuaApi, COMPONENT_TABLE, UPDATE_FN};
use crate::battle::*;
use crate::bindable::*;

pub fn inject_component_api(lua_api: &mut BattleLuaApi) {
    // constructor in entity_api.rs Entity:create_component

    lua_api.add_dynamic_setter(COMPONENT_TABLE, UPDATE_FN, |api_ctx, lua, params| {
        let (table, callback): (rollback_mlua::Table, rollback_mlua::Function) =
            lua.unpack_multi(params)?;

        let id: GenerationalIndex = table.raw_get("#id")?;

        let api_ctx = &mut *api_ctx.borrow_mut();

        let component = (api_ctx.simulation.components)
            .get_mut(id.into())
            .ok_or_else(component_not_found)?;

        let key = lua.create_registry_value(table)?;
        component.update_callback = BattleCallback::new_transformed_lua_callback(
            lua,
            api_ctx.vm_index,
            callback,
            move |_, lua, _| {
                let table: rollback_mlua::Table = lua.registry_value(&key)?;
                lua.pack_multi(table)
            },
        )?;

        lua.pack_multi(())
    });

    lua_api.add_dynamic_function(COMPONENT_TABLE, "eject", |api_ctx, lua, params| {
        let table: rollback_mlua::Table = lua.unpack_multi(params)?;

        let id: GenerationalIndex = table.get("#id")?;

        let api_ctx = &mut *api_ctx.borrow_mut();

        let component = api_ctx
            .simulation
            .components
            .remove(id.into())
            .ok_or_else(component_not_found)?;

        if component.lifetime == ComponentLifetime::Local {
            let entities = &mut api_ctx.simulation.entities;

            let entity = entities
                .query_one_mut::<&mut Entity>(component.entity.into())
                .map_err(|_| entity_not_found())?;

            let arena_index: generational_arena::Index = id.into();
            let index = (entity.local_components)
                .iter()
                .position(|stored_index| *stored_index == arena_index)
                .unwrap();

            entity.local_components.swap_remove(index);
        }

        lua.pack_multi(())
    });

    lua_api.add_dynamic_function(COMPONENT_TABLE, "get_owner", |_, lua, params| {
        let table: rollback_mlua::Table = lua.unpack_multi(params)?;

        let entity_table: rollback_mlua::Table = table.get("#entity")?;

        lua.pack_multi(entity_table)
    });
}
