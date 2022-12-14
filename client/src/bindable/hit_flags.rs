pub type HitFlags = u32;

// simulating an enum here, could use a better solution
#[allow(non_snake_case)]
pub mod HitFlag {
    use super::HitFlags;
    use framework::prelude::Vec2;

    pub const NONE: HitFlags = 0x00000000;
    pub const RETAIN_INTANGIBLE: HitFlags = 0x00000001;
    pub const FREEZE: HitFlags = 0x00000002;
    pub const PIERCE_INVIS: HitFlags = 0x00000004;
    pub const FLINCH: HitFlags = 0x00000008;
    pub const SHAKE: HitFlags = 0x00000010;
    pub const PARALYZE: HitFlags = 0x00000020;
    pub const FLASH: HitFlags = 0x00000040;
    pub const PIERCE_GUARD: HitFlags = 0x00000080; // NOTE: this is what we refer to as "true breaking"
    pub const IMPACT: HitFlags = 0x00000100;
    pub const DRAG: HitFlags = 0x00000200;
    pub const BUBBLE: HitFlags = 0x00000400;
    pub const NO_COUNTER: HitFlags = 0x00000800;
    pub const ROOT: HitFlags = 0x00001000;
    pub const BLIND: HitFlags = 0x00002000;
    pub const CONFUSE: HitFlags = 0x00004000;
    pub const PIERCE_GROUND: HitFlags = 0x00008000;

    pub const LIST: [HitFlags; 15] = [
        RETAIN_INTANGIBLE,
        FREEZE,
        PIERCE_INVIS,
        FLINCH,
        SHAKE,
        PARALYZE,
        FLASH,
        PIERCE_GUARD,
        IMPACT,
        DRAG,
        BUBBLE,
        NO_COUNTER,
        ROOT,
        BLIND,
        CONFUSE,
    ];

    pub const STATUS_LIST: [HitFlags; 6] = [FREEZE, PARALYZE, BUBBLE, ROOT, BLIND, CONFUSE];

    pub fn status_animation_state(flag: HitFlags, height: f32) -> &'static str {
        match flag {
            CONFUSE => "CONFUSE",
            BLIND => "BLIND",
            FREEZE => {
                if height <= 48.0 {
                    "FREEZE_SMALL"
                } else if height <= 75.0 {
                    "FREEZE_MEDIUM"
                } else {
                    "FREEZE_LARGE"
                }
            }
            _ => "",
        }
    }

    pub fn status_sprite_position(flag: HitFlags, height: f32) -> Vec2 {
        match flag {
            CONFUSE => Vec2::new(0.0, -height),
            BLIND => Vec2::new(0.0, -height),
            FREEZE => Vec2::new(0.0, -height * 0.5),
            _ => Vec2::ZERO,
        }
    }
}
