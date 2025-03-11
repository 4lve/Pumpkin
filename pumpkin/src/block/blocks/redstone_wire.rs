use std::sync::atomic::AtomicBool;

use crate::block::redstone_view::get_received_redstone_power;
use crate::entity::player::Player;
use async_trait::async_trait;
use pumpkin_data::block::{
    Block, BlockState, EastWireConnection, Integer0To15, NorthWireConnection,
    ObserverLikeProperties, RedstoneWireLikeProperties, RepeaterLikeProperties,
    SouthWireConnection, WestWireConnection,
};
use pumpkin_data::block::{BlockProperties, HorizontalFacing};
use pumpkin_data::tag::Tagable;
use pumpkin_macros::pumpkin_block;
use pumpkin_protocol::server::play::SUseItemOn;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::block::BlockDirection;

use crate::{block::pumpkin_block::PumpkinBlock, server::Server, world::World};

/// This is a bit confusing but dot state is actually the X shape
const DOT_STATE: RedstoneWireLikeProperties = RedstoneWireLikeProperties {
    power: Integer0To15::L0,
    east: EastWireConnection::Side,
    north: NorthWireConnection::Side,
    south: SouthWireConnection::Side,
    west: WestWireConnection::Side,
};

#[pumpkin_block("minecraft:redstone_wire")]
pub struct RedstoneWireBlock {
    pub wire_gives_power: AtomicBool,
}

#[async_trait]
impl PumpkinBlock for RedstoneWireBlock {
    async fn on_place(
        &self,
        server: &Server,
        world: &World,
        block: &Block,
        _face: &BlockDirection,
        block_pos: &BlockPos,
        _use_item_on: &SUseItemOn,
        _player_direction: &HorizontalFacing,
        _other: bool,
    ) -> u16 {
        let wire_props = Self::get_placement_state(server, world, block_pos, DOT_STATE).await;

        wire_props.to_state_id(block)
    }

    async fn placed(
        &self,
        block: &Block,
        _player: &Player,
        location: BlockPos,
        server: &Server,
        world: &World,
    ) {
        let state = world.get_block_state(&location).await.unwrap();
        crate::block::redstone_controller::update(
            server, world, &location, block, &state, None, true,
        )
        .await;

        for direction in BlockDirection::vertical() {
            world
                .update_neighbors(server, &location.offset(direction.to_offset()), None)
                .await;
        }

        RedstoneWireBlock::update_offset_neighbors(server, world, &location).await;
    }

    async fn emits_redstone_power(&self, _state: &BlockState) -> bool {
        self.wire_gives_power
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn get_strong_power(&self, server: &Server, world: &World, block_pos: &BlockPos) -> u8 {
        self.wire_gives_power
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let power = get_received_redstone_power(server, world, block_pos).await;
        self.wire_gives_power
            .store(true, std::sync::atomic::Ordering::Relaxed);
        power
    }

    async fn on_neighbor_update(
        &self,
        server: &Server,
        world: &World,
        block: &Block,
        block_pos: &BlockPos,
        _source_face: &BlockDirection,
        _source_block_pos: &BlockPos,
    ) {
        let block_state = world.get_block_state(block_pos).await.unwrap();

        if self.can_place(server, world, block_pos).await {
            crate::block::redstone_controller::update(
                server,
                world,
                block_pos,
                block,
                &block_state,
                None,
                false,
            )
            .await;
        } else {
            // TODO: Break the block with drops
            world
                .set_block_state(block_pos, Block::AIR.default_state_id)
                .await;
        }
    }

    async fn get_state_for_neighbor_update(
        &self,
        server: &Server,
        world: &World,
        _block: &Block,
        block_pos: &BlockPos,
        state: &BlockState,
        source_face: &BlockDirection,
        source_block_pos: &BlockPos,
        _neighbor_state: &BlockState,
    ) -> u16 {
        //TODO: Fix this
        if source_face == &BlockDirection::Down {
            let floor = world.get_block_state(source_block_pos).await.unwrap();
            if !Self::can_run_on_top(&floor) {
                return Block::AIR.default_state_id;
            }
        } else if source_face == &BlockDirection::Up {
            let placement_state = Self::get_placement_state(
                server,
                world,
                block_pos,
                RedstoneWireLikeProperties::from_state_id(state.id, &Block::REDSTONE_WIRE),
            )
            .await;

            return placement_state.to_state_id(&Block::REDSTONE_WIRE);
        } else {
            let mut wire_props =
                RedstoneWireLikeProperties::from_state_id(state.id, &Block::REDSTONE_WIRE);
            let block_above = world.get_block_state(&block_pos.up()).await.unwrap();
            let wire_connection_type = Self::get_render_connection_type(
                server,
                world,
                *block_pos,
                *source_face,
                !block_above.is_solid,
            )
            .await;

            if wire_connection_type.is_connected()
                == wire_props.get_connection_type(*source_face).is_connected()
                && !Self::is_fully_connected(&wire_props)
            {
                wire_connection_type.set_connection(&mut wire_props, *source_face);

                return wire_props.to_state_id(&Block::REDSTONE_WIRE);
            } else {
                let mut new_props = DOT_STATE;
                new_props.power = wire_props.power;
                wire_connection_type.set_connection(&mut new_props, *source_face);
                new_props = Self::get_placement_state(server, world, block_pos, new_props).await;

                return new_props.to_state_id(&Block::REDSTONE_WIRE);
            }
        }

        state.id
    }

    async fn can_place(&self, _server: &Server, world: &World, block_pos: &BlockPos) -> bool {
        let floor = world.get_block_state(&block_pos.down()).await.unwrap();
        Self::can_run_on_top(&floor)
    }
}

impl RedstoneWireBlock {
    async fn update_offset_neighbors(server: &Server, world: &World, block_pos: &BlockPos) {
        for direction in BlockDirection::horizontal() {
            Self::update_neighbors(server, world, &block_pos.offset(direction.to_offset())).await;
        }

        for direction in BlockDirection::horizontal() {
            let other_pos = block_pos.offset(direction.to_offset());
            let other_state = world.get_block_state(&other_pos).await.unwrap();

            if other_state.is_solid {
                Self::update_neighbors(server, world, &other_pos.up()).await;
            } else {
                Self::update_neighbors(server, world, &other_pos.down()).await;
            }
        }
    }

