use crate::entity::player::Player;
use async_trait::async_trait;
use pumpkin_data::block::{Block, BlockFace, BlockState, LeverLikeProperties};
use pumpkin_data::{
    block::{BlockProperties, HorizontalFacing},
    item::Item,
};
use pumpkin_macros::pumpkin_block;
use pumpkin_protocol::server::play::SUseItemOn;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::{BlockDirection, HorizontalFacingHelper};

use crate::{
    block::{pumpkin_block::PumpkinBlock, registry::BlockActionResult},
    server::Server,
    world::World,
};

async fn toggle_lever(server: &Server, world: &World, block_pos: &BlockPos) {
    let (block, state) = world.get_block_and_block_state(block_pos).await.unwrap();

    let mut lever_props = LeverLikeProperties::from_state_id(state.id, &block);
    lever_props.powered = lever_props.powered.flip();
    world
        .set_block_state(block_pos, lever_props.to_state_id(&block))
        .await;

    world.update_neighbors(server, block_pos, None).await;
}

#[pumpkin_block("minecraft:lever")]
pub struct LeverBlock;

#[async_trait]
impl PumpkinBlock for LeverBlock {
    async fn on_place(
        &self,
        _server: &Server,
        _world: &World,
        block: &Block,
        face: &BlockDirection,
        _block_pos: &BlockPos,
        _use_item_on: &SUseItemOn,
        player_direction: &HorizontalFacing,
        _other: bool,
    ) -> u16 {
        let mut lever_props = LeverLikeProperties::from_state_id(block.default_state_id, block);

        match face {
            BlockDirection::Up => lever_props.face = BlockFace::Ceiling,
            BlockDirection::Down => lever_props.face = BlockFace::Floor,
            _ => lever_props.face = BlockFace::Wall,
        }

        if face == &BlockDirection::Up || face == &BlockDirection::Down {
            lever_props.facing = *player_direction;
        } else {
            lever_props.facing = face.opposite().to_cardinal_direction();
        };

        lever_props.to_state_id(block)
    }

    async fn use_with_item(
        &self,
        _block: &Block,
        _player: &Player,
        location: BlockPos,
        _item: &Item,
        server: &Server,
        world: &World,
    ) -> BlockActionResult {
        toggle_lever(server, world, &location).await;
        BlockActionResult::Consume
    }

    async fn normal_use(
        &self,
        _block: &Block,
        _player: &Player,
        location: BlockPos,
        server: &Server,
        world: &World,
    ) {
        toggle_lever(server, world, &location).await;
    }

    async fn emits_redstone_power(&self, _state: &BlockState) -> bool {
        true
    }

    async fn get_weak_redstone_power(
        &self,
        _server: &Server,
        _block: &Block,
        _world: &World,
        _block_pos: &BlockPos,
        state: &BlockState,
        _direction: &BlockDirection,
    ) -> u8 {
        let lever_props = LeverLikeProperties::from_state_id(state.id, _block);
        if lever_props.powered.to_bool() { 15 } else { 0 }
    }

    async fn get_strong_redstone_power(
        &self,
        _server: &Server,
        _block: &Block,
        _world: &World,
        _block_pos: &BlockPos,
        state: &BlockState,
        direction: &BlockDirection,
    ) -> u8 {
        let lever_props = LeverLikeProperties::from_state_id(state.id, _block);
        if lever_props.powered.to_bool() && *direction == lever_props.facing.to_block_direction() {
            15
        } else {
            0
        }
    }
}
