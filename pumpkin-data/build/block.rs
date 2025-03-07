use std::collections::HashSet;

use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro2::{Span, TokenStream};
use pumpkin_util::block_grouper::group_by_common_full_words;
use quote::{ToTokens, format_ident, quote};
use serde::Deserialize;
use syn::{Ident, LitBool, LitInt, LitStr};

#[derive(Deserialize, Clone, Debug)]
pub struct BlockProperty {
    pub name: String,
    pub values: Vec<String>,
}

type EnumRemap = (&'static [&'static str], &'static str);

const BOOL_REMAP: EnumRemap = (&["true", "false"], "Boolean");
const AXIS_REMAP: EnumRemap = (&["x", "y", "z"], "Axis");
const DIRECTION_REMAP: EnumRemap = (
    &["north", "east", "south", "west", "up", "down"],
    "Direction",
);
const REDSTONE_CONNECTION_REMAP: EnumRemap = (&["up", "side", "none"], "RedstoneConnection");
const CARDINAL_DIRECTION_REMAP: EnumRemap =
    (&["north", "east", "south", "west"], "CardinalDirection");
const STAIR_HALF_REMAP: EnumRemap = (&["bottom", "top"], "StairHalf");
const RAIL_SHAPE_REMAP: EnumRemap = (
    &[
        "north_south",
        "east_west",
        "ascending_east",
        "ascending_west",
        "ascending_north",
        "ascending_south",
    ],
    "RailShape",
);
const CHEST_TYPE_REMAP: EnumRemap = (&["single", "left", "right"], "ChestType");
const STRUCTURE_BLOCK_MODE_REMAP: EnumRemap =
    (&["save", "load", "corner", "data"], "StructureBlockMode");

/// This is done cause minecrafts default property system could map the same property key to multiple values depending on the block.
/// And while were at it adding a Boolean enum and some other remaps to make it easier to add traits and work with them globally.
/// For example CardinalDirection is also used when determining player direction.
const PROPERTIES_REMAPS: [EnumRemap; 9] = [
    BOOL_REMAP,
    AXIS_REMAP,
    DIRECTION_REMAP,
    REDSTONE_CONNECTION_REMAP,
    CARDINAL_DIRECTION_REMAP,
    STAIR_HALF_REMAP,
    RAIL_SHAPE_REMAP,
    CHEST_TYPE_REMAP,
    STRUCTURE_BLOCK_MODE_REMAP,
];

fn get_enum_name(props: Vec<String>, fallback: String) -> String {
    let props_set: Vec<&str> = props.iter().map(|s| s.as_str()).collect();

    for (variants, enum_name) in PROPERTIES_REMAPS {
        if props_set.len() == variants.len() && props_set.iter().all(|p| variants.contains(p)) {
            return enum_name.to_string();
        }
    }

    fallback.to_upper_camel_case()
}

