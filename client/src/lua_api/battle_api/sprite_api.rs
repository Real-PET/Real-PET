use super::errors::{entity_not_found, sprite_not_found};
use super::{BattleLuaApi, SPRITE_TABLE};
use crate::battle::Entity;
use crate::bindable::{EntityID, GenerationalIndex, LuaColor, LuaVector, SpriteColorMode};
use crate::lua_api::helpers::{absolute_path, inherit_metatable};
use crate::render::SpriteNode;
use framework::prelude::Vec2;

pub fn inject_sprite_api(lua_api: &mut BattleLuaApi) {
    lua_api.add_dynamic_function(SPRITE_TABLE, "create_node", |api_ctx, lua, params| {
        let table: rollback_mlua::Table = lua.unpack_multi(params)?;

        let id: EntityID = table.raw_get("#id")?;
        let index: GenerationalIndex = table.raw_get("#index")?;

        let api_ctx = &mut *api_ctx.borrow_mut();
        let entities = &mut api_ctx.simulation.entities;

        let entity = entities
            .query_one_mut::<&mut Entity>(id.into())
            .map_err(|_| entity_not_found())?;

        let child_index = entity
            .sprite_tree
            .insert_child(
                index,
                SpriteNode::new(api_ctx.game_io, SpriteColorMode::Add),
            )
            .ok_or_else(sprite_not_found)?;

        let child_table = create_sprite_table(lua, id, child_index, None)?;

        lua.pack_multi(child_table)
    });

    lua_api.add_dynamic_function(SPRITE_TABLE, "remove_node", |api_ctx, lua, params| {
        let (table, child_table): (rollback_mlua::Table, rollback_mlua::Table) =
            lua.unpack_multi(params)?;

        let id: EntityID = table.raw_get("#id")?;
        let index: GenerationalIndex = table.raw_get("#index")?;

        let child_id: EntityID = child_table.raw_get("#id")?;
        let child_index: GenerationalIndex = child_table.raw_get("#index")?;

        if id != child_id {
            return lua.pack_multi(());
        }

        let mut api_ctx = api_ctx.borrow_mut();
        let entities = &mut api_ctx.simulation.entities;

        let entity = entities
            .query_one_mut::<&mut Entity>(id.into())
            .map_err(|_| entity_not_found())?;

        // verify existence of sprite
        let sprite_node_node = entity
            .sprite_tree
            .get_node(index)
            .ok_or_else(sprite_not_found)?;

        // verify the child is a child of the sprite
        if !sprite_node_node.children().contains(&child_index) {
            return lua.pack_multi(());
        }

        entity.sprite_tree.remove(child_index);

        lua.pack_multi(())
    });

    getter(lua_api, "get_texture", |node, _, _: ()| {
        Ok(node.texture_path().to_string())
    });

    lua_api.add_dynamic_function(SPRITE_TABLE, "set_texture", move |api_ctx, lua, params| {
        let (table, path): (rollback_mlua::Table, String) = lua.unpack_multi(params)?;
        let path = absolute_path(lua, path)?;

        let id: EntityID = table.raw_get("#id")?;
        let index: GenerationalIndex = table.raw_get("#index")?;

        let api_ctx = &mut *api_ctx.borrow_mut();
        let simulation = &mut api_ctx.simulation;
        let entities = &mut simulation.entities;

        let entity = entities
            .query_one_mut::<&mut Entity>(id.into())
            .map_err(|_| entity_not_found())?;

        let sprite_node = entity
            .sprite_tree
            .get_mut(index)
            .ok_or_else(sprite_not_found)?;

        sprite_node.set_texture(api_ctx.game_io, path);

        if let Ok(animator_index) = table.raw_get::<_, GenerationalIndex>("#anim") {
            if let Some(animator) = simulation.animators.get_mut(animator_index.into()) {
                animator.apply(sprite_node);
            }
        }

        lua.pack_multi(())
    });

    setter(lua_api, "show", |node, _, _: ()| {
        node.set_visible(true);
        Ok(())
    });
    setter(lua_api, "hide", |node, _, _: ()| {
        node.set_visible(false);
        Ok(())
    });

    getter(lua_api, "get_layer", |node, _, _: ()| Ok(node.layer()));
    setter(lua_api, "set_layer", |node, _, layer| {
        node.set_layer(layer);
        Ok(())
    });

    //   "add_tags", [](WeakWrapper<SpriteProxyNode>& node, std::initializer_list<std::string> tags) {
    //     node.Unwrap()->AddTags(tags);
    //   },
    //   "remove_tags", [](WeakWrapper<SpriteProxyNode>& node, std::initializer_list<std::string> tags) {
    //     node.Unwrap()->RemoveTags(tags);
    //   },
    //   "has_tag", [](WeakWrapper<SpriteProxyNode>& node, const std::string& tag) -> bool{
    //     return node.Unwrap()->HasTag(tag);
    //   },
    //   "find_child_nodes_with_tags", [](WeakWrapper<SpriteProxyNode>& node, std::vector<std::string> tags) {
    //     auto nodes = node.Unwrap()->GetChildNodesWithTag(tags);
    //     std::vector<WeakWrapper<SceneNode>> result;
    //     result.reserve(nodes.size());

    //     for (auto node : nodes) {
    //       result.push_back(WeakWrapper(node));
    //     }

    //     return sol::as_table(result);
    //   },

    getter(lua_api, "get_offset", |node, _, _: ()| {
        Ok(LuaVector::from(node.offset()))
    });
    setter(lua_api, "set_offset", |node, _, offset: (f32, f32)| {
        node.set_offset(offset.into());
        Ok(())
    });

    getter(lua_api, "get_origin", |node, _, _: ()| {
        Ok(LuaVector::from(node.origin()))
    });
    setter(lua_api, "set_origin", |node, _, origin: (f32, f32)| {
        node.set_origin(origin.into());
        Ok(())
    });

    getter(lua_api, "get_scale", |node, _, _: ()| {
        Ok(LuaVector::from(node.scale()))
    });
    setter(lua_api, "set_scale", |node, _, scale: (f32, f32)| {
        node.set_scale(scale.into());
        Ok(())
    });

    setter(lua_api, "get_size", |node, _, _: ()| {
        Ok(LuaVector::from(node.size()))
    });
    setter(lua_api, "set_size", |node, _, size: (f32, f32)| {
        node.set_size(size.into());
        Ok(())
    });

    setter(lua_api, "get_width", |node, _, _: ()| {
        // update scale
        Ok(node.size().x)
    });
    setter(lua_api, "set_width", |node, _, width| {
        let height = node.size().y;
        node.set_size(Vec2::new(width, height));
        Ok(())
    });

    setter(lua_api, "get_height", |node, _, _: ()| {
        // update scale
        Ok(node.size().y)
    });
    setter(lua_api, "set_height", |node, _, height| {
        let width = node.size().y;
        node.set_size(Vec2::new(width, height));
        Ok(())
    });

    getter(lua_api, "get_color", |node, _, _: ()| {
        Ok(LuaColor::from(node.color()))
    });
    setter(lua_api, "set_color", |node, _, color: LuaColor| {
        node.set_color(color.into());
        Ok(())
    });

    getter(lua_api, "get_color_mode", |node, _, _: ()| {
        Ok(node.color_mode())
    });
    setter(lua_api, "set_color_mode", |node, _, mode| {
        node.set_color_mode(mode);
        Ok(())
    });

    setter(lua_api, "never_flip", |node, _, never_flip| {
        node.set_never_flip(never_flip);
        Ok(())
    });

    setter(
        lua_api,
        "enable_parent_shader",
        |node, _, enable: Option<bool>| {
            node.set_using_parent_shader(enable.unwrap_or(true));
            Ok(())
        },
    );
}