    async fn update_neighbors(server: &Server, world: &World, block_pos: &BlockPos) {
        let block = world.get_block(block_pos).await.unwrap();
        if block == Block::REDSTONE_WIRE {
            world.update_neighbors(server, block_pos, None).await;
            for direction in BlockDirection::all() {
                world
                    .update_neighbors(server, &block_pos.offset(direction.to_offset()), None)
                    .await;
            }
        }
    }

    fn is_fully_connected(props: &RedstoneWireLikeProperties) -> bool {
        props.north.is_connected()
            && props.south.is_connected()
            && props.east.is_connected()
            && props.west.is_connected()
    }

    fn is_not_connected(props: &RedstoneWireLikeProperties) -> bool {
        !props.north.is_connected()
            && !props.south.is_connected()
            && !props.east.is_connected()
            && !props.west.is_connected()
    }

    fn is_side_connected(props: &RedstoneWireLikeProperties, direction: &BlockDirection) -> bool {
        match direction {
            BlockDirection::North => props.north.is_connected(),
            BlockDirection::South => props.south.is_connected(),
            BlockDirection::East => props.east.is_connected(),
            BlockDirection::West => props.west.is_connected(),
            _ => false,
        }
    }
    async fn get_default_wire_state(
        server: &Server,
        world: &World,
        block_pos: &BlockPos,
        props: RedstoneWireLikeProperties,
    ) -> RedstoneWireLikeProperties {
        let mut props = props;
        let not_solid = !world
            .get_block_state(&block_pos.up())
            .await
            .unwrap()
            .is_solid;

        for direction in BlockDirection::horizontal() {
            if !Self::is_side_connected(&props, &direction) {
                let connection_type = Self::get_render_connection_type(
                    server, world, *block_pos, direction, not_solid,
                )
                .await;
                connection_type.set_connection(&mut props, direction);
            }
        }

        props
    }

    async fn get_placement_state(
        server: &Server,
        world: &World,
        block_pos: &BlockPos,
        old_props: RedstoneWireLikeProperties,
    ) -> RedstoneWireLikeProperties {
        let not_connected = Self::is_not_connected(&old_props);
        let mut props = RedstoneWireLikeProperties::default(&Block::REDSTONE_WIRE);
        props.power = old_props.power;

        let mut props = Self::get_default_wire_state(server, world, block_pos, props).await;

        if not_connected && Self::is_not_connected(&props) {
            return props;
        }

        let north_connected = props.north.is_connected();
        let south_connected = props.south.is_connected();
        let east_connected = props.east.is_connected();
        let west_connected = props.west.is_connected();

        let is_north_south_disconnected = !north_connected && !south_connected;
        let is_east_west_disconnected = !east_connected && !west_connected;

        if !west_connected && is_north_south_disconnected {
            props.west = WestWireConnection::Side;
        }

        if !east_connected && is_north_south_disconnected {
            props.east = EastWireConnection::Side;
        }

        if !north_connected && is_east_west_disconnected {
            props.north = NorthWireConnection::Side;
        }

        if !south_connected && is_east_west_disconnected {
            props.south = SouthWireConnection::Side;
        }

        props
    }

    async fn get_render_connection_type(
        server: &Server,
        world: &World,
        location: BlockPos,
        direction: BlockDirection,
        not_solid: bool,
    ) -> WireConnectionType {
        let other_block_pos = location.offset(direction.to_offset());
        let (other_block, other_block_state) = world
            .get_block_and_block_state(&other_block_pos)
            .await
            .unwrap();

        if not_solid {
            let can_run_on_top = other_block.is_tagged_with("minecraft:trapdoors").unwrap()
                || Self::can_run_on_top(&other_block_state);

            let connects_up = Self::connects_to(
                server,
                &world.get_block_state(&other_block_pos.up()).await.unwrap(),
                None,
            )
            .await;

            if can_run_on_top && connects_up {
                // TODO: Check if side is solid instead
                if other_block_state.is_solid {
                    return WireConnectionType::Up;
                }
                return WireConnectionType::Side;
            }
        }

        if !Self::connects_to(server, &other_block_state, Some(direction)).await
            && (other_block_state.is_solid
                || !Self::connects_to(
                    server,
                    &world
                        .get_block_state(&other_block_pos.down())
                        .await
                        .unwrap(),
                    None,
                )
                .await)
        {
            return WireConnectionType::None;
        }
        return WireConnectionType::Side;
    }

