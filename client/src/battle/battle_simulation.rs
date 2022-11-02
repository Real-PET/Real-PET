use super::rollback_vm::RollbackVM;
use super::*;
use crate::bindable::*;
use crate::lua_api::create_entity_table;
use crate::packages::PackageNamespace;
use crate::render::ui::{FontStyle, PlayerHealthUI, Text};
use crate::render::*;
use crate::resources::*;
use crate::saves::Card;
use framework::prelude::*;
use generational_arena::Arena;
use packets::structures::BattleStatistics;
use std::cell::RefCell;

const DEFAULT_PLAYER_LAYOUTS: [[(i32, i32); 4]; 4] = [
    [(2, 2), (0, 0), (0, 0), (0, 0)],
    [(2, 2), (5, 2), (0, 0), (0, 0)],
    [(2, 2), (4, 3), (6, 1), (0, 0)],
    [(1, 3), (3, 1), (4, 3), (6, 1)],
];

pub struct BattleSimulation {
    pub battle_started: bool,
    pub statistics: BattleStatistics,
    pub inputs: Vec<PlayerInput>,
    pub time: FrameTime,
    pub battle_time: FrameTime,
    pub camera: Camera,
    pub background: Background,
    pub turn_guage: TurnGauge,
    pub field: Field,
    pub entities: hecs::World,
    pub queued_attacks: Vec<AttackBox>,
    pub defense_judge: DefenseJudge,
    pub animators: Arena<BattleAnimator>,
    pub card_actions: Arena<CardAction>,
    pub components: Arena<Component>,
    pub pending_callbacks: Vec<BattleCallback>,
    pub local_player_id: EntityID,
    pub local_health_ui: PlayerHealthUI,
    pub player_spawn_positions: Vec<(i32, i32)>,
    pub perspective_flipped: bool,
    pub intro_complete: bool,
    pub stale: bool, // resimulation
    pub exit: bool,
}

impl BattleSimulation {
    pub fn new(game_io: &GameIO<Globals>, background: Background, player_count: usize) -> Self {
        let mut camera = Camera::new(game_io);
        camera.snap(Vec2::new(0.0, 10.0));

        let spawn_count = player_count.min(4);
        let mut player_spawn_positions = DEFAULT_PLAYER_LAYOUTS[spawn_count - 1].to_vec();
        player_spawn_positions.resize(spawn_count, (0, 0));

        Self {
            battle_started: false,
            statistics: BattleStatistics::new(),
            time: 0,
            battle_time: 0,
            inputs: vec![PlayerInput::new(); player_count],
            camera,
            background,
            turn_guage: TurnGauge::new(),
            field: Field::new(game_io, 8, 5),
            entities: hecs::World::new(),
            queued_attacks: Vec::new(),
            defense_judge: DefenseJudge::new(),
            animators: Arena::new(),
            card_actions: Arena::new(),
            components: Arena::new(),
            pending_callbacks: Vec::new(),
            local_player_id: EntityID::DANGLING,
            local_health_ui: PlayerHealthUI::new(game_io),
            player_spawn_positions,
            perspective_flipped: false,
            intro_complete: false,
            stale: false,
            exit: false,
        }
    }

