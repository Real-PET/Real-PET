use super::{BattleAnimator, BattleCallback, Entity, Field};
use crate::bindable::{ActionLockout, CardProperties, EntityID, GenerationalIndex};
use crate::render::{DerivedFrame, FrameTime, SpriteNode, Tree};
use generational_arena::Arena;

#[derive(Clone)]
pub struct CardAction {
    pub active_frames: FrameTime,
    pub deleted: bool,
    pub executed: bool,
    pub used: bool,
    pub interrupted: bool,
    pub entity: EntityID,
    pub state: String,
    pub prev_state: Option<String>,
    pub frame_callbacks: Vec<(usize, BattleCallback)>,
    pub sprite_index: GenerationalIndex,
    pub properties: CardProperties,
    pub derived_frames: Option<Vec<DerivedFrame>>,
    pub steps: Vec<CardActionStep>,
    pub step_index: usize,
    pub attachments: Vec<CardActionAttachment>,
    pub lockout_type: ActionLockout,
    pub time_freeze_blackout_tiles: bool,
    pub old_position: (i32, i32),
    pub can_move_to_callback: Option<BattleCallback<(i32, i32), bool>>,
    pub update_callback: Option<BattleCallback>,
    pub execute_callback: Option<BattleCallback>,
    pub end_callback: Option<BattleCallback>,
    pub animation_end_callback: Option<BattleCallback>,
}

impl CardAction {
    pub fn new(entity: EntityID, state: String, sprite_index: GenerationalIndex) -> Self {
        Self {
            active_frames: 0,
            deleted: false,
            executed: false,
            used: false,
            interrupted: false,
            entity,
            state,
            prev_state: None,
            frame_callbacks: Vec::new(),
            sprite_index,
            properties: CardProperties::default(),
            derived_frames: None,
            steps: Vec::new(),
            step_index: 0,
            attachments: Vec::new(),
            lockout_type: ActionLockout::Animation,
            time_freeze_blackout_tiles: false,
            old_position: (0, 0),
            can_move_to_callback: None,
            update_callback: None,
            execute_callback: None,
            end_callback: None,
            animation_end_callback: None,
        }
    }

    pub fn is_async(&self) -> bool {
        matches!(self.lockout_type, ActionLockout::Async(_))
    }

    pub fn complete_sync(
        &mut self,
        entities: &mut hecs::World,
        animators: &mut Arena<BattleAnimator>,
        pending_callbacks: &mut Vec<BattleCallback>,
        field: &mut Field,
    ) {
        let entity_id = self.entity.into();
        let entity = entities.query_one_mut::<&mut Entity>(entity_id).unwrap();

        // unset card_action_index to allow other card actions to be used
        entity.card_action_index = None;

        // revert animation
        if let Some(state) = self.prev_state.as_ref() {
            let animator = &mut animators[entity.animator_index];
            let callbacks = animator.set_state(state);
            pending_callbacks.extend(callbacks);

            let sprite_node = entity.sprite_tree.root_mut();
            animator.apply(sprite_node);
        }

        // update reservations as they're ignored while in a sync card action
        if entity.auto_reserves_tiles {
            let old_tile = field.tile_at_mut(self.old_position).unwrap();
            old_tile.remove_reservation_for(entity.id);

            let current_tile = field.tile_at_mut((entity.x, entity.y)).unwrap();
            current_tile.reserve_for(entity.id);
        }
    }
}

#[derive(Clone)]
pub struct CardActionAttachment {
    pub point_name: String,
    pub sprite_index: GenerationalIndex,
    pub animator_index: generational_arena::Index,
    pub parent_animator_index: generational_arena::Index,
}

impl CardActionAttachment {
    pub fn new(
        point_name: String,
        sprite_index: GenerationalIndex,
        animator_index: generational_arena::Index,
        parent_animator_index: generational_arena::Index,
    ) -> Self {
        Self {
            point_name,
            sprite_index,
            animator_index,
            parent_animator_index,
        }
    }

    pub fn apply_animation(
        &self,
        sprite_tree: &mut Tree<SpriteNode>,
        animators: &mut Arena<BattleAnimator>,
    ) {
        let sprite_node = match sprite_tree.get_mut(self.sprite_index) {
            Some(sprite_node) => sprite_node,
            None => return,
        };

        let animator = &mut animators[self.animator_index];
        animator.enable();
        animator.apply(sprite_node);

        // attach to point
        let parent_animator = &mut animators[self.parent_animator_index];

        if let Some(point) = parent_animator.point(&self.point_name) {
            sprite_node.set_offset(point - parent_animator.origin());
            sprite_node.set_visible(true);
        } else {
            sprite_node.set_visible(false);
        }
    }
}

#[derive(Clone, Default)]
pub struct CardActionStep {
    pub completed: bool,
    pub callback: BattleCallback,
}
