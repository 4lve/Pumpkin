use std::sync::Arc;

use async_trait::async_trait;
use pumpkin_data::block::{Block, BlockState};
use pumpkin_data::item::Item;
use pumpkin_data::{
    screen::WindowType,
    sound::{Sound, SoundCategory},
};
use pumpkin_macros::pumpkin_block;
use pumpkin_protocol::{client::play::CBlockAction, codec::var_int::VarInt};
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::registry::get_block;
use pumpkin_world::block_entities::chest::ChestBlockEntity;

use crate::world::World;
use crate::{
    block::{pumpkin_block::PumpkinBlock, registry::BlockActionResult},
    entity::player::Player,
    server::Server,
};

#[derive(PartialEq)]
pub enum ChestState {
    IsOpened,
    IsClosed,
}

#[pumpkin_block("minecraft:chest")]
pub struct ChestBlock;

#[async_trait]
impl PumpkinBlock for ChestBlock {
    async fn normal_use(
        &self,
        block: &Block,
        player: &Player,
        _location: BlockPos,
        server: &Server,
        _world: &Arc<World>,
    ) {
    }

    async fn placed(
        &self,
        world: &Arc<World>,
        _block: &Block,
        _state_id: u16,
        pos: &BlockPos,
        _old_state_id: u16,
        _notify: bool,
    ) {
        let chest = ChestBlockEntity::new(*pos);
        world.add_block_entity(Arc::new(chest)).await;
    }

    async fn on_state_replaced(
        &self,
        world: &Arc<World>,
        _block: &Block,
        location: BlockPos,
        _old_state_id: u16,
        _moved: bool,
    ) {
        world.remove_block_entity(&location).await;
    }

    async fn use_with_item(
        &self,
        block: &Block,
        player: &Player,
        _location: BlockPos,
        _item: &Item,
        server: &Server,
        _world: &Arc<World>,
    ) -> BlockActionResult {
        BlockActionResult::Consume
    }
}
