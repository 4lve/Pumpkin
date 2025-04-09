use std::sync::Arc;

use async_trait::async_trait;
use pumpkin_world::item::ItemStack;
use tokio::sync::Mutex;

use crate::inventory::Inventory;

// Slot.java
// This is a trait due to crafting slots being a thing
#[async_trait]
pub trait Slot {
    fn new<I: Inventory>(inventory: Arc<Mutex<I>>, index: usize) -> Self;

    fn get_inventory<I: Inventory>(&self) -> &Arc<Mutex<I>>;

    fn get_index(&self) -> usize;

    fn set_index(&mut self, index: usize);

    fn on_quick_transfer(&self, new_item: ItemStack, original: ItemStack) {
        let diff = new_item.item_count - original.item_count;
        if diff > 0 {
            self.on_crafted(original, diff);
        }
    }

    fn on_crafted(&self, _stack: ItemStack, _amount: u8) {}

    fn on_crafted_single(&self, _stack: ItemStack) {}

    fn on_take(&self, _amount: u8) {}

    // TODO: Source takes player as parameter
    fn on_take_item(&self, _stack: &ItemStack) {
        self.mark_dirty();
    }

    fn can_insert(&self, _stack: &ItemStack) -> bool {
        true
    }

    fn get_stack(&self) -> ItemStack;

    fn has_stack(&self) -> bool {
        !self.get_stack().is_empty()
    }

    async fn set_stack<I: Inventory>(&self, stack: ItemStack) {
        self.set_stack_no_callbacks::<I>(stack).await;
    }

    async fn set_stack_prev<I: Inventory>(&self, stack: ItemStack, _previous_stack: ItemStack) {
        self.set_stack_no_callbacks::<I>(stack).await;
    }

    async fn set_stack_no_callbacks<I: Inventory>(&self, stack: ItemStack) {
        let mut inv = self.get_inventory::<I>().lock().await;
        inv.set_stack(self.get_index(), stack);
        drop(inv);
        self.mark_dirty();
    }

    fn mark_dirty(&self);

    fn get_max_item_count<I: Inventory>(&self) -> u8 {
        I::get_max_count_per_stack()
    }

    fn get_max_item_count_for_stack<I: Inventory>(&self, stack: &ItemStack) -> u8 {
        self.get_max_item_count::<I>()
            .min(stack.get_max_stack_size())
    }

    async fn take_stack<I: Inventory>(&self, amount: u8) -> ItemStack {
        let mut inv = self.get_inventory::<I>().lock().await;
        let stack = inv.remove_stack_specific(self.get_index(), amount);
        drop(inv);
        stack
    }

    // TODO: Source takes player as parameter
    fn can_take_items(&self) -> bool {
        true
    }

    fn is_enabled(&self) -> bool {
        true
    }

    async fn try_take_stack_range<I: Inventory>(&self, min: u8, max: u8) -> Option<ItemStack> {
        // TODO: Player is passed in here

        let min = min.min(max);
        let stack = self.take_stack::<I>(min).await;

        if stack.is_empty() {
            None
        } else {
            if self.get_stack().is_empty() {
                self.set_stack_prev::<I>(ItemStack::EMPTY, stack).await;
            }

            Some(stack)
        }
    }

    async fn take_stack_range<I: Inventory>(&self, min: u8, max: u8) -> ItemStack {
        let stack = self.try_take_stack_range::<I>(min, max).await;

        if let Some(stack) = stack {
            self.on_take_item(&stack);
        }

        stack.unwrap_or(ItemStack::EMPTY)
    }

    async fn insert_stack<I: Inventory>(&self, stack: ItemStack) -> ItemStack {
        self.insert_stack_count::<I>(stack, stack.item_count).await
    }

    async fn insert_stack_count<I: Inventory>(&self, mut stack: ItemStack, count: u8) -> ItemStack {
        if !stack.is_empty() && self.can_insert(&stack) {
            let mut stack_self = self.get_stack();
            let min_count = count
                .min(stack.item_count)
                .min(self.get_max_item_count_for_stack::<I>(&stack) - stack_self.item_count);

            if min_count <= 0 {
                return stack;
            } else {
                if stack_self.is_empty() {
                    self.set_stack::<I>(stack.split(min_count)).await;
                } else if stack.are_items_and_components_equal(&stack_self) {
                    stack.decrement(min_count);
                    stack_self.increment(min_count);
                    self.set_stack::<I>(stack_self).await;
                }

                return stack;
            }
        } else {
            stack
        }
    }
}
