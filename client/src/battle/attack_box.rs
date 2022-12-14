use super::{Entity, Spell};
use crate::bindable::{EntityID, HitProperties, Team, TileHighlight};

#[derive(Clone)]
pub struct AttackBox {
    pub attacker_id: EntityID,
    pub team: Team,
    pub x: i32,
    pub y: i32,
    pub props: HitProperties,
    pub highlight: bool,
}

impl AttackBox {
    pub fn new_from((x, y): (i32, i32), entity: &Entity, spell: &Spell) -> Self {
        Self {
            attacker_id: entity.id,
            team: entity.team,
            x,
            y,
            props: spell.hit_props,
            highlight: spell.requested_highlight == TileHighlight::Automatic,
        }
    }
}