    pub fn clone(&mut self, game_io: &GameIO<Globals>) -> Self {
        let mut entities = hecs::World::new();

        // starting with Entity as every entity will have Entity
        for (id, entity) in self.entities.query_mut::<&Entity>() {
            entities.spawn_at(id, (entity.clone(),));
        }

        // cloning every component
        for (id, component) in self.entities.query_mut::<&Artifact>() {
            let _ = entities.insert_one(id, component.clone());
        }

        for (id, component) in self.entities.query_mut::<&Character>() {
            let _ = entities.insert_one(id, component.clone());
        }

        for (id, component) in self.entities.query_mut::<&Living>() {
            let _ = entities.insert_one(id, component.clone());
        }

        for (id, component) in self.entities.query_mut::<&Obstacle>() {
            let _ = entities.insert_one(id, component.clone());
        }

        for (id, component) in self.entities.query_mut::<&Player>() {
            let _ = entities.insert_one(id, component.clone());
        }

        for (id, component) in self.entities.query_mut::<&Spell>() {
            let _ = entities.insert_one(id, component.clone());
        }

        Self {
            battle_started: self.battle_started.clone(),
            statistics: self.statistics.clone(),
            inputs: self.inputs.clone(),
            time: self.time.clone(),
            battle_time: self.battle_time.clone(),
            camera: self.camera.clone(game_io),
            background: self.background.clone(),
            turn_guage: self.turn_guage.clone(),
            field: self.field.clone(),
            entities,
            queued_attacks: self.queued_attacks.clone(),
            defense_judge: self.defense_judge.clone(),
            animators: self.animators.clone(),
            card_actions: self.card_actions.clone(),
            components: self.components.clone(),
            pending_callbacks: self.pending_callbacks.clone(),
            local_player_id: self.local_player_id.clone(),
            local_health_ui: self.local_health_ui.clone(),
            player_spawn_positions: self.player_spawn_positions.clone(),
            perspective_flipped: self.perspective_flipped.clone(),
            intro_complete: self.intro_complete.clone(),
            stale: self.stale.clone(),
            exit: self.exit.clone(),
        }
    }

    pub fn initialize_uninitialized(&mut self) {
        self.field.initialize_uninitialized();

        type PlayerQuery<'a> = (&'a mut Entity, &'a Player, &'a Living);

