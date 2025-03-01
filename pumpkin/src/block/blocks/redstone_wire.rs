use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{
    block::{
        properties::{BlockPropertyMetadata, cardinal::North},
        redstone_controller::{DefaultRedstoneController, RedstoneController},
    },
    entity::player::Player,
};
use async_trait::async_trait;
use pumpkin_data::item::Item;
use pumpkin_macros::pumpkin_block;
use pumpkin_protocol::server::play::SUseItemOn;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::{
    BlockDirection,
    registry::{
        BLOCK_ID_BY_STATE_ID, BLOCKS_BY_ID, Block, State, get_block_by_state_id,
        get_block_collision_shapes, is_side_solid, is_solid,
    },
};

use crate::{
    block::{properties::Direction, pumpkin_block::PumpkinBlock, registry::BlockActionResult},
    server::Server,
    world::World,
};

pub const WIRE_CONNECTION_EAST: usize = 0;
pub const WIRE_CONNECTION_NORTH: usize = 1;
pub const WIRE_CONNECTION_POWER_LEVEL: usize = 2;
pub const WIRE_CONNECTION_SOUTH: usize = 3;
pub const WIRE_CONNECTION_WEST: usize = 4;
pub const REPEATER_FACING: usize = 1;
const OBSERVER_FACING: usize = 0;
// Okay, this is really confusing, in source code dot state is when it has all sides connected
const DOT_STATE: [&str; 5] = ["side", "side", "0", "side", "side"];
const DEFAULT_STATE: [&str; 5] = ["none", "none", "0", "none", "none"];

pub fn is_connected(state: &str) -> bool {
    state != North::None.value()
}

pub fn is_fully_connected(states: &Vec<String>) -> bool {
    return is_connected(states[WIRE_CONNECTION_EAST].as_str())
        && is_connected(states[WIRE_CONNECTION_NORTH].as_str())
        && is_connected(states[WIRE_CONNECTION_SOUTH].as_str())
        && is_connected(states[WIRE_CONNECTION_WEST].as_str());
}

pub fn is_not_fully_connected(states: &Vec<String>) -> bool {
    return !is_connected(states[WIRE_CONNECTION_EAST].as_str())
        && !is_connected(states[WIRE_CONNECTION_NORTH].as_str())
        && !is_connected(states[WIRE_CONNECTION_SOUTH].as_str())
        && !is_connected(states[WIRE_CONNECTION_WEST].as_str());
}

pub fn get_property_index(direction: BlockDirection) -> usize {
    match direction {
        BlockDirection::North => WIRE_CONNECTION_NORTH,
        BlockDirection::South => WIRE_CONNECTION_SOUTH,
        BlockDirection::East => WIRE_CONNECTION_EAST,
        BlockDirection::West => WIRE_CONNECTION_WEST,
        _ => WIRE_CONNECTION_NORTH,
    }
}

pub async fn connects_to(
    state: &State,
    direction: Option<BlockDirection>,
    server: &Server,
) -> bool {
    let block = get_block_by_state_id(state.id).unwrap();

    if block.name == "redstone_wire" {
        return true;
    } else if block.name == "repeater" {
        if let Some(direction) = direction {
            let repeater_state = server
                .block_properties_manager
                .get_states(block, state)
                .await;
            let facing = BlockDirection::try_from(repeater_state[REPEATER_FACING].as_str())
                .unwrap_or(BlockDirection::North);
            return facing == direction || facing == direction.opposite();
        }
    } else if block.name == "observer" {
        if let Some(direction) = direction {
            let observer_state = server
                .block_properties_manager
                .get_states(block, state)
                .await;
            let facing = BlockDirection::try_from(observer_state[OBSERVER_FACING].as_str())
                .unwrap_or(BlockDirection::North);
            return facing == direction;
        }
    } else if let Some(pumpkin_block) = server.block_registry.get_pumpkin_block(block) {
        return pumpkin_block.emits_redstone_power(state) && direction.is_some();
    }

    false
}