fn check_for_prop_duplicates(blocks: &Vec<Block>) {
    let mut unique_props: Vec<(String, Vec<String>)> = Vec::new();
    let mut unique_props_names: Vec<String> = Vec::new();

    for block in blocks {
        for prop in block.properties.clone() {
            if !unique_props
                .iter()
                .any(|(_, props)| props.iter().all(|p| prop.values.contains(p)))
            {
                unique_props.push((prop.name.clone(), prop.values.clone()));
            }
        }
    }

    for (name, values) in unique_props {
        let enum_name = get_enum_name(values, name);
        if !PROPERTIES_REMAPS
            .iter()
            .any(|(_, prop_name)| enum_name == *prop_name)
        {
            unique_props_names.push(enum_name);
        }
    }

    // Check for duplicates in unique_props_names
    let unique_set: HashSet<_> = unique_props_names.iter().collect();
    if unique_props_names.len() != unique_set.len() {
        // Find the duplicates
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();

        for name in &unique_props_names {
            if !seen.insert(name) {
                duplicates.push(name);
            }
        }

        panic!("Duplicate property enum names found: {:?}", duplicates);
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum NormalInvProvider {
    #[serde(rename = "minecraft:uniform")]
    Uniform(UniformIntProvider),
    // TODO: Add more...
}

impl ToTokens for NormalInvProvider {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            NormalInvProvider::Uniform(uniform) => {
                tokens.extend(quote! {
                    NormalInvProvider::Uniform(#uniform)
                });
            }
        }
    }
}
#[derive(Deserialize, Clone, Debug)]
pub struct UniformIntProvider {
    min_inclusive: i32,
    max_inclusive: i32,
}

impl ToTokens for UniformIntProvider {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let min_inclusive = LitInt::new(&self.min_inclusive.to_string(), Span::call_site());
        let max_inclusive = LitInt::new(&self.max_inclusive.to_string(), Span::call_site());

        tokens.extend(quote! {
            UniformIntProvider { min_inclusive: #min_inclusive, max_inclusive: #max_inclusive }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum InvProvider {
    Object(NormalInvProvider),
    Constant(i32),
}
impl ToTokens for InvProvider {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            InvProvider::Object(inv_provider) => {
                tokens.extend(quote! {
                    InvProvider::Object(#inv_provider)
                });
            }
            InvProvider::Constant(i) => tokens.extend(quote! {
                InvProvider::Constant(#i)
            }),
        }
    }
}
#[derive(Deserialize, Clone, Debug)]
pub struct Experience {
    pub experience: InvProvider,
}

impl ToTokens for Experience {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let experience = self.experience.to_token_stream();

        tokens.extend(quote! {
            Experience { experience: #experience }
        });
    }
}
#[derive(Deserialize, Clone, Debug)]
pub struct PropertyStruct {
    pub name: String,
    pub values: Vec<String>,
}

impl ToTokens for PropertyStruct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = Ident::new(&self.name, Span::call_site());
        let mut prefix = "";

        if self.values.iter().any(|value| value == "1") {
            prefix = "L";
        }

        let variant_count = self.values.clone().len() as u16;
        let values_index = (0..self.values.clone().len() as u16).collect::<Vec<_>>();

        let ident_values = self.values.iter().map(|value| {
            Ident::new(
                &(prefix.to_owned() + value).to_upper_camel_case(),
                Span::call_site(),
            )
        });

        let values_2 = ident_values.clone();
        let values_3 = ident_values.clone();

        let from_values = self.values.iter().map(|value| {
            let ident = Ident::new(
                &(prefix.to_owned() + value).to_upper_camel_case(),
                Span::call_site(),
            );
            quote! {
                #value => Self::#ident
            }
        });
        let to_values = self.values.iter().map(|value| {
            let ident = Ident::new(
                &(prefix.to_owned() + value).to_upper_camel_case(),
                Span::call_site(),
            );
            quote! {
                Self::#ident => #value
            }
        });

        tokens.extend(quote! {
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub enum #name {
                #(#ident_values),*
            }

            impl EnumVariants for #name {
                fn variant_count() -> u16 {
                    #variant_count
                }

                fn to_index(&self) -> u16 {
                    match self {
                        #(Self::#values_2 => #values_index),*
                    }
                }

                fn from_index(index: u16) -> Self {
                    match index {
                        #(#values_index => Self::#values_3,)*
                        _ => panic!("Invalid index: {}", index),
                    }
                }

                fn to_value(&self) -> &str {
                    match self {
                        #(#to_values),*
                    }
                }

                fn from_value(value: &str) -> Self {
                    match value {
                        #(#from_values),*,
                        _ => panic!("Invalid value: {:?}", value),
                    }
                }

            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct BlockPropertyStruct {
    pub generic_name: String,
    pub entries: Vec<(String, String)>,
}

impl ToTokens for BlockPropertyStruct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = Ident::new(
            &(self.generic_name.clone() + "_block_props").to_upper_camel_case(),
            Span::call_site(),
        );

        let mut entries = self.entries.clone();
        entries.reverse();

        let values = entries.iter().map(|(key, value)| {
            let key = Ident::new_raw(&key.to_owned(), Span::call_site());
            let value = Ident::new(value, Span::call_site());

            quote! {
                #key: #value
            }
        });

        let field_names: Vec<_> = entries
            .iter()
            .map(|(key, _)| Ident::new_raw(key, Span::call_site()))
            .collect();

        let field_types: Vec<_> = entries
            .iter()
            .map(|(_, ty)| Ident::new(ty, Span::call_site()))
            .collect();

        let to_props_values = entries.iter().map(|(key, _value)| {
            let key2 = Ident::new_raw(&key.to_owned(), Span::call_site());

            quote! {
                props.push((#key.to_string(), self.#key2.to_value().to_string()));
            }
        });

        let from_props_values = entries.iter().map(|(key, value)| {
            let key2 = Ident::new_raw(&key.to_owned(), Span::call_site());
            let value = Ident::new(value, Span::call_site());

            quote! {
                #key => block_props.#key2 = #value::from_value(&value)
            }
        });

        tokens.extend(quote! {
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub struct #name {
                #(pub #values),*
            }

            impl BlockProperties for #name {
                #[allow(unused_assignments)]
                fn to_index(&self) -> u16 {
                    let mut index = 0;
                    let mut multiplier = 1;

                    #(
                        index += self.#field_names.to_index() * multiplier;
                        multiplier *= #field_types::variant_count();
                    )*

                    index
                }

                #[allow(unused_assignments)]
                fn from_index(mut index: u16) -> Self {
                    Self {
                        #(
                            #field_names: {
                                let value = index % #field_types::variant_count();
                                index /= #field_types::variant_count();
                                #field_types::from_index(value)
                            }
                        ),*
                    }
                }

                fn to_state_id(&self, block: &Block) -> u16 {
                    block.states[0].id + self.to_index()
                }

                fn from_state_id(state_id: u16, block: &Block) -> Option<Self> {
                    if state_id >= block.states[0].id && state_id <= block.states.last().unwrap().id {
                        let index = state_id - block.states[0].id;
                        Some(Self::from_index(index))
                    } else {
                        None
                    }
                }

                fn default(block: &Block) -> Self {
                    Self::from_state_id(block.default_state_id, block).unwrap()
                }

                #[allow(clippy::vec_init_then_push)]
                fn to_props(&self) -> Vec<(String, String)> {
                    let mut props = vec![];

                    #(#to_props_values)*

                    props
                }

                fn from_props(props: Vec<(String, String)>, block: &Block) -> Self {
                    let mut block_props = Self::default(block);

                    for (key, value) in props {
                        match key.as_str() {
                            #(#from_props_values),*,
                            _ => panic!("Invalid key: {}", key),
                        }
                    }

                    block_props
                }
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct CollisionShape {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl ToTokens for CollisionShape {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let min_x = &self.min[0];
        let min_y = &self.min[1];
        let min_z = &self.min[2];

        let max_x = &self.max[0];
        let max_y = &self.max[1];
        let max_z = &self.max[2];

        tokens.extend(quote! {
            CollisionShape {
                min: [#min_x, #min_y, #min_z],
                max: [#max_x, #max_y, #max_z],
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct BlockState {
    pub id: u16,
    pub air: bool,
    pub luminance: u8,
    pub burnable: bool,
    pub tool_required: bool,
    pub hardness: f32,
    pub sided_transparency: bool,
    pub replaceable: bool,
    pub collision_shapes: Vec<u16>,
    pub opacity: Option<u32>,
    pub block_entity_type: Option<u32>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BlockStateRef {
    pub id: u16,
    pub state_idx: u16,
}

impl ToTokens for BlockState {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        //let id = LitInt::new(&self.id.to_string(), Span::call_site());
        let air = LitBool::new(self.air, Span::call_site());
        let luminance = LitInt::new(&self.luminance.to_string(), Span::call_site());
        let burnable = LitBool::new(self.burnable, Span::call_site());
        let tool_required = LitBool::new(self.tool_required, Span::call_site());
        let hardness = self.hardness;
        let sided_transparency = LitBool::new(self.sided_transparency, Span::call_site());
        let replaceable = LitBool::new(self.replaceable, Span::call_site());
        let opacity = match self.opacity {
            Some(opacity) => {
                let opacity = LitInt::new(&opacity.to_string(), Span::call_site());
                quote! { Some(#opacity) }
            }
            None => quote! { None },
        };
        let block_entity_type = match self.block_entity_type {
            Some(block_entity_type) => {
                let block_entity_type =
                    LitInt::new(&block_entity_type.to_string(), Span::call_site());
                quote! { Some(#block_entity_type) }
            }
            None => quote! { None },
        };

        let collision_shapes = self
            .collision_shapes
            .iter()
            .map(|shape_id| LitInt::new(&shape_id.to_string(), Span::call_site()));

        tokens.extend(quote! {
            PartialBlockState {
                air: #air,
                luminance: #luminance,
                burnable: #burnable,
                tool_required: #tool_required,
                hardness: #hardness,
                sided_transparency: #sided_transparency,
                replaceable: #replaceable,
                collision_shapes: &[#(#collision_shapes),*],
                opacity: #opacity,
                block_entity_type: #block_entity_type,
            }
        });
    }
}

impl ToTokens for BlockStateRef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let id = LitInt::new(&self.id.to_string(), Span::call_site());
        let state_idx = LitInt::new(&self.state_idx.to_string(), Span::call_site());

        tokens.extend(quote! {
            BlockStateRef {
                id: #id,
                state_idx: #state_idx,
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct LootTable {
    r#type: LootTableType,
    random_sequence: Option<String>,
    pools: Option<Vec<LootPool>>,
}

impl ToTokens for LootTable {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let loot_table_type = self.r#type.to_token_stream();
        let random_sequence = match &self.random_sequence {
            Some(seq) => quote! { Some(#seq) },
            None => quote! { None },
        };
        let pools = match &self.pools {
            Some(pools) => {
                let pool_tokens: Vec<_> = pools.iter().map(|pool| pool.to_token_stream()).collect();
                quote! { Some(&[#(#pool_tokens),*]) }
            }
            None => quote! { None },
        };

        tokens.extend(quote! {
            LootTable {
                r#type: #loot_table_type,
                random_sequence: #random_sequence,
                pools: #pools,
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct LootPool {
    entries: Vec<LootPoolEntry>,
    rolls: f32, // TODO
    bonus_rolls: f32,
}

impl ToTokens for LootPool {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let entries_tokens: Vec<_> = self
            .entries
            .iter()
            .map(|entry| entry.to_token_stream())
            .collect();
        let rolls = &self.rolls;
        let bonus_rolls = &self.bonus_rolls;

        tokens.extend(quote! {
            LootPool {
                entries: &[#(#entries_tokens),*],
                rolls: #rolls,
                bonus_rolls: #bonus_rolls,
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ItemEntry {
    name: String,
}

impl ToTokens for ItemEntry {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = LitStr::new(&self.name, Span::call_site());

        tokens.extend(quote! {
            ItemEntry {
                name: #name,
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct AlternativeEntry {
    children: Vec<LootPoolEntry>,
}

impl ToTokens for AlternativeEntry {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let children = self.children.iter().map(|entry| entry.to_token_stream());

        tokens.extend(quote! {
            AlternativeEntry {
                children: &[#(#children),*],
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum LootPoolEntryTypes {
    #[serde(rename = "minecraft:empty")]
    Empty,
    #[serde(rename = "minecraft:item")]
    Item(ItemEntry),
    #[serde(rename = "minecraft:loot_table")]
    LootTable,
    #[serde(rename = "minecraft:dynamic")]
    Dynamic,
    #[serde(rename = "minecraft:tag")]
    Tag,
    #[serde(rename = "minecraft:alternatives")]
    Alternatives(AlternativeEntry),
    #[serde(rename = "minecraft:sequence")]
    Sequence,
    #[serde(rename = "minecraft:group")]
    Group,
}

impl ToTokens for LootPoolEntryTypes {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            LootPoolEntryTypes::Empty => {
                tokens.extend(quote! { LootPoolEntryTypes::Empty });
            }
            LootPoolEntryTypes::Item(item) => {
                tokens.extend(quote! { LootPoolEntryTypes::Item(#item) });
            }
            LootPoolEntryTypes::LootTable => {
                tokens.extend(quote! { LootPoolEntryTypes::LootTable });
            }
            LootPoolEntryTypes::Dynamic => {
                tokens.extend(quote! { LootPoolEntryTypes::Dynamic });
            }
            LootPoolEntryTypes::Tag => {
                tokens.extend(quote! { LootPoolEntryTypes::Tag });
            }
            LootPoolEntryTypes::Alternatives(alt) => {
                tokens.extend(quote! { LootPoolEntryTypes::Alternatives(#alt) });
            }
            LootPoolEntryTypes::Sequence => {
                tokens.extend(quote! { LootPoolEntryTypes::Sequence });
            }
            LootPoolEntryTypes::Group => {
                tokens.extend(quote! { LootPoolEntryTypes::Group });
            }
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "condition")]
pub enum LootCondition {
    #[serde(rename = "minecraft:inverted")]
    Inverted,
    #[serde(rename = "minecraft:any_of")]
    AnyOf,
    #[serde(rename = "minecraft:all_of")]
    AllOf,
    #[serde(rename = "minecraft:random_chance")]
    RandomChance,
    #[serde(rename = "minecraft:random_chance_with_enchanted_bonus")]
    RandomChanceWithEnchantedBonus,
    #[serde(rename = "minecraft:entity_properties")]
    EntityProperties,
    #[serde(rename = "minecraft:killed_by_player")]
    KilledByPlayer,
    #[serde(rename = "minecraft:entity_scores")]
    EntityScores,
    #[serde(rename = "minecraft:block_state_property")]
    BlockStateProperty,
    #[serde(rename = "minecraft:match_tool")]
    MatchTool,
    #[serde(rename = "minecraft:table_bonus")]
    TableBonus,
    #[serde(rename = "minecraft:survives_explosion")]
    SurvivesExplosion,
    #[serde(rename = "minecraft:damage_source_properties")]
    DamageSourceProperties,
    #[serde(rename = "minecraft:location_check")]
    LocationCheck,
    #[serde(rename = "minecraft:weather_check")]
    WeatherCheck,
    #[serde(rename = "minecraft:reference")]
    Reference,
    #[serde(rename = "minecraft:time_check")]
    TimeCheck,
    #[serde(rename = "minecraft:value_check")]
    ValueCheck,
    #[serde(rename = "minecraft:enchantment_active_check")]
    EnchantmentActiveCheck,
}

impl ToTokens for LootCondition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = match self {
            LootCondition::Inverted => quote! { LootCondition::Inverted },
            LootCondition::AnyOf => quote! { LootCondition::AnyOf },
            LootCondition::AllOf => quote! { LootCondition::AllOf },
            LootCondition::RandomChance => quote! { LootCondition::RandomChance },
            LootCondition::RandomChanceWithEnchantedBonus => {
                quote! { LootCondition::RandomChanceWithEnchantedBonus }
            }
            LootCondition::EntityProperties => quote! { LootCondition::EntityProperties },
            LootCondition::KilledByPlayer => quote! { LootCondition::KilledByPlayer },
            LootCondition::EntityScores => quote! { LootCondition::EntityScores },
            LootCondition::BlockStateProperty => quote! { LootCondition::BlockStateProperty },
            LootCondition::MatchTool => quote! { LootCondition::MatchTool },
            LootCondition::TableBonus => quote! { LootCondition::TableBonus },
            LootCondition::SurvivesExplosion => quote! { LootCondition::SurvivesExplosion },
            LootCondition::DamageSourceProperties => {
                quote! { LootCondition::DamageSourceProperties }
            }
            LootCondition::LocationCheck => quote! { LootCondition::LocationCheck },
            LootCondition::WeatherCheck => quote! { LootCondition::WeatherCheck },
            LootCondition::Reference => quote! { LootCondition::Reference },
            LootCondition::TimeCheck => quote! { LootCondition::TimeCheck },
            LootCondition::ValueCheck => quote! { LootCondition::ValueCheck },
            LootCondition::EnchantmentActiveCheck => {
                quote! { LootCondition::EnchantmentActiveCheck }
            }
        };

        tokens.extend(name);
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct LootPoolEntry {
    #[serde(flatten)]
    content: LootPoolEntryTypes,
    conditions: Option<Vec<LootCondition>>,
}

impl ToTokens for LootPoolEntry {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let content = &self.content;
        let conditions_tokens = match &self.conditions {
            Some(conds) => {
                let cond_tokens: Vec<_> = conds.iter().map(|c| c.to_token_stream()).collect();
                quote! { Some(&[#(#cond_tokens),*]) }
            }
            None => quote! { None },
        };

        tokens.extend(quote! {
            LootPoolEntry {
                content: #content,
                conditions: #conditions_tokens,
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename = "snake_case")]
pub enum LootTableType {
    #[serde(rename = "minecraft:empty")]
    /// Nothing will be dropped
    Empty,
    #[serde(rename = "minecraft:block")]
    /// A Block will be dropped
    Block,
    #[serde(rename = "minecraft:chest")]
    /// A Item will be dropped
    Chest,
}

impl ToTokens for LootTableType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = match self {
            LootTableType::Empty => quote! { LootTableType::Empty },
            LootTableType::Block => quote! { LootTableType::Block },
            LootTableType::Chest => quote! { LootTableType::Chest },
        };

        tokens.extend(name);
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Block {
    pub id: u16,
    pub name: String,
    pub translation_key: String,
    pub hardness: f32,
    pub blast_resistance: f32,
    pub item_id: u16,
    pub loot_table: Option<LootTable>,
    pub slipperiness: f32,
    pub velocity_multiplier: f32,
    pub jump_velocity_multiplier: f32,
    pub properties: Vec<BlockProperty>,
    pub default_state_id: u16,
    pub states: Vec<BlockState>,
    pub experience: Option<Experience>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OptimizedBlock {
    pub id: u16,
    pub name: String,
    pub translation_key: String,
    pub hardness: f32,
    pub blast_resistance: f32,
    pub item_id: u16,
    pub loot_table: Option<LootTable>,
    pub slipperiness: f32,
    pub velocity_multiplier: f32,
    pub jump_velocity_multiplier: f32,
    pub default_state_id: u16,
    pub states: Vec<BlockStateRef>,
    pub experience: Option<Experience>,
}

impl ToTokens for OptimizedBlock {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let id = LitInt::new(&self.id.to_string(), Span::call_site());
        let name = LitStr::new(&self.name, Span::call_site());
        let translation_key = LitStr::new(&self.translation_key, Span::call_site());
        let hardness = &self.hardness;
        let blast_resistance = &self.blast_resistance;
        let item_id = LitInt::new(&self.item_id.to_string(), Span::call_site());
        let default_state_id = LitInt::new(&self.default_state_id.to_string(), Span::call_site());
        let slipperiness = &self.slipperiness;
        let velocity_multiplier = &self.velocity_multiplier;
        let jump_velocity_multiplier = &self.jump_velocity_multiplier;
        let experience = match &self.experience {
            Some(exp) => {
                let exp_tokens = exp.to_token_stream();
                quote! { Some(#exp_tokens) }
            }
            None => quote! { None },
        };
        // Generate state tokens
        let states = self.states.iter().map(|state| state.to_token_stream());
        let loot_table = match &self.loot_table {
            Some(table) => {
                let table_tokens = table.to_token_stream();
                quote! { Some(#table_tokens) }
            }
            None => quote! { None },
        };

        tokens.extend(quote! {
            Block {
                id: #id,
                name: #name,
                translation_key: #translation_key,
                hardness: #hardness,
                blast_resistance: #blast_resistance,
                slipperiness: #slipperiness,
                velocity_multiplier: #velocity_multiplier,
                jump_velocity_multiplier: #jump_velocity_multiplier,
                item_id: #item_id,
                default_state_id: #default_state_id,
                states: &[#(#states),*],
                loot_table: #loot_table,
                experience: #experience,
            }
        });
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct BlockAssets {
    pub blocks: Vec<Block>,
    pub shapes: Vec<CollisionShape>,
    pub block_entity_types: Vec<String>,
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=../assets/blocks.json");

    let blocks_assets: BlockAssets = serde_json::from_str(include_str!("../../assets/blocks.json"))
        .expect("Failed to parse blocks.json");

    check_for_prop_duplicates(&blocks_assets.blocks);

    let mut type_from_raw_id_arms = TokenStream::new();
    let mut type_from_name = TokenStream::new();
    let mut block_from_state_id = TokenStream::new();
    let mut block_from_item_id = TokenStream::new();
    let mut block_properties_from_state_and_name = TokenStream::new();
    let mut block_properties_from_props_and_name = TokenStream::new();
    let mut existing_item_ids: Vec<u16> = Vec::new();
    let mut constants = TokenStream::new();

    let mut unique_states = Vec::new();
    for block in blocks_assets.blocks.clone() {
        for state in block.states.clone() {
            // Check if this state is already in unique_states by comparing all fields except id
            let already_exists = unique_states.iter().any(|s: &BlockState| {
                s.air == state.air
                    && s.luminance == state.luminance
                    && s.burnable == state.burnable
                    && s.tool_required == state.tool_required
                    && s.hardness == state.hardness
                    && s.sided_transparency == state.sided_transparency
                    && s.replaceable == state.replaceable
                    && s.collision_shapes == state.collision_shapes
            });

            if !already_exists {
                unique_states.push(state);
            }
        }
    }

    let mut block_props: Vec<BlockPropertyStruct> = Vec::new();
    let mut properties: Vec<PropertyStruct> = Vec::new();
    let mut optimized_blocks: Vec<(String, OptimizedBlock)> = Vec::new();
    #[expect(clippy::type_complexity)]
    let mut shared_props: Vec<(Vec<(String, String)>, Vec<String>)> = Vec::new();

    for block in blocks_assets.blocks.clone() {
        let optimized_block = OptimizedBlock {
            id: block.id,
            name: block.name.clone(),
            translation_key: block.translation_key.clone(),
            hardness: block.hardness,
            blast_resistance: block.blast_resistance,
            item_id: block.item_id,
            default_state_id: block.default_state_id,
            slipperiness: block.slipperiness,
            velocity_multiplier: block.velocity_multiplier,
            jump_velocity_multiplier: block.jump_velocity_multiplier,
            loot_table: block.loot_table,
            experience: block.experience,
            states: block
                .states
                .iter()
                .map(|state| {
                    // Find the index in unique_states by comparing all fields except id
                    let state_idx = unique_states
                        .iter()
                        .position(|s| {
                            s.air == state.air
                                && s.luminance == state.luminance
                                && s.burnable == state.burnable
                                && s.tool_required == state.tool_required
                                && s.hardness == state.hardness
                                && s.sided_transparency == state.sided_transparency
                                && s.replaceable == state.replaceable
                                && s.collision_shapes == state.collision_shapes
                        })
                        .unwrap() as u16;

                    BlockStateRef {
                        id: state.id,
                        state_idx,
                    }
                })
                .collect(),
        };

        optimized_blocks.push((block.name.clone(), optimized_block));

        // Process properties
        if !block.properties.is_empty() {
            let entries: Vec<(String, String)> = block
                .properties
                .iter()
                .map(|prop| {
                    (
                        prop.name.clone(),
                        get_enum_name(prop.values.clone(), prop.name.clone()),
                    )
                })
                .collect();
            if shared_props.iter().any(|(props, _)| props == &entries) {
                shared_props
                    .iter_mut()
                    .find(|(props, _)| props == &entries)
                    .unwrap()
                    .1
                    .push(block.name.clone());
            } else {
                shared_props.push((entries, vec![block.name.clone()]));
            }
        }

        // Add unique property types
        for prop in block.properties {
            let enum_name = get_enum_name(prop.values.clone(), prop.name.clone());

            if !properties.iter().any(|p| p.name == enum_name) {
                properties.push(PropertyStruct {
                    name: enum_name.clone(),
                    values: prop.values,
                });
            }
        }
    }

    let props_grouped_by_props = shared_props
        .iter()
        .map(|(_, blocks)| blocks.clone())
        .collect::<Vec<_>>();

    let grouped_prop_names = group_by_common_full_words(props_grouped_by_props);

    for (name, group_blocks) in grouped_prop_names {
        block_props.push(BlockPropertyStruct {
            generic_name: name.clone(),
            entries: shared_props
                .iter()
                .find(|(_, blocks)| blocks.contains(&group_blocks[0]))
                .unwrap()
                .0
                .clone(),
        });

        for block in group_blocks {
            let block_name = Ident::new(&block.to_shouty_snake_case(), Span::call_site());
            let name = Ident::new(
                &(name.clone() + "_block_props").to_upper_camel_case(),
                Span::call_site(),
            );
            block_properties_from_state_and_name.extend(quote! {
                #block => Some(Box::new(#name::from_state_id(state_id, &Block::#block_name).unwrap())),
            });

            block_properties_from_props_and_name.extend(quote! {
                #block => Some(Box::new(#name::from_props(props, &Block::#block_name))),
            });
        }
    }

    // Generate collision shapes array
    let shapes = blocks_assets
        .shapes
        .iter()
        .map(|shape| shape.to_token_stream());

    let unique_states = unique_states.iter().map(|state| state.to_token_stream());

    let block_props = block_props.iter().map(|prop| prop.to_token_stream());
    let properties = properties.iter().map(|prop| prop.to_token_stream());

    // Generate block entity types array
    let block_entity_types = blocks_assets
        .block_entity_types
        .iter()
        .map(|entity_type| LitStr::new(entity_type, Span::call_site()));

    // Generate constants and match arms for each block
    for (name, block) in optimized_blocks {
        let const_ident = format_ident!("{}", name.to_shouty_snake_case());
        let block_tokens = block.to_token_stream();
        let id_lit = LitInt::new(&block.id.to_string(), Span::call_site());
        let state_start = block.states.iter().map(|state| state.id).min().unwrap();
        let state_end = block.states.iter().map(|state| state.id).max().unwrap();
        let item_id = block.item_id;

        constants.extend(quote! {
            pub const #const_ident: Block = #block_tokens;
        });

        type_from_raw_id_arms.extend(quote! {
            #id_lit => Some(Self::#const_ident),
        });

        type_from_name.extend(quote! {
            #name => Some(Self::#const_ident),
        });

        block_from_state_id.extend(quote! {
            #state_start..=#state_end => Some(Self::#const_ident),
        });

        if !existing_item_ids.contains(&item_id) {
            block_from_item_id.extend(quote! {
                #item_id => Some(Self::#const_ident),
            });
            existing_item_ids.push(item_id);
        }
    }

    quote! {
        use crate::{tag::{Tagable, RegistryKey}, item::Item};
        use pumpkin_util::math::int_provider::{UniformIntProvider, InvProvider, NormalInvProvider};



        #[derive(Clone, Debug)]
        pub struct Experience {
            pub experience: InvProvider,
        }

        #[derive(Clone, Debug)]
        pub struct PartialBlockState {
            pub air: bool,
            pub luminance: u8,
            pub burnable: bool,
            pub tool_required: bool,
            pub hardness: f32,
            pub sided_transparency: bool,
            pub replaceable: bool,
            pub collision_shapes: &'static [u16],
            pub opacity: Option<u32>,
            pub block_entity_type: Option<u32>,
        }

        #[derive(Clone, Debug)]
        pub struct BlockState {
            pub id: u16,
            pub air: bool,
            pub luminance: u8,
            pub burnable: bool,
            pub tool_required: bool,
            pub hardness: f32,
            pub sided_transparency: bool,
            pub replaceable: bool,
            pub collision_shapes: &'static [u16],
            pub opacity: Option<u32>,
            pub block_entity_type: Option<u32>,
        }

        #[derive(Clone, Debug)]
        pub struct BlockStateRef {
            pub id: u16,
            pub state_idx: u16,
        }

        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        pub struct LootTable {
            r#type: LootTableType,
            random_sequence: Option<&'static str>,
            pools: Option<&'static [LootPool]>,
        }

        impl LootTable {
            pub fn get_loot(&self) -> Vec<(Item, u16)> {
                let mut items = vec![];
                if let Some(pools) = &self.pools {
                    for i in 0..pools.len() {
                        let pool = &pools[i];
                        items.extend_from_slice(&pool.get_loot());
                    }
                }
                items
            }
        }

        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        pub struct LootPool {
            entries: &'static [LootPoolEntry],
            rolls: f32, // TODO
            bonus_rolls: f32,
        }

        impl LootPool {
            pub fn get_loot(&self) -> Vec<(Item, u16)> {
                let i = self.rolls.round() as i32 + self.bonus_rolls.floor() as i32; // TODO: mul by luck
                let mut items = vec![];
                for _ in 0..i {
                    for entry_idx in 0..self.entries.len() {
                        let entry = &self.entries[entry_idx];
                        if let Some(conditions) = &entry.conditions {
                            if !conditions.iter().all(|condition| condition.test()) {
                                continue;
                            }
                        }
                        items.extend_from_slice(&entry.content.get_items());
                    }
                }
                items
            }
        }

        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        pub struct ItemEntry {
            name: &'static str,
        }

        impl ItemEntry {
            pub fn get_items(&self) -> Vec<(Item, u16)> {
                let item = Item::from_registry_key(&self.name.replace("minecraft:", "")).unwrap();
                vec![(item, 1)]
            }
        }

        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        pub struct AlternativeEntry {
            children: &'static [LootPoolEntry],
        }
        impl AlternativeEntry {
            pub fn get_items(&self) -> Vec<(Item, u16)> {
                let mut items = vec![];
                for i in 0..self.children.len() {
                    let child = &self.children[i];
                    if let Some(conditions) = &child.conditions {
                        if !conditions.iter().all(|condition| condition.test()) {
                            continue;
                        }
                    }
                    items.extend_from_slice(&child.content.get_items());
                }
                items
            }
        }


        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        pub enum LootPoolEntryTypes {
            Empty,
            Item(ItemEntry),
            LootTable,
            Dynamic,
            Tag,
            Alternatives(AlternativeEntry),
            Sequence,
            Group,
        }

        impl LootPoolEntryTypes {
            pub fn get_items(&self) -> Vec<(Item, u16)> {
                match self {
                    LootPoolEntryTypes::Empty => todo!(),
                    LootPoolEntryTypes::Item(item_entry) => item_entry.get_items(),
                    LootPoolEntryTypes::LootTable => todo!(),
                    LootPoolEntryTypes::Dynamic => todo!(),
                    LootPoolEntryTypes::Tag => todo!(),
                    LootPoolEntryTypes::Alternatives(alternative) => alternative.get_items(),
                    LootPoolEntryTypes::Sequence => todo!(),
                    LootPoolEntryTypes::Group => todo!(),
                }
            }
        }

        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        pub enum LootCondition {
            Inverted,
            AnyOf,
            AllOf,
            RandomChance,
            RandomChanceWithEnchantedBonus,
            EntityProperties,
            KilledByPlayer,
            EntityScores,
            BlockStateProperty,
            MatchTool,
            TableBonus,
            SurvivesExplosion,
            DamageSourceProperties,
            LocationCheck,
            WeatherCheck,
            Reference,
            TimeCheck,
            ValueCheck,
            EnchantmentActiveCheck,
        }

        #[expect(clippy::match_like_matches_macro)]
        impl LootCondition {
            // TODO: This is trash, Make this right
            pub fn test(&self) -> bool {
                match self {
                    LootCondition::SurvivesExplosion => true,
                    _ => false,
                }
            }
        }

        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        pub struct LootPoolEntry {
            content: LootPoolEntryTypes,
            conditions: Option<&'static [LootCondition]>,
        }

        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        pub enum LootTableType {
            /// Nothing will be dropped
            Empty,
            /// A Block will be dropped
            Block,
            /// A Item will be dropped
            Chest,
        }

        #[derive(Clone, Debug)]
        pub struct Block {
            pub id: u16,
            pub name: &'static str,
            pub translation_key: &'static str,
            pub hardness: f32,
            pub blast_resistance: f32,
            pub slipperiness: f32,
            pub velocity_multiplier: f32,
            pub jump_velocity_multiplier: f32,
            pub item_id: u16,
            pub default_state_id: u16,
            pub states: &'static [BlockStateRef],
            pub loot_table: Option<LootTable>,
            pub experience: Option<Experience>,
        }

        impl PartialEq for Block {
            fn eq(&self, other: &Self) -> bool {
                self.id == other.id
            }
        }

        #[derive(Clone, Copy, Debug)]
        pub struct BlockProperty {
            pub name: &'static str,
            pub values: &'static [&'static str],
        }

        #[derive(Clone, Copy, Debug)]
        pub struct CollisionShape {
            pub min: [f64; 3],
            pub max: [f64; 3],
        }

        #[derive(Clone, Copy, Debug)]
        pub struct BlockStateData {
            pub air: bool,
            pub luminance: u8,
            pub burnable: bool,
            pub tool_required: bool,
            pub hardness: f32,
            pub sided_transparency: bool,
            pub replaceable: bool,
            pub collision_shapes: &'static [u16],
            pub opacity: Option<u32>,
            pub block_entity_type: Option<u32>,
        }


        pub trait BlockProperties where Self: 'static {
            // Convert properties to an index (0 to N-1)
            fn to_index(&self) -> u16;
            // Convert an index back to properties
            fn from_index(index: u16) -> Self where Self: Sized;

            // Convert properties to a state id
            fn to_state_id(&self, block: &Block) -> u16;
            // Convert a state id back to properties
            fn from_state_id(state_id: u16, block: &Block) -> Option<Self> where Self: Sized;
            // Get the default properties
            fn default(block: &Block) -> Self where Self: Sized;

            // Convert properties to a vec of (name, value)
            fn to_props(&self) -> Vec<(String, String)>;

            // Convert properties to a block state, add them onto the default state
            fn from_props(props: Vec<(String, String)>, block: &Block) -> Self where Self: Sized;
        }

        pub trait EnumVariants {
            fn variant_count() -> u16;
            fn to_index(&self) -> u16;
            fn from_index(index: u16) -> Self;
            fn to_value(&self) -> &str;
            fn from_value(value: &str) -> Self;
        }



        pub static COLLISION_SHAPES: &[CollisionShape] = &[
            #(#shapes),*
        ];

        pub static BLOCK_STATES: &[PartialBlockState] = &[
            #(#unique_states),*
        ];

        pub static BLOCK_ENTITY_TYPES: &[&str] = &[
            #(#block_entity_types),*
        ];



        impl Block {
            #constants

            #[doc = r" Try to parse a Block from a resource location string"]
            pub fn from_registry_key(name: &str) -> Option<Self> {
                match name {
                    #type_from_name
                    _ => None
                }
            }

            #[doc = r" Try to parse a Block from a raw id"]
            pub const fn from_id(id: u16) -> Option<Self> {
                match id {
                    #type_from_raw_id_arms
                    _ => None
                }
            }

            #[doc = r" Try to parse a Block from a state id"]
            pub const fn from_state_id(id: u16) -> Option<Self> {
                match id {
                    #block_from_state_id
                    _ => None
                }
            }

            #[doc = r" Try to parse a Block from an item id"]
            pub const fn from_item_id(id: u16) -> Option<Self> {
                #[allow(unreachable_patterns)]
                match id {
                    #block_from_item_id
                    _ => None
                }
            }

            #[doc = r" Get the properties of the block"]
            pub fn properties(&self, state_id: u16) -> Option<Box<dyn BlockProperties>> {
                match self.name {
                    #block_properties_from_state_and_name
                    _ => None
                }
            }

            #[doc = r" Get the properties of the block"]
            pub fn from_properties(&self, props: Vec<(String, String)>) -> Option<Box<dyn BlockProperties>> {
                match self.name {
                    #block_properties_from_props_and_name
                    _ => None
                }
            }
        }

        #(#properties)*

        #(#block_props)*

        impl BlockStateRef {
            pub fn get_state(&self) -> BlockState {
                let partial_state = &BLOCK_STATES[self.state_idx as usize];
                BlockState {
                    id: self.id,
                    air: partial_state.air,
                    luminance: partial_state.luminance,
                    burnable: partial_state.burnable,
                    tool_required: partial_state.tool_required,
                    hardness: partial_state.hardness,
                    sided_transparency: partial_state.sided_transparency,
                    replaceable: partial_state.replaceable,
                    collision_shapes: partial_state.collision_shapes,
                    opacity: partial_state.opacity,
                    block_entity_type: partial_state.block_entity_type,
                }
            }
        }

        impl Tagable for Block {
            #[inline]
            fn tag_key() -> RegistryKey {
                RegistryKey::Block
            }

            #[inline]
            fn registry_key(&self) -> &str {
                self.name
            }
        }

        impl CardinalDirection {
            pub fn opposite(&self) -> Self {
                match self {
                    CardinalDirection::North => CardinalDirection::South,
                    CardinalDirection::South => CardinalDirection::North,
                    CardinalDirection::East => CardinalDirection::West,
                    CardinalDirection::West => CardinalDirection::East
                }
            }
        }

        impl Boolean {
            pub fn flip(&self) -> Self {
                match self {
                    Boolean::True => Boolean::False,
                    Boolean::False => Boolean::True,
                }
            }

            pub fn to_bool(&self) -> bool {
                match self {
                    Boolean::True => true,
                    Boolean::False => false,
                }
            }

            pub fn from_bool(value: bool) -> Self {
                if value {
                    Boolean::True
                } else {
                    Boolean::False
                }
            }
        }
    }
}