    pub async fn connects_to(
        server: &Server,
        state: &BlockState,
        direction: Option<BlockDirection>,
    ) -> bool {
        let block = Block::from_state_id(state.id).unwrap();

        if block == Block::REDSTONE_WIRE {
            return true;
        } else if block == Block::REPEATER {
            if let Some(direction) = direction {
                let repeater_props =
                    RepeaterLikeProperties::from_state_id(state.id, &Block::REPEATER);

                return repeater_props.facing == direction.to_horizontal_facing()
                    || repeater_props.facing == direction.opposite().to_horizontal_facing();
            }
        } else if block == Block::OBSERVER {
            if let Some(direction) = direction {
                let observer_props =
                    ObserverLikeProperties::from_state_id(state.id, &Block::OBSERVER);

                return observer_props.facing == direction.to_facing();
            }
        } else if let Some(pumpkin_block) = server.block_registry.get_pumpkin_block(&block) {
            return pumpkin_block.emits_redstone_power(state).await && direction.is_some();
        }

        false
    }

    fn can_run_on_top(_floor: &BlockState) -> bool {
        // TODO: Implement this
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WireConnectionType {
    Up,
    Side,
    None,
}

impl WireConnectionType {
    fn set_connection(&self, props: &mut RedstoneWireLikeProperties, direction: BlockDirection) {
        match direction {
            BlockDirection::North => {
                props.north = match self {
                    WireConnectionType::Up => NorthWireConnection::Up,
                    WireConnectionType::Side => NorthWireConnection::Side,
                    WireConnectionType::None => NorthWireConnection::None,
                }
            }
            BlockDirection::South => {
                props.south = match self {
                    WireConnectionType::Up => SouthWireConnection::Up,
                    WireConnectionType::Side => SouthWireConnection::Side,
                    WireConnectionType::None => SouthWireConnection::None,
                }
            }
            BlockDirection::East => {
                props.east = match self {
                    WireConnectionType::Up => EastWireConnection::Up,
                    WireConnectionType::Side => EastWireConnection::Side,
                    WireConnectionType::None => EastWireConnection::None,
                }
            }
            BlockDirection::West => {
                props.west = match self {
                    WireConnectionType::Up => WestWireConnection::Up,
                    WireConnectionType::Side => WestWireConnection::Side,
                    WireConnectionType::None => WestWireConnection::None,
                }
            }
            _ => {}
        }
    }

    fn is_connected(&self) -> bool {
        return self != &Self::None;
    }
}

trait WireConnection {
    fn is_connected(&self) -> bool;
    fn as_wire_connection_type(&self) -> WireConnectionType;
}

impl WireConnection for NorthWireConnection {
    fn is_connected(&self) -> bool {
        self != &Self::None
    }

    fn as_wire_connection_type(&self) -> WireConnectionType {
        match self {
            Self::Up => WireConnectionType::Up,
            Self::Side => WireConnectionType::Side,
            Self::None => WireConnectionType::None,
        }
    }
}

impl WireConnection for SouthWireConnection {
    fn is_connected(&self) -> bool {
        self != &Self::None
    }

    fn as_wire_connection_type(&self) -> WireConnectionType {
        match self {
            Self::Up => WireConnectionType::Up,
            Self::Side => WireConnectionType::Side,
            Self::None => WireConnectionType::None,
        }
    }
}
impl WireConnection for EastWireConnection {
    fn is_connected(&self) -> bool {
        self != &Self::None
    }

    fn as_wire_connection_type(&self) -> WireConnectionType {
        match self {
            Self::Up => WireConnectionType::Up,
            Self::Side => WireConnectionType::Side,
            Self::None => WireConnectionType::None,
        }
    }
}
impl WireConnection for WestWireConnection {
    fn is_connected(&self) -> bool {
        self != &Self::None
    }

    fn as_wire_connection_type(&self) -> WireConnectionType {
        match self {
            Self::Up => WireConnectionType::Up,
            Self::Side => WireConnectionType::Side,
            Self::None => WireConnectionType::None,
        }
    }
}

trait RedstoneWireHelper {
    fn get_connection_type(&self, direction: BlockDirection) -> WireConnectionType;
}

impl RedstoneWireHelper for RedstoneWireLikeProperties {
    fn get_connection_type(&self, direction: BlockDirection) -> WireConnectionType {
        match direction {
            BlockDirection::North => self.north.as_wire_connection_type(),
            BlockDirection::South => self.south.as_wire_connection_type(),
            BlockDirection::East => self.east.as_wire_connection_type(),
            BlockDirection::West => self.west.as_wire_connection_type(),
            _ => WireConnectionType::None,
        }
    }
}
