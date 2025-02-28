use crate::entity::player::Player;
use async_trait::async_trait;
use pumpkin_data::item::Item;
use pumpkin_macros::pumpkin_block;
use pumpkin_protocol::server::play::SUseItemOn;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::{
    BlockDirection,
    registry::{Block, State, get_block_collision_shapes, is_side_solid},
};

use crate::{
    block::{properties::Direction, pumpkin_block::PumpkinBlock, registry::BlockActionResult},
    server::Server,
    world::World,
};

pub fn can_run_on_top(floor: &State) -> bool {
    is_side_solid(
        &get_block_collision_shapes(floor.id).unwrap(),
        BlockDirection::Top,
    )
}

#[pumpkin_block("minecraft:lever")]
pub struct LeverBlock;

#[async_trait]
impl PumpkinBlock for LeverBlock {
    async fn can_place_on_side(
        &self,
        world: &World,
        location: BlockPos,
        _side: BlockDirection,
    ) -> bool {
        let target_block_pos = BlockPos(location.0 + BlockDirection::Bottom.to_offset());
        can_run_on_top(world.get_block_state(&target_block_pos).await.unwrap())
    }

    async fn on_place(
        &self,
        server: &Server,
        world: &World,
        block: &Block,
        face: &BlockDirection,
        block_pos: &BlockPos,
        use_item_on: &SUseItemOn,
        player_direction: &Direction,
        other: bool,
    ) -> u16 {
        let face = match face {
            BlockDirection::Bottom | BlockDirection::Top => *face,
            _ => face.opposite(),
        };

        server
            .block_properties_manager
            .on_place_state(
                world,
                server,
                block,
                &face,
                block_pos,
                use_item_on,
                player_direction,
                other,
            )
            .await
    }

    async fn use_with_item(
        &self,
        _block: &Block,
        _player: &Player,
        _location: BlockPos,
        _item: &Item,
        _server: &Server,
        _world: &World,
    ) -> BlockActionResult {
        BlockActionResult::Consume
    }

    fn emits_redstone_power(&self, _block_state: &State) -> bool {
        true
    }
}
