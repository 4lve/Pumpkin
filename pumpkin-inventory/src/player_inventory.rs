use crate::container_click::MouseClick;
use crate::crafting::check_if_matches_crafting;
use crate::equipment_slot::EquipmentSlot;
use crate::inventory::{Clearable, Inventory, InventoryIterator};
use crate::{InventoryError, WindowType};
use pumpkin_data::item::Item;
use pumpkin_protocol::client::play::Player;
use pumpkin_world::item::ItemStack;
use std::collections::HashMap;
use std::iter::Chain;
use std::slice::IterMut;

/*
    Inventory Layout:
    - 0: Crafting Output
    - 1-4: Crafting Input
    - 5-8: Armor
    - 9-35: Main Inventory
    - 36-44: Hotbar
    - 45: Offhand

*/

#[derive(Debug, Clone)]
pub struct PlayerInventory {
    pub main_inventory: [ItemStack; Self::MAIN_SIZE],
    pub equipment_slots: HashMap<i32, EquipmentSlot>,
    pub change_count: u32,
    pub selected_slot: usize,
}

impl PlayerInventory {
    const MAIN_SIZE: usize = 36;
    const HOTBAR_SIZE: usize = 9;
    const OFF_HAND_SLOT: usize = 45;

    // TODO: Add inventory load from nbt
    pub fn new() -> Self {
        Self {
            main_inventory: [ItemStack::EMPTY; Self::MAIN_SIZE],
            equipment_slots: Self::build_equipment_slots(),
            change_count: 0,
            selected_slot: 0,
        }
    }

    /// getSelectedStack in source
    pub fn held_item(&self) -> &ItemStack {
        self.main_inventory.get(self.selected_slot).unwrap()
    }

    pub fn is_valid_hotbar_index(slot: usize) -> bool {
        slot <= Self::HOTBAR_SIZE
    }

    fn build_equipment_slots() -> HashMap<i32, EquipmentSlot> {
        let mut equipment_slots = HashMap::new();
        equipment_slots.insert(
            EquipmentSlot::FEET.get_offset_entity_slot_id(Self::MAIN_SIZE as i32),
            EquipmentSlot::FEET,
        );
        equipment_slots.insert(
            EquipmentSlot::LEGS.get_offset_entity_slot_id(Self::MAIN_SIZE as i32),
            EquipmentSlot::LEGS,
        );
        equipment_slots.insert(
            EquipmentSlot::CHEST.get_offset_entity_slot_id(Self::MAIN_SIZE as i32),
            EquipmentSlot::CHEST,
        );
        equipment_slots.insert(
            EquipmentSlot::HEAD.get_offset_entity_slot_id(Self::MAIN_SIZE as i32),
            EquipmentSlot::HEAD,
        );
        equipment_slots.insert(40, EquipmentSlot::OFF_HAND);
        equipment_slots
    }
}

impl Clearable for PlayerInventory {
    fn clear(&mut self) {
        todo!()
    }
}

impl Inventory for PlayerInventory {
    fn size(&self) -> usize {
        self.main_inventory.len() + self.equipment_slots.len()
    }

    fn is_empty(&self) -> bool {
        for item in self.main_inventory.iter() {
            if !item.is_empty() {
                return false;
            }
        }

        // TODO: Check equipment slots

        true
    }

    fn get_stack(&self, slot: usize) -> ItemStack {
        if slot < self.main_inventory.len() {
            self.main_inventory[slot]
        } else {
            todo!()
        }
    }

    fn remove_stack_specific(&mut self, slot: usize, amount: u8) -> ItemStack {
        todo!()
    }

    fn remove_stack(&mut self, slot: usize) -> ItemStack {
        todo!()
    }

    fn set_stack(&mut self, slot: usize, stack: ItemStack) {
        todo!()
    }

    fn mark_dirty(&mut self) {
        self.change_count += 1;
    }
}

impl IntoIterator for PlayerInventory {
    type Item = ItemStack;
    type IntoIter = InventoryIterator<PlayerInventory>;

