use std::sync::Arc;

use crate::block::redstone_controller::DefaultRedstoneController;
use crate::entity::player::Player;
use async_trait::async_trait;
use pumpkin_data::block::{Block, BlockFace, BlockState, LeverLikeProperties, RedstoneWireLikeProperties};
use pumpkin_data::{
    block::{BlockProperties, HorizontalFacing},
    item::Item,
};
use pumpkin_macros::pumpkin_block;
use pumpkin_protocol::server::play::SUseItemOn;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::BlockDirection;

use crate::{
    block::{pumpkin_block::PumpkinBlock, registry::BlockActionResult},
    server::Server,
    world::World,
};


#[pumpkin_block("minecraft:redstone_wire")]
pub struct RedstoneWireBlock {
    pub wire_gives_power: bool,
    pub redstone_controller: Arc<DefaultRedstoneController>,
}

#[async_trait]
impl PumpkinBlock for RedstoneWireBlock {
    async fn on_place(
        &self,
        _server: &Server,
        _world: &World,
        block: &Block,
        _face: &BlockDirection,
        _block_pos: &BlockPos,
        _use_item_on: &SUseItemOn,
        _player_direction: &HorizontalFacing,
        _other: bool,
    ) -> u16 {
        let mut wire_props = RedstoneWireLikeProperties::default(block);

        wire_props.to_state_id(block)
    }


    async fn emits_redstone_power(&self, _block: &Block, _state: &BlockState) -> bool {
        self.wire_gives_power
    }
}

impl RedstoneWireBlock {
    pub async fn get_strong_power(world: &World, block_pos: &BlockPos) -> u8 {
        0
    }
}