pub async fn get_render_connection_type(
    world: &World,
    location: BlockPos,
    direction: BlockDirection,
    is_not_solid: bool,
    server: &Server,
) -> North {
    let other_block_pos = location.offset(direction.to_offset());
    let other_block = world.get_block(&other_block_pos).await.unwrap();
    let other_block_state = world.get_block_state(&other_block_pos).await.unwrap();

    if is_not_solid {
        let is_trapdoor =
            other_block.name.contains("trapdoor") || can_run_on_top(other_block_state);
        let block_up = other_block_pos.offset(BlockDirection::Top.to_offset());
        let block_up_state = world.get_block_state(&block_up).await.unwrap();
        let connects_to_up = connects_to(block_up_state, Some(direction.opposite()), server).await;

        if is_trapdoor && connects_to_up {
            if let Some(shapes) = get_block_collision_shapes(other_block_state.id) {
                if is_side_solid(&shapes, direction.opposite()) {
                    return North::Up;
                }
            }

            return North::Side;
        }
    }

    if !connects_to(other_block_state, Some(direction), server).await
        && (is_solid(&get_block_collision_shapes(other_block_state.id).unwrap_or_default())
            || !connects_to(
                world
                    .get_block_state(&other_block_pos.offset(BlockDirection::Bottom.to_offset()))
                    .await
                    .unwrap(),
                Some(direction),
                server,
            )
            .await)
    {
        return North::None;
    }

    North::Side
}

pub async fn get_default_wire_state(
    world: &World,
    location: BlockPos,
    states: &Vec<String>,
    server: &Server,
) -> Vec<String> {
    let mut states = states.clone();
    let no_solid_block_above = !is_solid(
        &get_block_collision_shapes(
            world
                .get_block(&location.offset(BlockDirection::Top.to_offset()))
                .await
                .unwrap()
                .id,
        )
        .unwrap(),
    );

    for dir in BlockDirection::horizontal() {
        let property_index = get_property_index(dir);

        if !is_connected(states[property_index].as_str()) {
            let render_connection_type =
                get_render_connection_type(world, location, dir, no_solid_block_above, server)
                    .await;
            states[property_index] = render_connection_type.value();
        }
    }

    states
}

pub async fn get_placement_state(
    world: &World,
    location: BlockPos,
    states: &Vec<String>,
    server: &Server,
) -> Vec<String> {
    let mut states = states.clone();
    let is_disconnected = is_not_fully_connected(&states);
    let mut default_states = DEFAULT_STATE
        .to_vec()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    default_states[WIRE_CONNECTION_POWER_LEVEL] = states[WIRE_CONNECTION_POWER_LEVEL].clone();
    states = get_default_wire_state(world, location, &default_states, server).await;

    if is_disconnected && is_not_fully_connected(&states) {
        return states;
    } else {
        let north_connected = is_connected(states[WIRE_CONNECTION_NORTH].as_str());
        let south_connected = is_connected(states[WIRE_CONNECTION_SOUTH].as_str());
        let east_connected = is_connected(states[WIRE_CONNECTION_EAST].as_str());
        let west_connected = is_connected(states[WIRE_CONNECTION_WEST].as_str());

        let is_north_south_disconnected = !north_connected && !south_connected;
        let is_east_west_disconnected = !east_connected && !west_connected;

        if !west_connected && is_north_south_disconnected {
            states[WIRE_CONNECTION_WEST] = North::Side.value();
        }

        if !east_connected && is_north_south_disconnected {
            states[WIRE_CONNECTION_EAST] = North::Side.value();
        }

        if !north_connected && is_east_west_disconnected {
            states[WIRE_CONNECTION_NORTH] = North::Side.value();
        }

        if !south_connected && is_east_west_disconnected {
            states[WIRE_CONNECTION_SOUTH] = North::Side.value();
        }
        return states;
    }
}

pub fn can_run_on_top(floor: &State) -> bool {
    is_solid(&get_block_collision_shapes(floor.id).unwrap())
}

pub async fn get_strong_power(
    redstone_wire: &RedstoneWireBlock,
    world: &World,
    pos: &BlockPos,
    server: &Server,
) -> u8 {
    redstone_wire
        .wire_gives_power
        .store(false, Ordering::Relaxed);
    let power = world.get_received_redstone_power(pos, server).await;
    redstone_wire
        .wire_gives_power
        .store(true, Ordering::Relaxed);
    power
}