pub fn create_sprite_table(
    lua: &rollback_mlua::Lua,
    entity_id: EntityID,
    index: GenerationalIndex,
    animator_index: Option<generational_arena::Index>,
) -> rollback_mlua::Result<rollback_mlua::Table> {
    let table = lua.create_table()?;
    table.raw_set("#id", entity_id)?;
    table.raw_set("#index", index)?;

    if let Some(index) = animator_index {
        table.raw_set("#anim", GenerationalIndex::from(index))?;
    }

    inherit_metatable(lua, SPRITE_TABLE, &table)?;

    Ok(table)
}

fn getter<F, P, R>(lua_api: &mut BattleLuaApi, name: &str, callback: F)
where
    R: for<'lua> rollback_mlua::ToLua<'lua>,
    P: for<'lua> rollback_mlua::FromLuaMulti<'lua>,
    F: for<'lua> Fn(&SpriteNode, &'lua rollback_mlua::Lua, P) -> rollback_mlua::Result<R> + 'static,
{
    lua_api.add_dynamic_function(SPRITE_TABLE, name, move |api_ctx, lua, params| {
        let (table, param): (rollback_mlua::Table, P) = lua.unpack_multi(params)?;

        let id: EntityID = table.raw_get("#id")?;
        let index: GenerationalIndex = table.raw_get("#index")?;

        let mut api_ctx = api_ctx.borrow_mut();
        let entities = &mut api_ctx.simulation.entities;

        let entity = entities
            .query_one_mut::<&Entity>(id.into())
            .map_err(|_| entity_not_found())?;

        let sprite_node = entity.sprite_tree.get(index).ok_or_else(sprite_not_found)?;

        lua.pack_multi(callback(sprite_node, lua, param)?)
    });
}

fn setter<F, P, R>(lua_api: &mut BattleLuaApi, name: &str, callback: F)
where
    R: for<'lua> rollback_mlua::ToLuaMulti<'lua>,
    P: for<'lua> rollback_mlua::FromLuaMulti<'lua>,
    F: for<'lua> Fn(&mut SpriteNode, &'lua rollback_mlua::Lua, P) -> rollback_mlua::Result<R>
        + 'static,
{
    lua_api.add_dynamic_function(SPRITE_TABLE, name, move |api_ctx, lua, params| {
        let (table, param): (rollback_mlua::Table, P) = lua.unpack_multi(params)?;

        let id: EntityID = table.raw_get("#id")?;
        let index: GenerationalIndex = table.raw_get("#index")?;

        let mut api_ctx = api_ctx.borrow_mut();
        let entities = &mut api_ctx.simulation.entities;

        let entity = entities
            .query_one_mut::<&mut Entity>(id.into())
            .map_err(|_| entity_not_found())?;

        let sprite_node = entity
            .sprite_tree
            .get_mut(index)
            .ok_or_else(sprite_not_found)?;

        lua.pack_multi(callback(sprite_node, lua, param)?)
    });
}
