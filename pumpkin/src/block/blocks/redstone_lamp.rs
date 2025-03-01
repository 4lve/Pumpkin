use async_trait::async_trait;
use pumpkin_macros::pumpkin_block;
use pumpkin_protocol::server::play::SUseItemOn;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::{
    BlockDirection, BlockState,
    registry::{Block, State},
};

use crate::{
    block::{
        properties::{Direction, cardinal::North},
        pumpkin_block::PumpkinBlock,
    },
    server::Server,
    world::World,
};

const DEFAULT_STATE: [&str; 1] = ["false"];
#[pumpkin_block("minecraft:redstone_lamp")]
pub struct RedstoneLamp;

#[async_trait]
impl PumpkinBlock for RedstoneLamp {
    #[allow(clippy::too_many_arguments)]
    async fn on_place(
        &self,
        server: &Server,
        world: &World,
        block: &Block,
        _face: &BlockDirection,
        block_pos: &BlockPos,
        _use_item_on: &SUseItemOn,
        _player_direction: &Direction,
        _other: bool,
    ) -> u16 {
        if let Some(properties) = server
            .block_properties_manager
            .properties_registry
            .get(&block.id)
        {
            let mut states: Vec<String> = DEFAULT_STATE
                .to_vec()
                .iter()
                .map(|s| s.to_string())
                .collect();

            let is_receiving_power = world.is_receiving_redstone_power(block_pos, server).await;
            if is_receiving_power {
                states[0] = "true".to_string();
            }

            let state_mapping = properties.state_mappings.get(&states);
            if let Some(state_mapping) = state_mapping {
                return block.states[0].id + state_mapping;
            }
            log::error!("Failed to get Block Properties mapping for {}", block.name);
        }
        block.default_state_id
    }

    async fn neighbor_update(
        &self,
        block: &Block,
        world: &World,
        pos: &BlockPos,
        server: &Server,
        state: &State,
        _source_block: &Block,
        _wire_orentation: Option<North>,
        _notify: bool,
    ) {
        let lit = server
            .block_properties_manager
            .get_states(block, state)
            .await[0]
            .clone();
        if lit == "true" && !world.is_receiving_redstone_power(pos, server).await {
            world.schedule_block_tick(&pos, 4).await;
        } else if lit == "false" && world.is_receiving_redstone_power(pos, server).await {
            server
                .block_properties_manager
                .set_block_state(block, pos, world, server, vec!["true".to_string()])
                .await;
        }
    }

    async fn scheduled_tick(
        &self,
        block: &Block,
        block_state: &State,
        server: &Server,
        world: &World,
        location: &BlockPos,
    ) {
        let lit = server
            .block_properties_manager
            .get_states(block, block_state)
            .await[0]
            .clone();
        if lit == "true" && !world.is_receiving_redstone_power(location, server).await {
            server
                .block_properties_manager
                .set_block_state(block, location, world, server, vec!["false".to_string()])
                .await;
        }
    }
}
