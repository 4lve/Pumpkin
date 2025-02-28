use crate::{
    block::properties::{BlockPropertyMetadata, powered::Powered},
    entity::player::Player,
};
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

const BUTTON_POWERED_STATE: usize = 2;

pub fn can_run_on_top(floor: &State) -> bool {
    is_side_solid(
        &get_block_collision_shapes(floor.id).unwrap(),
        BlockDirection::Top,
    )
}

pub async fn on_use(world: &World, location: BlockPos, server: &Server, block: &Block) {
    server
        .block_properties_manager
        .update_states(
            block,
            world.get_block_state(&location).await.unwrap(),
            &location,
            world,
            server,
            &|states: Vec<String>| async {
                let mut new_states = states;

                if new_states
                    .get(BUTTON_POWERED_STATE)
                    .is_some_and(|v| v == &Powered::False().value())
                {
                    new_states[BUTTON_POWERED_STATE] = Powered::True().value();
                    world.schedule_block_tick(&location, 20).await;
                }

                new_states
            },
        )
        .await;
}

//TODO: Use item tag or something here
#[pumpkin_block("minecraft:oak_button")]
pub struct ButtonBlock;

#[async_trait]
impl PumpkinBlock for ButtonBlock {
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

    async fn normal_use(
        &self,
        block: &Block,
        _player: &Player,
        location: BlockPos,
        server: &Server,
        world: &World,
    ) {
        on_use(world, location, server, block).await;
    }

    async fn use_with_item(
        &self,
        block: &Block,
        _player: &Player,
        location: BlockPos,
        _item: &Item,
        server: &Server,
        world: &World,
    ) -> BlockActionResult {
        on_use(world, location, server, block).await;
        BlockActionResult::Consume
    }

    async fn on_state_replaced(
        &self,
        _block: &Block,
        states: &Vec<String>,
        _server: &Server,
        _world: &World,
    ) {
        println!("Button state replaced: {:?}", states);
    }
}
