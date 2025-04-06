use async_trait::async_trait;
use pumpkin_data::item::Item;
use pumpkin_world::item::ItemStack;

// Inventory.java
#[async_trait]
pub trait Inventory: Send + Sync + IntoIterator<Item = ItemStack> + Clone {
    async fn size(&self) -> usize;

    async fn is_empty(&self) -> bool;

    async fn get_stack(&self, slot: usize) -> ItemStack;

    async fn remove_stack_specific(&mut self, slot: usize, amount: u8) -> ItemStack;

    async fn remove_stack(&mut self, slot: usize) -> ItemStack;

    async fn set_stack(&mut self, slot: usize, stack: ItemStack);

    async fn mark_dirty(&mut self);

    /*
    boolean canPlayerUse(PlayerEntity player);

    default void onOpen(PlayerEntity player) {
    }

    default void onClose(PlayerEntity player) {
    }
    */

    /// isValid is source
    async fn is_valid_slot_for(&self, _slot: usize, _stack: &ItemStack) -> bool {
        true
    }

    async fn can_transfer_to<I: Inventory>(
        &self,
        _hopper_inventory: I,
        _slot: usize,
        _stack: &ItemStack,
    ) -> bool {
        true
    }

    fn count(&self, item: &Item) -> u8 {
        let mut count = 0;

        for stack in self.clone().into_iter() {
            if stack.get_item().id == item.id {
                count += stack.item_count;
            }
        }

        count
    }

    fn contains_any_predicate(&self, predicate: impl Fn(&ItemStack) -> bool) -> bool {
        for stack in self.clone().into_iter() {
            if predicate(&stack) {
                return true;
            }
        }

        false
    }

    fn contains_any(&self, items: &[Item]) -> bool {
        self.contains_any_predicate(move |stack| {
            !stack.is_empty() && items.contains(&stack.get_item())
        })
    }

    // TODO: canPlayerUse
}

pub trait Clearable {
    fn clear(&mut self);
}