        for (_, (entity, player, living)) in self.entities.query_mut::<PlayerQuery>() {
            if player.local {
                self.local_player_id = entity.id;
                self.local_health_ui.snap_health(living.health);
            }

            let pos = self
                .player_spawn_positions
                .get(player.index)
                .cloned()
                .unwrap_or_default();

            entity.x = pos.0;
            entity.y = pos.1;

            let animator = &mut self.animators[entity.animator_index];

            if animator.current_state().is_none() {
                let callbacks = animator.set_state("PLAYER_IDLE");
                self.pending_callbacks.extend(callbacks);
            }
        }
    }

    pub fn pre_update(&mut self, game_io: &GameIO<Globals>, vms: &[RollbackVM]) {
        #[cfg(debug_assertions)]
        self.assertions();

        // update bg
        self.background.update();

        // reset frame temporary variables
        self.prepare_updates();

        // update sprites
        self.update_animations();

        // spawn pending entities
        self.spawn_pending();

        // apply animations after spawning to display frame 0
        self.apply_animations();

        // animation + spawn callbacks
        self.call_pending_callbacks(game_io, vms);
    }

    pub fn post_update(&mut self, game_io: &GameIO<Globals>, vms: &[RollbackVM]) {
        // update scene components
        for (_, component) in &self.components {
            if component.lifetime == ComponentLifetime::BattleStep {
                self.pending_callbacks
                    .push(component.update_callback.clone());
            }
        }

        // should this be called every time we invoke lua?
        self.call_pending_callbacks(game_io, vms);

        // remove dead entities
        self.cleanup_erased_entities();

        self.update_ui();

        self.time += 1;
    }

    fn prepare_updates(&mut self) {
        // entities should only update once, clearing the flag that tracks this
        for (_, entity) in self.entities.query_mut::<&mut Entity>() {
            entity.updated = false;

            let sprite_node = entity.sprite_tree.root_mut();

            // reset frame temp properties
            entity.tile_offset = Vec2::ZERO;
            sprite_node.set_color(Color::BLACK);
            sprite_node.set_color_mode(SpriteColorMode::Add);
        }
    }

    fn update_animations(&mut self) {
        for (_, animator) in &mut self.animators {
            self.pending_callbacks.extend(animator.update());
        }
    }

    fn spawn_pending(&mut self) {
        for (_, entity) in self.entities.query_mut::<&mut Entity>() {
            if !entity.pending_spawn {
                continue;
            }

            entity.pending_spawn = false;
            entity.spawned = true;
            entity.on_field = true;

            let tile = self.field.tile_at_mut((entity.x, entity.y)).unwrap();
            tile.entity_count += 1;

            if entity.team == Team::Unset {
                entity.team = tile.team();
            }

            if entity.facing == Direction::None {
                entity.facing = tile.direction();
            }

            self.animators[entity.animator_index].enable();
            self.pending_callbacks.push(entity.spawn_callback.clone());

            if self.battle_started {
                self.pending_callbacks
                    .push(entity.battle_start_callback.clone())
            }
        }
    }

    fn apply_animations(&mut self) {
        for (_, entity) in self.entities.query_mut::<&mut Entity>() {
            let sprite_node = entity.sprite_tree.root_mut();

            // update root sprite
            self.animators[entity.animator_index].apply(sprite_node);
        }

        // update attachment sprites
        // separate loop from entities to account for async actions
        for (_, action) in &mut self.card_actions {
            if !action.executed {
                continue;
            }

            let entity = match (self.entities).query_one_mut::<&mut Entity>(action.entity.into()) {
                Ok(entity) => entity,
                Err(_) => continue,
            };

            for attachment in &mut action.attachments {
                attachment.apply_animation(&mut entity.sprite_tree, &mut self.animators);
            }
        }
    }

    pub fn call_pending_callbacks(&mut self, game_io: &GameIO<Globals>, vms: &[RollbackVM]) {
        let callbacks = std::mem::take(&mut self.pending_callbacks);

        for callback in callbacks {
            callback.call(game_io, self, vms, ());
        }
    }

    fn cleanup_erased_entities(&mut self) {
        let mut pending_removal = Vec::new();
        let mut components_pending_removal = Vec::new();
        let mut card_actions_pending_removal = Vec::new();

        for (id, entity) in self.entities.query_mut::<&Entity>() {
            if !entity.erased {
                continue;
            }

            if entity.spawned {
                let tile = self.field.tile_at_mut((entity.x, entity.y)).unwrap();
                tile.unignore_attacker(entity.id);
                tile.entity_count -= 1;
            }

            for (index, component) in &mut self.components {
                if component.entity == entity.id {
                    components_pending_removal.push(index);
                }
            }

            for (index, card_action) in &mut self.card_actions {
                if card_action.entity == entity.id {
                    card_actions_pending_removal.push(index);
                }
            }

            self.animators.remove(entity.animator_index);

            pending_removal.push(id);
        }

        for id in pending_removal {
            self.entities.despawn(id).unwrap();
        }

        for index in components_pending_removal {
            self.components.remove(index);
        }

        for index in card_actions_pending_removal {
            // action_end callbacks would already be handled by delete listeners
            self.card_actions.remove(index);
        }
    }

    fn update_ui(&mut self) {
        if let Ok(living) = (self.entities).query_one_mut::<&Living>(self.local_player_id.into()) {
            self.local_health_ui.set_health(living.health);
        } else {
            self.local_health_ui.set_health(0);
        }

        self.local_health_ui.update();
    }

    pub fn delete_entity(&mut self, game_io: &GameIO<Globals>, vms: &[RollbackVM], id: EntityID) {
        let entity = match self.entities.query_one_mut::<&mut Entity>(id.into()) {
            Ok(entity) => entity,
            _ => return,
        };

        if entity.deleted {
            return;
        }

        let delete_indices: Vec<_> = (self.card_actions)
            .iter()
            .filter(|(_, action)| action.entity == id && action.used)
            .map(|(index, _)| index)
            .collect();

        entity.deleted = true;

        let callbacks = std::mem::take(&mut entity.delete_callbacks);
        let delete_callback = entity.delete_callback.clone();

        // delete card actions
        self.delete_card_actions(game_io, vms, &delete_indices);

        // call delete callbacks after
        self.pending_callbacks.extend(callbacks);
        self.pending_callbacks.push(delete_callback);

        self.call_pending_callbacks(game_io, vms);
    }

    pub fn delete_card_actions(
        &mut self,
        game_io: &GameIO<Globals>,
        vms: &[RollbackVM],

        delete_indices: &[generational_arena::Index],
    ) {
        for index in delete_indices {
            let card_action = self.card_actions.get(*index).unwrap();

            // remove the index from the entity
            let entity = self
                .entities
                .query_one_mut::<&mut Entity>(card_action.entity.into())
                .unwrap();

            if !entity.deleted && entity.card_action_index == Some(*index) {
                entity.card_action_index = None;

                // revert state
                if let Some(state) = card_action.prev_state.as_ref() {
                    let animator = &mut self.animators[entity.animator_index];
                    let callbacks = animator.set_state(state);
                    self.pending_callbacks.extend(callbacks);

                    let sprite_node = entity.sprite_tree.root_mut();
                    animator.apply(sprite_node);
                }
            }

            // end callback
            if let Some(callback) = card_action.end_callback.clone() {
                callback.call(game_io, self, vms, ());
            }

            let card_action = self.card_actions.get(*index).unwrap();

            // remove attachments from the entity
            let entity = self
                .entities
                .query_one_mut::<&mut Entity>(card_action.entity.into())
                .unwrap();

            entity.sprite_tree.remove(card_action.sprite_index);

            for attachment in &card_action.attachments {
                self.animators.remove(attachment.animator_index);
            }

            // finally remove the card action
            self.card_actions.remove(*index);
        }

        self.call_pending_callbacks(game_io, vms);
    }

    fn create_entity(&mut self, game_io: &GameIO<Globals>) -> EntityID {
        let mut animator = BattleAnimator::new();
        animator.disable();
        let animator_index = self.animators.insert(animator);

        let id: EntityID = self.entities.reserve_entity().into();

        self.entities
            .spawn_at(id.into(), (Entity::new(game_io, id, animator_index),));

        id
    }

    pub fn find_vm(
        vms: &[RollbackVM],
        package_id: &str,
        namespace: PackageNamespace,
    ) -> rollback_mlua::Result<usize> {
        let vm_index = namespace
            .find_with_fallback(|namespace| {
                vms.iter()
                    .position(|vm| vm.package_id == package_id && vm.namespace == namespace)
            })
            .ok_or_else(|| {
                rollback_mlua::Error::RuntimeError(format!(
                    "no package with id {:?} found",
                    package_id
                ))
            })?;

        Ok(vm_index)
    }

    fn create_character(
        &mut self,
        game_io: &GameIO<Globals>,
        rank: CharacterRank,
    ) -> rollback_mlua::Result<EntityID> {
        let id = self.create_entity(game_io);

        self.entities
            .insert(id.into(), (Character::new(rank), Living::default()))
            .unwrap();

        let entity = self
            .entities
            .query_one_mut::<&mut Entity>(id.into())
            .unwrap();

        entity.can_move_to_callback = BattleCallback::new(move |_, simulation, _, dest| {
            if simulation.field.is_edge(dest) {
                // can't walk on edge tiles
                return false;
            }

            let tile = match simulation.field.tile_at_mut(dest) {
                Some(tile) => tile,
                None => return false,
            };

            let entity = simulation
                .entities
                .query_one_mut::<&Entity>(id.into())
                .unwrap();

            if !entity.ignore_hole_tiles && tile.state().is_hole() {
                // can't walk on holes
                return false;
            }

            if !entity.share_tile && tile.has_other_reservations(entity.id) {
                // reserved by another entity
                return false;
            }

            // tile can't belong to the opponent team
            tile.team() == entity.team || tile.team() == Team::Other
        });

        entity.delete_callback = BattleCallback::new(move |_, simulation, _, _| {
            // let entity = simulation
            //     .entities
            //     .query_one_mut::<&mut Entity>(id.into())
            //     .unwrap();

            Component::new_character_deletion(simulation, id);
            // todo: explosions
        });

        Ok(id)
    }

    pub fn load_player(
        &mut self,
        game_io: &GameIO<Globals>,
        vms: &[RollbackVM],

        package_id: &str,
        namespace: PackageNamespace,
        index: usize,
        local: bool,
        cards: Vec<Card>,
    ) -> rollback_mlua::Result<EntityID> {
        let vm_index = Self::find_vm(vms, package_id, namespace)?;
        let id = self.create_character(game_io, CharacterRank::V1)?;

        let (entity, living) = self
            .entities
            .query_one_mut::<(&mut Entity, &mut Living)>(id.into())
            .unwrap();

        // spawn immediately
        entity.pending_spawn = true;

        // use preloaded package properties
        let player_package = (game_io.globals().player_packages)
            .package_or_fallback(namespace, package_id)
            .unwrap();

        entity.element = player_package.element;
        entity.name = player_package.name.clone();
        living.set_health(player_package.health);

        // derive states
        let animator = &mut self.animators[entity.animator_index];

        let move_anim_state = animator.derive_state("PLAYER_MOVE", Player::MOVE_FRAMES.to_vec());
        let flinch_anim_state = animator.derive_state("PLAYER_HIT", Player::HIT_FRAMES.to_vec());
        entity.move_anim_state = Some(move_anim_state);
        living.flinch_anim_state = Some(flinch_anim_state);

        let charge_index =
            (entity.sprite_tree).insert_root_child(SpriteNode::new(game_io, SpriteColorMode::Add));
        let charge_sprite = &mut entity.sprite_tree[charge_index];
        charge_sprite.set_texture(game_io, ResourcePaths::BATTLE_CHARGE.to_string());
        charge_sprite.set_visible(false);
        charge_sprite.set_layer(-2);
        charge_sprite.set_offset(Vec2::new(0.0, -20.0));

        entity.delete_callback = BattleCallback::new(move |game_io, simulation, _, _| {
            let (entity, living) = simulation
                .entities
                .query_one_mut::<(&mut Entity, &Living)>(id.into())
                .unwrap();

            let x = entity.x;
            let y = entity.y;

            // flinch
            let player_animator = &mut simulation.animators[entity.animator_index];
            let callbacks = player_animator.set_state(living.flinch_anim_state.as_ref().unwrap());
            simulation.pending_callbacks.extend(callbacks);

            let player_root_node = entity.sprite_tree.root_mut();
            player_animator.apply(player_root_node);

            player_animator.disable();

            Component::new_character_deletion(simulation, id);

            // create transformation shine artifact

            let artifact_id = simulation.create_artifact(game_io);
            let artifact_entity = simulation
                .entities
                .query_one_mut::<&mut Entity>(artifact_id.into())
                .unwrap();

            artifact_entity.x = x;
            artifact_entity.y = y;
            artifact_entity.pending_spawn = true;

            let root_node = artifact_entity.sprite_tree.root_mut();
            root_node.set_texture(game_io, ResourcePaths::BATTLE_TRANSFORM_SHINE.to_string());

            let animator = &mut simulation.animators[artifact_entity.animator_index];
            animator.load(game_io, ResourcePaths::BATTLE_TRANSFORM_SHINE_ANIMATION);
            let _ = animator.set_state("DEFAULT");

            animator.on_complete(BattleCallback::new(move |_, simulation, _, _| {
                let artifact_entity = simulation
                    .entities
                    .query_one_mut::<&mut Entity>(artifact_id.into())
                    .unwrap();

                artifact_entity.erased = true;
            }));
        });

        self.entities
            .insert(
                id.into(),
                (Player::new(game_io, index, local, charge_index, cards),),
            )
            .unwrap();

        // call init function
        let lua = &vms[vm_index].lua;
        let player_init: rollback_mlua::Function = lua.globals().get("player_init")?;

        let api_ctx = RefCell::new(BattleScriptContext {
            vm_index,
            vms,
            game_io,
            simulation: self,
        });

        let lua_api = &game_io.globals().battle_api;

        lua_api.inject_dynamic(lua, &api_ctx, move |lua| {
            let table = create_entity_table(lua, id)?;
            player_init.call(table)
        });

        Ok(id)
    }

    pub fn load_character(
        &mut self,
        game_io: &GameIO<Globals>,
        vms: &[RollbackVM],

        package_id: &str,
        namespace: PackageNamespace,
        rank: CharacterRank,
    ) -> rollback_mlua::Result<EntityID> {
        let vm_index = Self::find_vm(vms, package_id, namespace)?;
        let id = self.create_character(game_io, rank)?;

        let lua = &vms[vm_index].lua;
        let character_init: rollback_mlua::Function = lua
            .globals()
            .get("character_init")
            .or_else(|_| lua.globals().get("package_init"))?;

        let api_ctx = RefCell::new(BattleScriptContext {
            vm_index,
            vms,
            game_io,
            simulation: self,
        });

        let lua_api = &game_io.globals().battle_api;

        lua_api.inject_dynamic(lua, &api_ctx, move |lua| {
            let table = create_entity_table(lua, id)?;
            character_init.call(table)
        });

        Ok(id)
    }

    pub fn create_artifact(&mut self, game_io: &GameIO<Globals>) -> EntityID {
        let id = self.create_entity(game_io);

        self.entities
            .insert(id.into(), (Artifact::default(),))
            .unwrap();

        let entity = self
            .entities
            .query_one_mut::<&mut Entity>(id.into())
            .unwrap();

        entity.ignore_hole_tiles = true;
        entity.ignore_tile_effects = true;
        entity.can_move_to_callback = BattleCallback::stub(true);

        id
    }

    pub fn create_spell(&mut self, game_io: &GameIO<Globals>) -> EntityID {
        let id = self.create_entity(game_io);

        self.entities
            .insert(id.into(), (Spell::default(),))
            .unwrap();

        let entity = self
            .entities
            .query_one_mut::<&mut Entity>(id.into())
            .unwrap();

        entity.ignore_hole_tiles = true;
        entity.ignore_tile_effects = true;
        entity.can_move_to_callback = BattleCallback::stub(true);

        id
    }

    pub fn create_obstacle(&mut self, game_io: &GameIO<Globals>) -> EntityID {
        let id = self.create_entity(game_io);

        self.entities
            .insert(
                id.into(),
                (Spell::default(), Obstacle::default(), Living::default()),
            )
            .unwrap();

        id
    }

    pub fn draw(&mut self, game_io: &mut GameIO<Globals>, render_pass: &mut RenderPass) {
        // resolve perspective
        if let Ok(entity) = (self.entities).query_one_mut::<&Entity>(self.local_player_id.into()) {
            self.perspective_flipped = entity.team == Team::Blue;
        }

        // draw background
        self.background.draw(game_io, render_pass);

        // draw field
        self.field
            .draw(game_io, render_pass, &self.camera, self.perspective_flipped);

        // draw entities, sorting by position
        let mut sorted_entities = Vec::with_capacity(self.entities.len() as usize);

        for (_, entity) in self.entities.query_mut::<&mut Entity>() {
            if entity.on_field {
                sorted_entities.push(entity);
            }
        }

        sorted_entities.sort_by_key(|entity| (entity.y, entity.x));

        // reusing vec to avoid realloctions
        let mut sprite_nodes_recycled = Vec::new();
        let mut sprite_queue =
            SpriteColorQueue::new(game_io, &self.camera, SpriteColorMode::default());

        for entity in sorted_entities {
            let mut sprite_nodes = sprite_nodes_recycled;

            // offset for calculating initial placement position
            let mut offset: Vec2 = entity.corrected_offset(self.perspective_flipped);

            // elevation
            offset.y -= entity.elevation;
            let shadow_node = &mut entity.sprite_tree[entity.shadow_index];
            shadow_node.set_offset(Vec2::new(shadow_node.offset().x, entity.elevation));

            let tile_center =
                (self.field).calc_tile_center((entity.x, entity.y), self.perspective_flipped);
            let initial_position = tile_center + offset;

            // true if only one is true, since flipping twice causes us to no longer be flipped
            let flipped = self.perspective_flipped ^ entity.flipped();

            // offset each child by parent node accounting for perspective
            (entity.sprite_tree).inherit_from_parent(initial_position, flipped);

            // capture root values before mutable reference
            let root_node = entity.sprite_tree.root();
            let root_color_mode = root_node.color_mode();
            let root_color = root_node.color();

            // sort nodes
            sprite_nodes.extend(entity.sprite_tree.values_mut());
            sprite_nodes.sort_by_key(|node| -node.layer());

            // draw nodes
            for node in sprite_nodes.iter_mut() {
                if !node.inherited_visible() {
                    // could possibly filter earlier,
                    // but not expecting huge trees of invisible nodes
                    continue;
                }

                // resolve shader
                let color_mode;
                let color;
                let original_color = node.color();

                if node.using_parent_shader() {
                    color_mode = root_color_mode;
                    color = root_color;
                } else {
                    color_mode = node.color_mode();
                    color = node.color();
                }

                sprite_queue.set_color_mode(color_mode);
                node.set_color(color);

                // finally drawing the sprite
                sprite_queue.draw_sprite(node.sprite());

                node.set_color(original_color);
            }

            sprite_nodes_recycled = recycle_vec(sprite_nodes);
        }

        // draw hp on living entities
        if self.intro_complete {
            let mut hp_text = Text::new(game_io, FontStyle::EntityHP);
            hp_text.style.letter_spacing = 0.0;
            let tile_size = self.field.tile_size();

            type Query<'a> = hecs::Without<(&'a Entity, &'a Living, &'a Character), &'a Obstacle>;

            for (_, (entity, living, ..)) in self.entities.query_mut::<Query>() {
                if entity.deleted
                    || !entity.on_field
                    || living.health == 0
                    || !entity.sprite_tree.root().visible()
                    || entity.id == self.local_player_id
                {
                    continue;
                }

                let tile_position = (entity.x, entity.y);
                let tile_center =
                    (self.field).calc_tile_center(tile_position, self.perspective_flipped);

                let entity_offset = entity.corrected_offset(self.perspective_flipped);

                hp_text.text = living.health.to_string();
                let text_size = hp_text.measure().size;

                (hp_text.style.bounds).set_position(tile_center + entity_offset);
                hp_text.style.bounds.x -= text_size.x * 0.5;
                hp_text.style.bounds.y += tile_size.y * 0.5 - text_size.y;
                hp_text.draw(game_io, &mut sprite_queue);
            }
        }

        render_pass.consume_queue(sprite_queue);
    }

    pub fn draw_ui(&mut self, game_io: &GameIO<Globals>, sprite_queue: &mut SpriteColorQueue) {
        self.local_health_ui.draw(game_io, sprite_queue);
    }

    #[cfg(debug_assertions)]
    fn assertions(&mut self) {
        // verify entity counts on tiles
        let cols = self.field.cols();
        let rows = self.field.rows();

        let mut entity_counts = vec![0; cols * rows];

        for (_, entity) in self.entities.query_mut::<&Entity>() {
            if !entity.spawned {
                continue;
            }

            assert!(self.field.in_bounds((entity.x, entity.y)));
            entity_counts[entity.y as usize * cols + entity.x as usize] += 1;
        }

        for ((col, row), tile) in self.field.iter_mut() {
            assert_eq!(tile.entity_count, entity_counts[row * cols + col]);
        }

        // make sure card actions are being deleted
        let held_action_count = self
            .entities
            .query_mut::<&Entity>()
            .into_iter()
            .filter(|(_, entity)| entity.card_action_index.is_some())
            .count();

        let executed_action_count = self
            .card_actions
            .iter()
            .filter(|(_, action)| {
                action.executed && matches!(action.lockout_type, ActionLockout::Async(_))
            })
            .count();

        // if there's more executed actions than held actions, we forgot to delete one
        // we can have more actions than held actions still since
        // scripters don't need to attach card actions to entities
        // we also can ignore async actions
        assert!(held_action_count >= executed_action_count);
    }
}

pub fn recycle_vec<'a, 'b, T: ?Sized>(mut data: Vec<&'a mut T>) -> Vec<&'b mut T> {
    data.clear();
    unsafe { std::mem::transmute(data) }
}