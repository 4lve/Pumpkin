use blocks::doors::register_door_blocks;
use blocks::fence_gates::register_fence_gate_blocks;
use blocks::fences::register_fence_blocks;
use blocks::logs::register_log_blocks;
use blocks::{chest::ChestBlock, furnace::FurnaceBlock, lever::LeverBlock, tnt::TNTBlock};
use pumpkin_data::block::{Block, BlockState};
use pumpkin_data::entity::EntityType;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_world::item::ItemStack;
use rand::Rng;

use crate::block::registry::BlockRegistry;
use crate::entity::item::ItemEntity;
use crate::world::World;
use crate::{block::blocks::crafting_table::CraftingTableBlock, entity::player::Player};
use crate::{block::blocks::jukebox::JukeboxBlock, entity::experience_orb::ExperienceOrbEntity};
use std::sync::Arc;

mod blocks;
pub mod pumpkin_block;
pub mod registry;

#[must_use]
pub fn default_registry() -> Arc<BlockRegistry> {
    let mut manager = BlockRegistry::default();

    manager.register(JukeboxBlock);
    manager.register(CraftingTableBlock);
    manager.register(FurnaceBlock);
    manager.register(ChestBlock);
    manager.register(TNTBlock);
    manager.register(LeverBlock);

    register_door_blocks(&mut manager);
    register_fence_blocks(&mut manager);
    register_fence_gate_blocks(&mut manager);
    register_log_blocks(&mut manager);

    Arc::new(manager)
}

pub async fn drop_loot(world: &Arc<World>, block: &Block, pos: &BlockPos, experience: bool) {
    if let Some(table) = &block.loot_table {
        let loot = table.get_loot();
        for (item, count) in loot {
            drop_stack(world, pos, ItemStack::new(count as u8, item)).await;
        }
    }

    if experience {
        if let Some(experience) = &block.experience {
            let amount = experience.experience.get();
            // TODO: Silk touch gives no exp
            if amount > 0 {
                ExperienceOrbEntity::spawn(world, pos.to_f64(), amount as u32).await;
            }
        }
    }
}

#[allow(dead_code)]
async fn drop_stack(world: &Arc<World>, pos: &BlockPos, stack: ItemStack) {
    let height = EntityType::ITEM.dimension[1] / 2.0;
    let pos = Vector3::new(
        f64::from(pos.0.x) + 0.5 + rand::thread_rng().gen_range(-0.25..0.25),
        f64::from(pos.0.y) + 0.5 + rand::thread_rng().gen_range(-0.25..0.25) - f64::from(height),
        f64::from(pos.0.z) + 0.5 + rand::thread_rng().gen_range(-0.25..0.25),
    );

    let entity = world.create_entity(pos, EntityType::ITEM);
    let item_entity =
        Arc::new(ItemEntity::new(entity, stack.item.id, u32::from(stack.item_count)).await);
    world.spawn_entity(item_entity.clone()).await;
    item_entity.send_meta_packet().await;
}

pub async fn calc_block_breaking(player: &Player, state: &BlockState, block_name: &str) -> f32 {
    let hardness = state.hardness;
    #[expect(clippy::float_cmp)]
    if hardness == -1.0 {
        // unbreakable
        return 0.0;
    }
    let i = if player.can_harvest(state, block_name).await {
        30
    } else {
        100
    };

    player.get_mining_speed(block_name).await / hardness / i as f32
}
