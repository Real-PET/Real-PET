use super::{rollback_vm::RollbackVM, BattleScriptContext};
use super::{BattleSimulation, Living};
use crate::bindable::{DefensePriority, EntityID, HitFlag, HitProperties};
use crate::lua_api::create_entity_table;
use crate::resources::Globals;
use framework::prelude::GameIO;
use rollback_mlua::prelude::{LuaFunction, LuaRegistryKey, LuaResult, LuaTable};
use std::cell::RefCell;
use std::sync::Arc;

#[derive(Clone)]
pub struct DefenseRule {
    pub collision_only: bool,
    pub priority: DefensePriority,
    pub vm_index: usize,
    pub table: Arc<LuaRegistryKey>,
}

impl DefenseRule {
    pub fn add(
        api_ctx: &mut BattleScriptContext,
        lua: &rollback_mlua::Lua,
        defense_table: LuaTable,
        entity_id: EntityID,
    ) -> LuaResult<()> {
        let simulation = &mut api_ctx.simulation;
        let entities = &mut simulation.entities;

        let Ok(living) = entities.query_one_mut::<&mut Living>(entity_id.into()) else {
            return Ok(());
        };

        let key = lua.create_registry_value(defense_table.clone())?;

        let mut rule = DefenseRule {
            collision_only: defense_table.get("#collision_only")?,
            priority: defense_table.get("#priority")?,
            vm_index: api_ctx.vm_index,
            table: Arc::new(key),
        };

        if rule.priority == DefensePriority::Last {
            living.defense_rules.push(rule);
            return Ok(());
        }

        let priority = rule.priority;

        if let Some(index) = living
            .defense_rules
            .iter()
            .position(|r| r.priority >= priority)
        {
            // there's a rule with the same or greater priority
            let existing_rule = &mut living.defense_rules[index];

            if existing_rule.priority > rule.priority {
                // greater priority, we'll insert just before
                living.defense_rules.insert(index, rule);
            } else {
                // same priority, we'll replace
                std::mem::swap(existing_rule, &mut rule);

                // call the on_replace_func on the old rule
                rule.call_on_replace(api_ctx.game_io, simulation, api_ctx.vms);
            }
        } else {
            // nothing should exist after this rule, just append
            living.defense_rules.push(rule);
        }

        Ok(())
    }

    pub fn remove(
        api_ctx: &mut BattleScriptContext,
        lua: &rollback_mlua::Lua,
        defense_table: LuaTable,
        entity_id: EntityID,
    ) -> LuaResult<()> {
        let simulation = &mut api_ctx.simulation;
        let entities = &mut simulation.entities;

        let Ok(living) = entities.query_one_mut::<&mut Living>(entity_id.into()) else {
            return Ok(());
        };

        let priority = defense_table.get("#priority")?;

        let similar_rule_index = living
            .defense_rules
            .iter()
            .position(|rule| rule.vm_index == api_ctx.vm_index && rule.priority == priority);

        if let Some(index) = similar_rule_index {
            let existing_rule = &living.defense_rules[index];

            if lua.registry_value::<LuaTable>(&existing_rule.table)? == defense_table {
                living.defense_rules.remove(index);
            }
        }

        Ok(())
    }

    fn call_on_replace(
        &self,
        game_io: &GameIO<Globals>,
        simulation: &mut BattleSimulation,
        vms: &[RollbackVM],
    ) {
        let lua_api = &game_io.globals().battle_api;

        let context = RefCell::new(BattleScriptContext {
            vm_index: self.vm_index,
            vms,
            game_io,
            simulation,
        });

        let lua = &vms[self.vm_index].lua;

        let table: LuaTable = lua.registry_value(&self.table).unwrap();

        lua_api.inject_dynamic(lua, &context, |_| {
            table.raw_set("#replaced", true)?;

            if let Ok(callback) = table.get::<_, LuaFunction>("on_replace_func") {
                callback.call(())?;
            };

            Ok(())
        });
    }
}

#[derive(Clone, Copy)]
pub struct DefenseJudge {
    pub impact_blocked: bool,
    pub damage_blocked: bool,
}

impl DefenseJudge {
    pub fn new() -> Self {
        Self {
            impact_blocked: false,
            damage_blocked: false,
        }
    }

    pub fn judge(
        game_io: &GameIO<Globals>,
        simulation: &mut BattleSimulation,
        vms: &[RollbackVM],
        defender_id: EntityID,
        attacker_id: EntityID,
        defense_rules: &[DefenseRule],
        collision_only: bool,
    ) {
        let lua_api = &game_io.globals().battle_api;

        for defense_rule in defense_rules {
            if defense_rule.collision_only != collision_only {
                continue;
            }

            let context = RefCell::new(BattleScriptContext {
                vm_index: defense_rule.vm_index,
                vms,
                game_io,
                simulation,
            });

            let lua = &vms[defense_rule.vm_index].lua;

            let table: LuaTable = lua.registry_value(&defense_rule.table).unwrap();

            lua_api.inject_dynamic(lua, &context, |lua| {
                let Ok(callback): LuaResult<LuaFunction> = table.get("can_block_func") else {
                    return Ok(());
                };

                // todo: use constant?
                let battle_table: LuaTable = lua.globals().get("Battle")?;
                let judge_table: LuaTable = battle_table.get("DefenseJudge")?;

                let attacker_table = create_entity_table(lua, attacker_id)?;
                let defender_table = create_entity_table(lua, defender_id)?;

                callback.call::<_, ()>((judge_table, attacker_table, defender_table))?;

                Ok(())
            });
        }
    }

    pub fn filter_statuses(
        game_io: &GameIO<Globals>,
        simulation: &mut BattleSimulation,
        vms: &[RollbackVM],
        props: &mut HitProperties,
        defense_rules: &[DefenseRule],
    ) {
        let lua_api = &game_io.globals().battle_api;
        let no_counter = props.flags & HitFlag::NO_COUNTER;

        for defense_rule in defense_rules {
            let context = RefCell::new(BattleScriptContext {
                vm_index: defense_rule.vm_index,
                vms,
                game_io,
                simulation,
            });

            let lua = &vms[defense_rule.vm_index].lua;

            let table: LuaTable = lua.registry_value(&defense_rule.table).unwrap();

            lua_api.inject_dynamic(lua, &context, |_| {
                let Ok(callback): LuaResult<LuaFunction> = table.get("filter_statuses_func") else {
                    return Ok(());
                };

                *props = callback.call(&*props)?;

                Ok(())
            });
        }

        // prevent accidental overwrite of this internal flag
        props.flags |= no_counter;
    }
}