pub async fn update_neighbors(world: &World, pos: &BlockPos, server: &Server) {
    let block = world.get_block(pos).await.unwrap();
    if block.name == "redstone_wire" {
        world.update_neighbors(pos, server, None).await;

        for direction in BlockDirection::vertical() {
            let pos = pos.offset(direction.to_offset());
            world.update_neighbors(&pos, server, None).await;
        }
    }
}

pub async fn update_offset_neighbors(world: &World, pos: &BlockPos, server: &Server) {
    for direction in BlockDirection::horizontal() {
        let pos = pos.offset(direction.to_offset());
        world.update_neighbors(&pos, server, None).await;
    }

    for direction in BlockDirection::horizontal() {
        let pos = pos.offset(direction.to_offset());
        if is_solid(&get_block_collision_shapes(world.get_block(&pos).await.unwrap().id).unwrap()) {
            world
                .update_neighbors(&pos.offset(BlockDirection::Top.to_offset()), server, None)
                .await;
        } else {
            world
                .update_neighbors(
                    &pos.offset(BlockDirection::Bottom.to_offset()),
                    server,
                    None,
                )
                .await;
        }
    }
}

#[pumpkin_block("minecraft:redstone_wire")]
pub struct RedstoneWireBlock {
    pub wire_gives_power: AtomicBool,
    pub redstone_controller: DefaultRedstoneController,
}

#[async_trait]
impl PumpkinBlock for RedstoneWireBlock {
    async fn can_place_on_side(
        &self,
        world: &World,
        location: BlockPos,
        _side: BlockDirection,
    ) -> bool {
        let target_block_pos = BlockPos(location.0 + BlockDirection::Bottom.to_offset());
        can_run_on_top(world.get_block_state(&target_block_pos).await.unwrap())
            || world.get_block(&target_block_pos).await.unwrap().name == "hopper"
    }

    async fn normal_use(
        &self,
        block: &Block,
        player: &Player,
        location: BlockPos,
        server: &Server,
        world: &World,
    ) {
        let block_state = world.get_block_state(&location).await.unwrap();
        let states = server
            .block_properties_manager
            .get_states(block, block_state)
            .await;

        if player.abilities.lock().await.allow_modify_world {
            if is_fully_connected(&states) || is_not_fully_connected(&states) {
                let mut new_states: Vec<String> = if is_fully_connected(&states) {
                    DEFAULT_STATE
                        .to_vec()
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    DOT_STATE.to_vec().iter().map(|s| s.to_string()).collect()
                };

                new_states[WIRE_CONNECTION_POWER_LEVEL] =
                    states[WIRE_CONNECTION_POWER_LEVEL].clone();

                new_states = get_placement_state(world, location, &new_states, server).await;

                if new_states != states {
                    server
                        .block_properties_manager
                        .set_block_state(block, &location, world, server, new_states)
                        .await
                }
            }
        }
    }

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
            let states = DOT_STATE.to_vec().iter().map(|s| s.to_string()).collect();

            let states = get_placement_state(world, *block_pos, &states, server).await;

