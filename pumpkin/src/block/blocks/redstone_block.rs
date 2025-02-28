use async_trait::async_trait;
use pumpkin_macros::pumpkin_block;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::{
    registry::State, BlockState
};

use crate::{
    block::{properties::Direction, pumpkin_block::PumpkinBlock},
    world::World,
};

#[pumpkin_block("minecraft:redstone_block")]
pub struct RedstoneBlock;

#[async_trait]
impl PumpkinBlock for RedstoneBlock {
    fn emits_redstone_power(&self, _block_state: &State) -> bool {
        true
    }

    fn get_weak_redstone_power(
        &self,
        _block_state: &BlockState,
        _world: &World,
        _pos: &BlockPos,
        _direction: &Direction,
    ) -> u8 {
        15
    }
}
