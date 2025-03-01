use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::{
    BlockDirection,
    registry::{State, get_block_by_state_id, get_block_collision_shapes, is_solid},
};

use crate::{server::Server, world::World};

use super::{
    blocks::redstone_wire::{RedstoneWireBlock, WIRE_CONNECTION_POWER_LEVEL, get_strong_power},
    properties::cardinal::North,
    pumpkin_block::PumpkinBlock,
};

pub trait RedstoneController {
    async fn update(
        &self,
        wire: &RedstoneWireBlock,
        world: &World,
        server: &Server,
        pos: &BlockPos,
        state: &State,
    ) -> ();

    async fn get_strong_power_at(
        &self,
        wire: &RedstoneWireBlock,
        world: &World,
        server: &Server,
        pos: &BlockPos,
    ) -> u8;

    async fn get_wire_power_at(&self, world: &World, server: &Server, pos: &BlockPos) -> u8;

    async fn calculate_wire_power(
        &self,
        wire: &RedstoneWireBlock,
        world: &World,
        server: &Server,
        pos: &BlockPos,
    ) -> u8;

    async fn calculate_power_at(
        &self,
        wire: &RedstoneWireBlock,
        world: &World,
        server: &Server,
        pos: &BlockPos,
    ) -> u8;
}

pub struct DefaultRedstoneController;

impl RedstoneController for DefaultRedstoneController {
    async fn get_strong_power_at(
        &self,
        wire: &RedstoneWireBlock,
        world: &World,
        server: &Server,
        pos: &BlockPos,
    ) -> u8 {
        get_strong_power(wire, world, pos, server).await
    }

    async fn get_wire_power_at(&self, world: &World, server: &Server, pos: &BlockPos) -> u8 {
        let block = world.get_block(pos).await.unwrap();
        if block.name == "redstone_wire" {
            let state = world.get_block_state(pos).await.unwrap();
            server
                .block_properties_manager
                .get_states(block, state)
                .await[2]
                .parse::<u8>()
                .unwrap()
        } else {
            0
        }
    }

    async fn calculate_wire_power(
        &self,
        wire: &RedstoneWireBlock,
        world: &World,
        server: &Server,
        pos: &BlockPos,
    ) -> u8 {
        let mut max_power = 0;
        for direction in BlockDirection::horizontal() {
            let other_pos = pos.offset(direction.to_offset());
            let other_state = world.get_block_state(&other_pos).await.unwrap();
            max_power = std::cmp::max(
                max_power,
                self.get_wire_power_at(world, server, &other_pos).await,
            );
            let block_up_pos = pos.offset(BlockDirection::Top.to_offset());
            let block_up_state = world.get_block_state(&block_up_pos).await.unwrap();

            if is_solid(&get_block_collision_shapes(other_state.id).unwrap())
                && !is_solid(&get_block_collision_shapes(block_up_state.id).unwrap())
            {
                let other_up_pos = other_pos.offset(BlockDirection::Top.to_offset());
                max_power = std::cmp::max(
                    max_power,
                    self.get_wire_power_at(world, server, &other_up_pos).await,
                );
            } else if !is_solid(&get_block_collision_shapes(other_state.id).unwrap()) {
                let other_down_pos = other_pos.offset(BlockDirection::Bottom.to_offset());
                max_power = std::cmp::max(
                    max_power,
                    self.get_wire_power_at(world, server, &other_down_pos).await,
                );
            }
        }

        if max_power == 0 {
            return 0;
        } else {
            return std::cmp::max(max_power - 1, 0);
        }
    }

    async fn calculate_power_at(
        &self,
        wire: &RedstoneWireBlock,
        world: &World,
        server: &Server,
        pos: &BlockPos,
    ) -> u8 {
        let wire_power = self.calculate_wire_power(wire, world, server, pos).await;
        let block_power = self.get_strong_power_at(wire, world, server, pos).await;
        std::cmp::max(wire_power, block_power)
    }

    async fn update(
        &self,
        wire: &RedstoneWireBlock,
        world: &World,
        server: &Server,
        pos: &BlockPos,
        state: &State,
    ) -> () {
        let power = self.calculate_power_at(wire, world, server, pos).await;
        let mut states = server
            .block_properties_manager
            .get_states(get_block_by_state_id(state.id).unwrap(), state)
            .await;

        if states[WIRE_CONNECTION_POWER_LEVEL] == power.to_string() {
            return;
        }

        states[WIRE_CONNECTION_POWER_LEVEL] = power.to_string();
        server
            .block_properties_manager
            .set_block_state(
                get_block_by_state_id(state.id).unwrap(),
                pos,
                world,
                server,
                states,
            )
            .await;

        let mut update_list: Vec<BlockPos> = vec![];

        update_list.push(pos.clone());

        for direction in BlockDirection::all() {
            let pos = pos.offset(direction.to_offset());
            update_list.push(pos);
        }

        for pos in update_list {
            world.update_neighbors(&pos, server, None).await;
        }
    }
}