            let state_mapping = properties.state_mappings.get(&states);
            if let Some(state_mapping) = state_mapping {
                return block.states[0].id + state_mapping;
            }
            log::error!("Failed to get Block Properties mapping for {}", block.name);
        }
        block.default_state_id
    }

    async fn placed(
        &self,
        _block: &Block,
        _player: &Player,
        location: BlockPos,
        server: &Server,
        world: &World,
    ) {
        let state = world.get_block_state(&location).await.unwrap();
        self.redstone_controller
            .update(self, world, server, &location, &state)
            .await;

        for direction in BlockDirection::vertical() {
            let pos = location.offset(direction.to_offset());
            world.update_neighbors(&pos, server, None).await;
        }

        update_offset_neighbors(world, &location, server).await;
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
        println!("Redstone wire used");
        BlockActionResult::Consume
    }

    async fn update_state_for_neighbor_update(
        &self,
        block: &Block,
        world: &World,
        server: &Server,
        state: &State,
        pos: &BlockPos,
        direction: &BlockDirection,
        neighbor_pos: &BlockPos,
        neighbor_state: &State,
    ) {
        if direction == &BlockDirection::Bottom {
            if !can_run_on_top(neighbor_state) {
                // Air
                world
                    .set_block_state(pos, BLOCKS_BY_ID[&0].states[0].id)
                    .await;
                return ();
            } else {
                return ();
            }
        } else if direction == &BlockDirection::Top {
            let states = server
                .block_properties_manager
                .get_states(block, state)
                .await;
            let new_states = get_placement_state(world, *pos, &states, server).await;
            server
                .block_properties_manager
                .set_block_state(block, pos, world, server, new_states)
                .await;
        } else {
            let mut states = server
                .block_properties_manager
                .get_states(block, state)
                .await;
            let connection_type = get_render_connection_type(
                world,
                *pos,
                *direction,
                !is_solid(&get_block_collision_shapes(neighbor_state.id).unwrap()),
                server,
            )
            .await;

            if is_connected(connection_type.value().as_str())
                == is_connected(states[get_property_index(*direction)].as_str())
                && !is_fully_connected(&states)
            {
                states[get_property_index(*direction)] = connection_type.value();
                server
                    .block_properties_manager
                    .set_block_state(block, pos, world, server, states)
                    .await;
            } else {
                let states = server
                    .block_properties_manager
                    .get_states(block, state)
                    .await;
                let mut dot_state = DOT_STATE
                    .to_vec()
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                dot_state[get_property_index(*direction)] = connection_type.value();
                dot_state[WIRE_CONNECTION_POWER_LEVEL] =
                    states[WIRE_CONNECTION_POWER_LEVEL].clone();

                let placement_state = get_placement_state(world, *pos, &dot_state, server).await;

                server
                    .block_properties_manager
                    .set_block_state(block, pos, world, server, placement_state)
                    .await;
            }
        }
    }

    async fn on_state_replaced(
        &self,
        block: &Block,
        _states: &Vec<String>,
        pos: &BlockPos,
        server: &Server,
        world: &World,
    ) {
    }

    // TODO: Bug when breaking a redstone wire under or above
    async fn neighbor_update(
        &self,
        block: &Block,
        world: &World,
        pos: &BlockPos,
        server: &Server,
        state: &State,
        source_block: &Block,
        _wire_orentation: Option<North>,
        _notify: bool,
    ) {
        let can_run_on_top = can_run_on_top(
            world
                .get_block_state(&pos.offset(BlockDirection::Bottom.to_offset()))
                .await
                .unwrap(),
        );
        if can_run_on_top {
            self.redstone_controller
                .update(self, world, server, pos, state)
                .await;
        } else {
            // TODO: Drop
            world.set_block_state(pos, 0).await;
        }
    }

    async fn get_strong_redstone_power(
        &self,
        block_state: &State,
        world: &World,
        pos: &BlockPos,
        direction: &BlockDirection,
        server: &Server,
    ) -> u8 {
        let wire_gives_power = self.wire_gives_power.load(Ordering::Relaxed);
        if wire_gives_power {
            self.get_weak_redstone_power(block_state, world, pos, direction, server)
                .await
        } else {
            0
        }
    }

    async fn get_weak_redstone_power(
        &self,
        block_state: &State,
        world: &World,
        pos: &BlockPos,
        direction: &BlockDirection,
        server: &Server,
    ) -> u8 {
        let wire_gives_power = self.wire_gives_power.load(Ordering::Relaxed);
        if wire_gives_power && direction != &BlockDirection::Bottom {
            let states = server
                .block_properties_manager
                .get_states(get_block_by_state_id(block_state.id).unwrap(), block_state)
                .await;
            let power = states[WIRE_CONNECTION_POWER_LEVEL].clone();
            let power = power.parse::<u8>().unwrap();
            if power == 0 {
                return 0;
            } else {
                if direction != &BlockDirection::Top
                    && !is_connected(
                        get_placement_state(world, *pos, &states, server).await
                            [get_property_index(direction.opposite())]
                        .as_str(),
                    )
                {
                    return 0;
                } else {
                    return power;
                }
            }
        } else {
            0
        }
    }

    fn emits_redstone_power(&self, _block_state: &State) -> bool {
        self.wire_gives_power.load(Ordering::Relaxed)
    }
}