    fn into_iter(self) -> Self::IntoIter {
        InventoryIterator::new(self)
    }
}

/*
impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerInventory {
    pub const CONTAINER_ID: i8 = 0;

    pub fn new() -> Self {
        Self {
            crafting: [const { None }; 4],
            crafting_output: None,
            items: [const { None }; 36],
            armor: [const { None }; 4],
            offhand: None,
            // TODO: What happens when a player spawns in with a different index?
            selected_slot: 0,
            state_id: 0,
            total_opened_containers: 2,
        }
    }

    /// Set the contents of an item in a slot.
    ///
    /// ## `item`
    /// The optional item to place in the slot
    ///
    /// ## `item_allowed_override`
    /// An override, which when enabled, makes it so that invalid items can be placed in slots they normally can't.
    /// Useful functionality for plugins in the future.
    pub fn set_slot(
        &mut self,
        slot: usize,
        item: Option<ItemStack>,
        item_allowed_override: bool,
    ) -> Result<(), InventoryError> {
        if item_allowed_override {
            if !(0..=SLOT_MAX).contains(&slot) {
                Err(InventoryError::InvalidSlot)?
            }
            *self.all_slots()[slot] = item;
            return Ok(());
        }
        let slot_condition = self.slot_condition(slot)?;
        if let Some(item) = item {
            if slot_condition(&item) {
                *self.all_slots()[slot] = Some(item);
            }
        }
        Ok(())
    }
    #[allow(clippy::type_complexity)]
    pub fn slot_condition(
        &self,
        slot: usize,
    ) -> Result<Box<dyn Fn(&ItemStack) -> bool>, InventoryError> {
        if !(0..=SLOT_MAX).contains(&slot) {
            return Err(InventoryError::InvalidSlot);
        }

        Ok(Box::new(match slot {
            SLOT_CRAFT_OUTPUT..=SLOT_CRAFT_INPUT_END | SLOT_INV_START..=SLOT_OFFHAND => |_| true,
            SLOT_HELM => |item: &ItemStack| item.is_helmet(),
            SLOT_CHEST => |item: &ItemStack| item.is_chestplate(),
            SLOT_LEG => |item: &ItemStack| item.is_leggings(),
            SLOT_BOOT => |item: &ItemStack| item.is_boots(),
            _ => unreachable!(),
        }))
    }
    pub fn get_slot(&mut self, slot: usize) -> Result<&mut Option<ItemStack>, InventoryError> {
        match slot {
            SLOT_CRAFT_OUTPUT => {
                // TODO: Add crafting check here
                Ok(&mut self.crafting_output)
            }
            SLOT_CRAFT_INPUT_START..=SLOT_CRAFT_INPUT_END => {
                Ok(&mut self.crafting[slot - SLOT_CRAFT_INPUT_START])
            }
            SLOT_HELM..=SLOT_BOOT => Ok(&mut self.armor[slot - SLOT_HELM]),
            SLOT_INV_START..=SLOT_HOTBAR_END => Ok(&mut self.items[slot - SLOT_INV_START]),
            SLOT_OFFHAND => Ok(&mut self.offhand),
            _ => Err(InventoryError::InvalidSlot),
        }
    }
    pub fn set_selected(&mut self, slot: usize) {
        debug_assert!((0..=SLOT_HOTBAR_INDEX).contains(&slot));
        self.selected_slot = slot as u8;
    }

    pub fn get_selected_slot(&self) -> usize {
        self.selected_slot as usize + SLOT_HOTBAR_START
    }

    pub fn increment_state_id(&mut self) {
        self.state_id = self.state_id % 100 + 1;
    }

    pub async fn get_mining_speed(&self, block_name: &str) -> f32 {
        self.held_item()
            .map_or_else(|| 1.0, |e| e.get_speed(block_name))
    }

    // NOTE: We actually want &mut Option instead of Option<&mut>
    pub fn held_item_mut(&mut self) -> &mut Option<ItemStack> {
        debug_assert!((0..=SLOT_HOTBAR_INDEX).contains(&(self.selected_slot as usize)));
        &mut self.items[self.get_selected_slot() - SLOT_INV_START]
    }

    #[inline]
    pub fn held_item(&self) -> Option<&ItemStack> {
        debug_assert!((0..=SLOT_HOTBAR_INDEX).contains(&(self.selected_slot as usize)));
        self.items[self.get_selected_slot() - SLOT_INV_START].as_ref()
    }

    pub fn decrease_current_stack(&mut self, amount: u8) -> bool {
        let held_item = self.held_item_mut();
        if let Some(item_stack) = held_item {
            item_stack.item_count -= amount;
            if item_stack.item_count == 0 {
                *held_item = None;
            }
            return true;
        };
        false
    }

    pub fn get_empty_hotbar_slot(&self) -> usize {
        if self.held_item().is_none() {
            return self.selected_slot as usize;
        }

        for slot in SLOT_HOTBAR_START..=SLOT_HOTBAR_END {
            if self.items[slot - SLOT_INV_START].is_none() {
                return slot - SLOT_HOTBAR_START;
            }
        }

        self.selected_slot as usize
    }

    pub fn get_slot_filtered<F>(&self, filter: &F) -> Option<usize>
    where
        F: Fn(Option<&ItemStack>) -> bool,
    {
        // Check selected slot
        if filter(self.items[self.get_selected_slot() - SLOT_INV_START].as_ref()) {
            Some(self.get_selected_slot())
        }
        // Check hotbar slots (27-35) first
        else if let Some(index) = self.items
            [SLOT_HOTBAR_START - SLOT_INV_START..=SLOT_HOTBAR_END - SLOT_INV_START]
            .iter()
            .enumerate()
            .position(|(index, item_stack)| {
                index != self.selected_slot as usize && filter(item_stack.as_ref())
            })
        {
            Some(index + SLOT_HOTBAR_START)
        }
        // Then check main inventory slots (0-26)
        else if let Some(index) = self.items[0..=SLOT_INV_END - SLOT_INV_START]
            .iter()
            .position(|item_stack| filter(item_stack.as_ref()))
        {
            Some(index + SLOT_INV_START)
        }
        // Check offhand
        else if filter(self.offhand.as_ref()) {
            Some(SLOT_OFFHAND)
        } else {
            None
        }
    }

    pub fn get_nonfull_slot_with_item(&self, item_id: u16) -> Option<usize> {
        let max_stack = Item::from_id(item_id)
            .expect("We passed an invalid item id")
            .components
            .max_stack_size;

        self.get_slot_filtered(&|item_stack| {
            item_stack.is_some_and(|item_stack| {
                item_stack.item.id == item_id && item_stack.item_count < max_stack
            })
        })
    }

    /// Returns a slot that has an item with less than the max stack size. If none, returns an empty
    /// slot. If none, returns `None`.`
    pub fn get_pickup_item_slot(&self, item_id: u16) -> Option<usize> {
        self.get_nonfull_slot_with_item(item_id)
            .or_else(|| self.get_empty_slot())
    }

    pub fn get_slot_with_item(&self, item_id: u16) -> Option<usize> {
        self.get_slot_filtered(&|item_stack| {
            item_stack.is_some_and(|item_stack| item_stack.item.id == item_id)
        })
    }

    pub fn get_empty_slot(&self) -> Option<usize> {
        self.get_slot_filtered(&|item_stack| item_stack.is_none())
    }

    pub fn get_empty_slot_no_order(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|slot| slot.is_none())
            .map(|index| index + SLOT_INV_START)
    }

    pub fn slots(&self) -> Box<[Option<&ItemStack>]> {
        let mut slots = vec![self.crafting_output.as_ref()];
        slots.extend(self.crafting.iter().map(|c| c.as_ref()));
        slots.extend(self.armor.iter().map(|c| c.as_ref()));
        slots.extend(self.items.iter().map(|c| c.as_ref()));
        slots.push(self.offhand.as_ref());
        slots.into_boxed_slice()
    }

    pub fn slots_mut(&mut self) -> Box<[&mut Option<ItemStack>]> {
        let mut slots = vec![&mut self.crafting_output];
        slots.extend(self.crafting.iter_mut());
        slots.extend(self.armor.iter_mut());
        slots.extend(self.items.iter_mut());
        slots.push(&mut self.offhand);
        slots.into_boxed_slice()
    }

    pub fn armor_slots(&self) -> Box<[Option<&ItemStack>]> {
        self.armor.iter().map(|item| item.as_ref()).collect()
    }

    pub fn crafting_slots(&self) -> Box<[Option<&ItemStack>]> {
        let mut slots = vec![self.crafting_output.as_ref()];
        slots.extend(self.crafting.iter().map(|c| c.as_ref()));
        slots.into_boxed_slice()
    }

    pub fn item_slots(&self) -> Box<[Option<&ItemStack>]> {
        self.items.iter().map(|item| item.as_ref()).collect()
    }

    pub fn offhand_slot(&self) -> Option<&ItemStack> {
        self.offhand.as_ref()
    }

    pub fn iter_items_mut(&mut self) -> IterMut<Option<ItemStack>> {
        self.items.iter_mut()
    }

    pub fn slots_with_hotbar_first(
        &mut self,
    ) -> Chain<IterMut<Option<ItemStack>>, IterMut<Option<ItemStack>>> {
        let (items, hotbar) = self.items.split_at_mut(SLOT_HOTBAR_START - SLOT_INV_START);
        hotbar.iter_mut().chain(items)
    }
}

impl Container for PlayerInventory {
    fn window_type(&self) -> &'static WindowType {
        &WindowType::Generic9x1
    }

    fn window_name(&self) -> &'static str {
        // We never send an `OpenContainer` with inventory, so it has no name.
        ""
    }

    fn handle_item_change(
        &mut self,
        carried_slot: &mut Option<ItemStack>,
        slot: usize,
        mouse_click: MouseClick,
        invert: bool,
    ) -> Result<(), InventoryError> {
        let slot_condition = self.slot_condition(slot)?;
        let item_slot = self.get_slot(slot)?;
        if let Some(item) = carried_slot {
            debug_assert!(
                item.item_count > 0,
                "We aren't setting the stack to `None` somewhere"
            );
            if slot_condition(item) {
                if invert {
                    handle_item_change(item_slot, carried_slot, mouse_click);
                } else {
                    handle_item_change(carried_slot, item_slot, mouse_click);
                }
            } else {
                return Err(InventoryError::InvalidSlot);
            }
        } else if invert {
            handle_item_change(item_slot, carried_slot, mouse_click);
        } else {
            handle_item_change(carried_slot, item_slot, mouse_click)
        }
        Ok(())
    }

    fn all_slots(&mut self) -> Box<[&mut Option<ItemStack>]> {
        self.slots_mut()
    }

    fn all_slots_ref(&self) -> Box<[Option<&ItemStack>]> {
        self.slots()
    }

    fn all_combinable_slots(&self) -> Box<[Option<&ItemStack>]> {
        self.items.iter().map(|item| item.as_ref()).collect()
    }

    fn all_combinable_slots_mut(&mut self) -> Box<[&mut Option<ItemStack>]> {
        self.items.iter_mut().collect()
    }

    fn craft(&mut self) -> bool {
        let v1 = [self.crafting[0].as_ref(), self.crafting[1].as_ref(), None];
        let v2 = [self.crafting[2].as_ref(), self.crafting[3].as_ref(), None];
        let v3 = [const { None }; 3];
        let together = [v1, v2, v3];

        self.crafting_output = check_if_matches_crafting(together);
        self.crafting.iter().any(|s| s.is_some())
    }

    fn crafting_output_slot(&self) -> Option<usize> {
        Some(SLOT_CRAFT_OUTPUT)
    }

    fn slot_in_crafting_input_slots(&self, slot: &usize) -> bool {
        (SLOT_CRAFT_INPUT_START..=SLOT_CRAFT_INPUT_END).contains(slot)
    }
}
 */
