use super::{Animator, HiddenSprite};
use crate::render::FrameTime;
use crate::resources::{AssetManager, Globals, ResourcePaths};
use crate::scenes::OverworldSceneBase;
use framework::prelude::{GameIO, Vec3};
use packets::structures::Direction;

#[derive(Debug, Clone, Copy)]
pub enum WarpType {
    In {
        position: Vec3,
        direction: Direction,
    },
    Out,
    /// Same as WarpType::Out. On completion, it transforms into WarpType::In
    Full {
        position: Vec3,
        direction: Direction,
    },
}

/// Attaches to the actor
#[derive(Default)]
pub struct WarpController {
    pub walk_timer: Option<FrameTime>,
    pub warp_entity: Option<hecs::Entity>,
}

/// Attaches to the warp animation entity
pub struct WarpEffect {
    pub actor_entity: hecs::Entity,
    pub warp_type: WarpType,
    pub last_frame: Option<usize>,
    pub callback: Option<Box<dyn FnOnce(&GameIO<Globals>, &mut OverworldSceneBase) + Send + Sync>>,
}

impl WarpEffect {
    pub fn spawn(
        game_io: &GameIO<Globals>,
        base_scene: &mut OverworldSceneBase,
        target_entity: hecs::Entity,
        callback: Box<dyn FnOnce(&GameIO<Globals>, &mut OverworldSceneBase) + Send + Sync>,
        warp_type: WarpType,
    ) -> hecs::Entity {
        let position = *base_scene
            .entities
            .query_one_mut::<&Vec3>(target_entity)
            .unwrap();

        let globals = game_io.globals();

        let screen_position = base_scene.map.world_3d_to_screen(position);

        if base_scene.ui_camera.bounds().contains(screen_position) {
            globals.audio.play_sound(&globals.appear_sfx);
        }

        let assets = &globals.assets;
        let texture = assets.texture(game_io, ResourcePaths::OVERWORLD_WARP);
        let mut animator = Animator::load_new(assets, ResourcePaths::OVERWORLD_WARP_ANIMATION);

        match warp_type {
            WarpType::In { .. } => {
                animator.set_state("IN");
                let _ = base_scene.entities.insert_one(target_entity, HiddenSprite);
            }
            WarpType::Out | WarpType::Full { .. } => {
                animator.set_state("OUT");
            }
        }

        let warp_entity = base_scene.spawn_artifact(game_io, texture, animator, position);

        base_scene
            .entities
            .insert_one(
                warp_entity,
                WarpEffect {
                    warp_type,
                    actor_entity: target_entity,
                    last_frame: None,
                    callback: Some(callback),
                },
            )
            .unwrap();

        let warp_controller = base_scene
            .entities
            .query_one_mut::<&mut WarpController>(target_entity)
            .unwrap();

        warp_controller.warp_entity = Some(warp_entity);

        warp_entity
    }

    pub fn warp_out(
        game_io: &GameIO<Globals>,
        base_scene: &mut OverworldSceneBase,
        target_entity: hecs::Entity,
        callback: impl FnOnce(&GameIO<Globals>, &mut OverworldSceneBase) + Send + Sync + 'static,
    ) -> hecs::Entity {
        Self::spawn(
            game_io,
            base_scene,
            target_entity,
            Box::new(callback),
            WarpType::Out,
        )
    }

    pub fn warp_in(
        game_io: &GameIO<Globals>,
        base_scene: &mut OverworldSceneBase,
        target_entity: hecs::Entity,
        position: Vec3,
        direction: Direction,
        callback: impl FnOnce(&GameIO<Globals>, &mut OverworldSceneBase) + Send + Sync + 'static,
    ) -> hecs::Entity {
        Self::spawn(
            game_io,
            base_scene,
            target_entity,
            Box::new(callback),
            WarpType::In {
                position,
                direction,
            },
        )
    }

    pub fn warp_full(
        game_io: &GameIO<Globals>,
        base_scene: &mut OverworldSceneBase,
        target_entity: hecs::Entity,
        position: Vec3,
        direction: Direction,
        callback: impl FnOnce(&GameIO<Globals>, &mut OverworldSceneBase) + Send + Sync + 'static,
    ) -> hecs::Entity {
        Self::spawn(
            game_io,
            base_scene,
            target_entity,
            Box::new(callback),
            WarpType::Full {
                position,
                direction,
            },
        )
    }
}
