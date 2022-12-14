use super::{BattleState, State};
use crate::battle::{BattleSimulation, Entity, Living, Player, RollbackVM, SharedBattleAssets};
use crate::bindable::{EntityID, SpriteColorMode};
use crate::ease::inverse_lerp;
use crate::render::{AnimatorLoopMode, FrameTime};
use crate::resources::Globals;
use framework::prelude::{Color, GameIO, Vec2};

const FADE_TIME: FrameTime = 15;

#[derive(Clone)]
pub struct FormActivateState {
    time: FrameTime,
    target_complete_time: Option<FrameTime>,
    artifact_entities: Vec<EntityID>,
    completed: bool,
}

impl State for FormActivateState {
    fn clone_box(&self) -> Box<dyn State> {
        Box::new(self.clone())
    }

    fn next_state(&self, _game_io: &GameIO<Globals>) -> Option<Box<dyn State>> {
        if self.completed {
            Some(Box::new(BattleState::new()))
        } else {
            None
        }
    }

    fn update(
        &mut self,
        game_io: &GameIO<Globals>,
        _shared_assets: &mut SharedBattleAssets,
        simulation: &mut BattleSimulation,
        vms: &[RollbackVM],
    ) {
        if self.time == 0 && !has_pending_activations(simulation) {
            self.completed = true;
            return;
        }

        if self.target_complete_time == Some(self.time) {
            self.mark_activated(simulation);
            self.completed = true;
            return;
        }

        // update fade
        let alpha = match self.target_complete_time {
            Some(time) => inverse_lerp!(time, time - FADE_TIME, self.time),
            None => inverse_lerp!(0, FADE_TIME, self.time),
        };

        let fade_color = Color::BLACK.multiply_alpha(alpha);
        simulation.fade_sprite.set_color(fade_color);

        // logic
        if self.target_complete_time.is_none() {
            match self.time {
                // flash white and activate forms
                15 => {
                    set_relevant_color(&mut simulation.entities, Color::WHITE);
                    self.activate_forms(game_io, simulation, vms);
                }
                // flash white for 9 more frames
                16..=25 => set_relevant_color(&mut simulation.entities, Color::WHITE),
                // reset color
                26 => set_relevant_color(&mut simulation.entities, Color::BLACK),
                // wait for the shine artifacts to complete animation
                27.. => self.detect_animation_end(game_io, simulation, vms),
                _ => {}
            }
        }

        self.time += 1;
    }
}

impl FormActivateState {
    pub fn new() -> Self {
        Self {
            time: 0,
            target_complete_time: None,
            artifact_entities: Vec::new(),
            completed: false,
        }
    }

    fn activate_forms(
        &mut self,
        game_io: &GameIO<Globals>,
        simulation: &mut BattleSimulation,
        vms: &[RollbackVM],
    ) {
        let mut updated_animators = Vec::new();

        // deactivate previous forms, and activate new forms
        for (_, (entity, living, player)) in simulation
            .entities
            .query_mut::<(&mut Entity, &mut Living, &Player)>()
        {
            let Some(index) = player.active_form else {
                continue;
            };

            if player.forms[index].activated {
                continue;
            }

            // clear statuses
            living.status_director.clear_statuses();

            // cancel movement
            entity.move_action = None;

            // set animation to idle
            let animator = &mut simulation.animators[entity.animator_index];
            let callbacks = animator.set_state(Player::IDLE_STATE);
            animator.set_loop_mode(AnimatorLoopMode::Loop);
            simulation.pending_callbacks.extend(callbacks);

            updated_animators.push(entity.animator_index);

            // deactivate previous forms
            for form in &player.forms {
                if !form.activated || form.deactivated {
                    continue;
                }

                if let Some(callback) = form.deactivate_callback.clone() {
                    simulation.pending_callbacks.push(callback);
                }
            }

            // activate new form
            let form = &player.forms[index];

            if let Some(callback) = form.activate_callback.clone() {
                simulation.pending_callbacks.push(callback);
            }
        }

        simulation.call_pending_callbacks(game_io, vms);

        self.spawn_shine(game_io, simulation);
    }

    fn spawn_shine(&mut self, game_io: &GameIO<Globals>, simulation: &mut BattleSimulation) {
        let mut relevant_ids = Vec::new();

        for (id, player) in simulation.entities.query_mut::<&Player>() {
            let Some(index) = player.active_form else {
                continue;
            };

            if player.forms[index].activated {
                continue;
            }

            relevant_ids.push(id);
        }

        for id in relevant_ids {
            let Ok(entity) = simulation.entities.query_one_mut::<&Entity>(id.into()) else {
                continue;
            };

            let mut full_position = entity.full_position();
            full_position.offset += Vec2::new(0.0, -entity.height * 0.5);

            let shine_id = simulation.create_transformation_shine(game_io);
            let shine_entity = simulation
                .entities
                .query_one_mut::<&mut Entity>(shine_id.into())
                .unwrap();

            shine_entity.copy_full_position(full_position);
            shine_entity.pending_spawn = true;

            self.artifact_entities.push(shine_id);
        }

        // play sfx
        let shine_sfx = &game_io.globals().shine_sfx;
        simulation.play_sound(game_io, shine_sfx);

        let transform_sfx = &game_io.globals().transform_sfx;
        simulation.play_sound(game_io, transform_sfx);
    }

    fn detect_animation_end(
        &mut self,
        game_io: &GameIO<Globals>,
        simulation: &mut BattleSimulation,
        vms: &[RollbackVM],
    ) {
        for id in self.artifact_entities.iter().cloned() {
            let Ok(entity) = simulation.entities.query_one_mut::<&mut Entity>(id.into()) else {
                continue;
            };

            // force update, since animations are only automatic in BattleState
            let animator = &mut simulation.animators[entity.animator_index];
            let callbacks = animator.update();
            simulation.pending_callbacks.extend(callbacks);
        }

        simulation.call_pending_callbacks(game_io, vms);

        let all_erased = self
            .artifact_entities
            .iter()
            .all(|id| !simulation.entities.contains((*id).into()));

        if !all_erased {
            return;
        }

        self.target_complete_time = Some(self.time + FADE_TIME);
    }

    fn mark_activated(&mut self, simulation: &mut BattleSimulation) {
        for (_, player) in simulation.entities.query_mut::<&mut Player>() {
            let Some(index) = player.active_form else {
                continue;
            };

            if player.forms[index].activated {
                continue;
            }

            for form in &mut player.forms {
                if form.activated {
                    form.deactivated = true;
                }
            }

            let form = &mut player.forms[index];
            form.activated = true;
            form.deactivated = false;
        }
    }
}

fn has_pending_activations(simulation: &mut BattleSimulation) -> bool {
    for (_, player) in simulation.entities.query_mut::<&Player>() {
        let Some(index) = player.active_form else {
            continue;
        };

        if !player.forms[index].activated {
            return true;
        }
    }

    return false;
}

fn set_relevant_color(entities: &mut hecs::World, color: Color) {
    for (_, (entity, player)) in entities.query_mut::<(&mut Entity, &Player)>() {
        let Some(index) = player.active_form else {
            continue;
        };

        if player.forms[index].activated {
            continue;
        }

        let root_node = entity.sprite_tree.root_mut();
        root_node.set_color_mode(SpriteColorMode::Add);
        root_node.set_color(color);
    }
}
