use crate::block::registry::BlockActionResult;
use crate::entity::player::Player;
use crate::server::Server;
use crate::world::World;
use async_trait::async_trait;
use pumpkin_data::item::Item;
use pumpkin_inventory::OpenContainer;
use pumpkin_protocol::server::play::SUseItemOn;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::registry::{Block, State};
use pumpkin_world::block::{BlockDirection, BlockState};
use std::sync::Arc;

use super::properties::cardinal::North;
use super::properties::Direction;

pub trait BlockMetadata {
    const NAMESPACE: &'static str;
    const ID: &'static str;
    fn name(&self) -> String {
        format!("{}:{}", Self::NAMESPACE, Self::ID)
    }
}

#[async_trait]
pub trait PumpkinBlock: Send + Sync {
    async fn normal_use(
        &self,
        _block: &Block,
        _player: &Player,
        _location: BlockPos,
        _server: &Server,
        _world: &World,
    ) {
    }
    fn should_drop_items_on_explosion(&self) -> bool {
        true
    }
    async fn explode(
        &self,
        _block: &Block,
        _world: &Arc<World>,
        _location: BlockPos,
        _server: &Server,
    ) {
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
        BlockActionResult::Continue
    }

    #[allow(clippy::too_many_arguments)]
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
        server
            .block_properties_manager
            .on_place_state(
                world,
                server,
                block,
                face,
                block_pos,
                use_item_on,
                player_direction,
                other,
            )
            .await
    }

    async fn can_place_on_side(
        &self,
        _world: &World,
        _location: BlockPos,
        _side: BlockDirection,
    ) -> bool {
        true
    }

    async fn placed(
        &self,
        _block: &Block,
        _player: &Player,
        _location: BlockPos,
        _server: &Server,
    ) {
    }

    async fn broken(
        &self,
        _block: &Block,
        _player: &Player,
        _location: BlockPos,
        _server: &Server,
    ) {
    }

    async fn close(
        &self,
        _block: &Block,
        _player: &Player,
        _location: BlockPos,
        _server: &Server,
        _container: &mut OpenContainer,
    ) {
    }

    fn emits_redstone_power(&self, _block_state: &State) -> bool {
        false
    }

    fn get_weak_redstone_power(
        &self,
        _block_state: &State,
        _world: &World,
        _pos: &BlockPos,
        _direction: &BlockDirection,
    ) -> u8 {
        0
    }

    fn get_strong_redstone_power(
        &self,
        _block_state: &State,
        _world: &World,
        _pos: &BlockPos,
        _direction: &BlockDirection,
    ) -> u8 {
        0
    }

    async fn scheduled_tick(
        &self,
        block: &Block,
        block_state: &State,
        server: &Server,
        world: &World,
        location: &BlockPos,
    ) {
        let new_state = server
            .block_properties_manager
            .on_scheduled_tick(block, block_state, world, server)
            .await;

        world.set_block_state(&location, new_state).await;
    }

    async fn on_state_replaced(
        &self,
        _block: &Block,
        _states: &Vec<String>,
        _server: &Server,
        _world: &World,
    ) {
    }

    async fn update_state_for_neighbor_update(
        &self,
        _block: &Block,
        _world: &World,
        _server: &Server,
        _state: &State,
        _pos: &BlockPos,
        _direction: &BlockDirection,
        _neighbor_pos: &BlockPos,
        _neighbor_state: &State,
    ) {
    }

    async fn neighbor_update(
        &self,
        _block: &Block,
        _world: &World,
        _server: &Server,
        _state: &State,
        _source_block: &Block,
        _wire_orentation: Option<North>,
        _notify: bool,
    ) {
    }

    async fn update(
        &self,
        _block: &Block,
        _world: &World,
        _server: &Server,
        _state: &State,
        _pos: &BlockPos,
        _wire_orentation: Option<North>,
        _block_added: bool,
    ) {
    }

    fn is_gate(&self) -> bool {
        false
    }
}
