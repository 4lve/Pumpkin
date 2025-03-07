use pumpkin_data::chunk::Biome;
use pumpkin_util::math::{vector2::Vector2, vector3::Vector3};

use crate::{
    biome::{BiomeSupplier, MultiNoiseBiomeSupplier},
    block::BlockState,
    generation::{chunk_noise::CHUNK_DIM, positions::chunk_pos},
};

use super::{
    GlobalRandomConfig,
    aquifer_sampler::{FluidLevel, FluidLevelSampler, FluidLevelSamplerImpl},
    biome_coords,
    chunk_noise::{ChunkNoiseGenerator, LAVA_BLOCK, WATER_BLOCK},
    noise_router::{
        multi_noise_sampler::{MultiNoiseSampler, MultiNoiseSamplerBuilderOptions},
        proto_noise_router::GlobalProtoNoiseRouter,
    },
    positions::chunk_pos::{start_block_x, start_block_z},
    section_coords,
    settings::GenerationSettings,
    surface::MaterialRuleContext,
};

pub struct StandardChunkFluidLevelSampler {
    top_fluid: FluidLevel,
    bottom_fluid: FluidLevel,
    bottom_y: i32,
}

impl StandardChunkFluidLevelSampler {
    pub fn new(top_fluid: FluidLevel, bottom_fluid: FluidLevel) -> Self {
        let bottom_y = top_fluid
            .max_y_exclusive()
            .min(bottom_fluid.max_y_exclusive());
        Self {
            top_fluid,
            bottom_fluid,
            bottom_y,
        }
    }
}

impl FluidLevelSamplerImpl for StandardChunkFluidLevelSampler {
    fn get_fluid_level(&self, _x: i32, y: i32, _z: i32) -> FluidLevel {
        if y < self.bottom_y {
            self.bottom_fluid.clone()
        } else {
            self.top_fluid.clone()
        }
    }
}

/// Vanilla Chunk Steps
///
/// 1. empty: The chunk is not yet loaded or generated.
///
/// 2. structures_starts: This step calculates the starting points for structure pieces. For structures that are the starting in this chunk, the position of all pieces are generated and stored.
///
/// 3. structures_references: A reference to nearby chunks that have a structures' starting point are stored.
///
/// 4. biomes: Biomes are determined and stored. No terrain is generated at this stage.
///
/// 5. noise: The base terrain shape and liquid bodies are placed.
///
/// 6. surface: The surface of the terrain is replaced with biome-dependent blocks.
///
/// 7. carvers: Carvers carve certain parts of the terrain and replace solid blocks with air.
///
/// 8. features: Features and structure pieces are placed and heightmaps are generated.
///
/// 9. initialize_light: The lighting engine is initialized and light sources are identified.
///
/// 10. light: The lighting engine calculates the light level for blocks.
///
/// 11. spawn: Mobs are spawned.
///
/// 12. full: Generation is done and a chunk can now be loaded. The proto-chunk is now converted to a level chunk and all block updates deferred in the above steps are executed.
///
pub struct ProtoChunk<'a> {
    chunk_pos: Vector2<i32>,
    pub sampler: ChunkNoiseGenerator<'a>,
    pub multi_noise_sampler: MultiNoiseSampler<'a>,
    random_config: &'a GlobalRandomConfig,
    settings: &'a GenerationSettings,
    default_block: BlockState,
    // These are local positions
    flat_block_map: Vec<BlockState>,
    flat_biome_map: Vec<Biome>,
    // may want to use chunk status
}

impl<'a> ProtoChunk<'a> {
    pub fn new(
        chunk_pos: Vector2<i32>,
        base_router: &'a GlobalProtoNoiseRouter,
        random_config: &'a GlobalRandomConfig,
        settings: &'a GenerationSettings,
    ) -> Self {
        let generation_shape = &settings.noise;

        let horizontal_cell_count = CHUNK_DIM / generation_shape.horizontal_cell_block_count();

        // TODO: Customize these
        let sampler = FluidLevelSampler::Chunk(StandardChunkFluidLevelSampler::new(
            FluidLevel::new(63, WATER_BLOCK),
            FluidLevel::new(-54, LAVA_BLOCK),
        ));

        let height = generation_shape.height;
        let start_x = chunk_pos::start_block_x(&chunk_pos);
        let start_z = chunk_pos::start_block_z(&chunk_pos);

        let sampler = ChunkNoiseGenerator::new(
            base_router,
            random_config,
            horizontal_cell_count,
            start_x,
            start_z,
            generation_shape,
            sampler,
            true,
            true,
        );
        // TODO: This is duplicate code already in ChunkNoiseGenerator::new
        let biome_pos = Vector2::new(
            biome_coords::from_block(start_x),
            biome_coords::from_block(start_z),
        );
        let horizontal_biome_end = biome_coords::from_block(
            horizontal_cell_count * generation_shape.horizontal_cell_block_count(),
        );
        let multi_noise_config = MultiNoiseSamplerBuilderOptions::new(
            biome_pos.x,
            biome_pos.z,
            horizontal_biome_end as usize,
        );
        let multi_noise_sampler = MultiNoiseSampler::generate(base_router, &multi_noise_config);
        let default_block = BlockState::new(&settings.default_block.name).unwrap();
        Self {
            chunk_pos,
            settings,
            default_block,
            random_config,
            sampler,
            multi_noise_sampler,
            flat_block_map: vec![
                BlockState::AIR;
                CHUNK_DIM as usize * CHUNK_DIM as usize * height as usize
            ],
            flat_biome_map: vec![
                Biome::Plains;
                CHUNK_DIM as usize * CHUNK_DIM as usize * height as usize
            ],
        }
    }

    #[inline]
    fn local_pos_to_index(&self, local_pos: &Vector3<i32>) -> usize {
        #[cfg(debug_assertions)]
        {
            assert!(local_pos.x >= 0 && local_pos.x <= 15);
            assert!(local_pos.y < self.sampler.height() as i32 && local_pos.y >= 0);
            assert!(local_pos.z >= 0 && local_pos.z <= 15);
        }
        self.sampler.height() as usize * CHUNK_DIM as usize * local_pos.x as usize
            + CHUNK_DIM as usize * local_pos.y as usize
            + local_pos.z as usize
    }

    #[inline]
    pub fn get_block_state(&self, local_pos: &Vector3<i32>) -> BlockState {
        let local_pos = Vector3::new(
            local_pos.x & 15,
            local_pos.y - self.sampler.min_y() as i32,
            local_pos.z & 15,
        );
        if local_pos.y < 0 || local_pos.y >= self.sampler.height() as i32 {
            BlockState::AIR
        } else {
            self.flat_block_map[self.local_pos_to_index(&local_pos)]
        }
    }

    #[inline]
    pub fn set_block_state(&mut self, local_pos: &Vector3<i32>, block_state: BlockState) {
        let local_pos = Vector3::new(
            local_pos.x & 15,
            local_pos.y - self.sampler.min_y() as i32,
            local_pos.z & 15,
        );
        let index = self.local_pos_to_index(&local_pos);
        self.flat_block_map[index] = block_state;
    }

    #[inline]
    pub fn get_biome(&self, local_pos: &Vector3<i32>) -> Biome {
        let local_pos = Vector3::new(
            local_pos.x & 15,
            local_pos.y - self.sampler.min_y() as i32,
            local_pos.z & 15,
        );
        if local_pos.y < 0 || local_pos.y >= self.sampler.height() as i32 {
            Biome::Plains
        } else {
            self.flat_biome_map[self.local_pos_to_index(&local_pos)]
        }
    }

    pub fn populate_biomes(&mut self) {
        let min_y = self.sampler.min_y();
        let bottom = section_coords::block_to_section(min_y) as i16;
        let top = section_coords::block_to_section(self.sampler.height()) as i16;

        let start_x = biome_coords::from_block(self.chunk_pos.x);
        let start_z = biome_coords::from_block(self.chunk_pos.z);

        for i in bottom..top {
            let start_y = biome_coords::from_block(i as i32);

            for x in 0..4 {
                for y in 0..4 {
                    for z in 0..4 {
                        let biome = MultiNoiseBiomeSupplier::biome(
                            Vector3::new(start_x + x, start_y + y, start_z + z),
                            &mut self.multi_noise_sampler,
                        );
                        let local_pos = Vector3 {
                            x: x & 15,
                            y: y - min_y as i32,
                            z: z & 15,
                        };
                        let index = self.local_pos_to_index(&local_pos);
                        self.flat_biome_map[index] = biome;
                    }
                }
            }
        }
    }

    pub fn populate_noise(&mut self) {
        let horizontal_cell_block_count = self.sampler.horizontal_cell_block_count();
        let vertical_cell_block_count = self.sampler.vertical_cell_block_count();

        let horizontal_cells = CHUNK_DIM / horizontal_cell_block_count;

        let min_y = self.sampler.min_y();
        let minimum_cell_y = min_y / vertical_cell_block_count as i8;
        let cell_height = self.sampler.height() / vertical_cell_block_count as u16;

        // TODO: Block state updates when we implement those
        self.sampler.sample_start_density();
        for cell_x in 0..horizontal_cells {
            self.sampler.sample_end_density(cell_x);
            let sample_start_x =
                (self.start_cell_x() + cell_x as i32) * horizontal_cell_block_count as i32;

            for cell_z in 0..horizontal_cells {
                for cell_y in (0..cell_height).rev() {
                    self.sampler.on_sampled_cell_corners(cell_x, cell_y, cell_z);
                    let sample_start_y =
                        (minimum_cell_y as i32 + cell_y as i32) * vertical_cell_block_count as i32;
                    let sample_start_z =
                        (self.start_cell_z() + cell_z as i32) * horizontal_cell_block_count as i32;
                    for local_y in (0..vertical_cell_block_count).rev() {
                        let block_y = (minimum_cell_y as i32 + cell_y as i32)
                            * vertical_cell_block_count as i32
                            + local_y as i32;
                        let delta_y = local_y as f64 / vertical_cell_block_count as f64;
                        self.sampler.interpolate_y(delta_y);

                        for local_x in 0..horizontal_cell_block_count {
                            let block_x = self.start_block_x()
                                + cell_x as i32 * horizontal_cell_block_count as i32
                                + local_x as i32;
                            let delta_x = local_x as f64 / horizontal_cell_block_count as f64;
                            self.sampler.interpolate_x(delta_x);

                            for local_z in 0..horizontal_cell_block_count {
                                let block_z = self.start_block_z()
                                    + cell_z as i32 * horizontal_cell_block_count as i32
                                    + local_z as i32;
                                let delta_z = local_z as f64 / horizontal_cell_block_count as f64;
                                self.sampler.interpolate_z(delta_z);

                                // TODO: Can the math here be simplified? Do the above values come
                                // to the same results?
                                let cell_offset_x = block_x - sample_start_x;
                                let cell_offset_y = block_y - sample_start_y;
                                let cell_offset_z = block_z - sample_start_z;

                                #[cfg(debug_assertions)]
                                {
                                    assert!(cell_offset_x >= 0);
                                    assert!(cell_offset_y >= 0);
                                    assert!(cell_offset_z >= 0);
                                }

                                let block_state = self
                                    .sampler
                                    .sample_block_state(
                                        sample_start_x,
                                        sample_start_y,
                                        sample_start_z,
                                        cell_offset_x as usize,
                                        cell_offset_y as usize,
                                        cell_offset_z as usize,
                                    )
                                    .unwrap_or(self.default_block);
                                //log::debug!("Sampled block state in {:?}", inst.elapsed());

                                let local_pos = Vector3 {
                                    x: block_x & 15,
                                    y: block_y - min_y as i32,
                                    z: block_z & 15,
                                };

                                #[cfg(debug_assertions)]
                                {
                                    assert!(local_pos.x < 16 && local_pos.x >= 0);
                                    assert!(
                                        local_pos.y < self.sampler.height() as i32
                                            && local_pos.y >= 0
                                    );
                                    assert!(local_pos.z < 16 && local_pos.z >= 0);
                                }
                                let index = self.local_pos_to_index(&local_pos);
                                self.flat_block_map[index] = block_state;
                            }
                        }
                    }
                }
            }

            self.sampler.swap_buffers();
        }
    }

    pub fn build_surface(&mut self) {
        let start_x = self.chunk_pos.x;
        let start_z = self.chunk_pos.z;

        let mut context = MaterialRuleContext {
            min_y: self.settings.noise.min_y,
            height: self.settings.noise.height,
            random_deriver: &self.random_config.base_random_deriver,
            block_pos: Vector3::new(0, 0, 0),
        };
        for x in 0..16 {
            for z in 0..16 {
                let x = start_x + x;
                let z = start_z + z;
                let top = self.sampler.height(); // TODO: use heightmaps
                for y in top..self.sampler.min_y() as u16 {
                    let state = self.get_block_state(&Vector3::new(x, y as i32, z));
                    if state.is_air() {
                        continue;
                    }
                    let pos = Vector3::new(x, y as i32, z);
                    context.block_pos = pos;
                    let new_state = self.settings.surface_rule.try_apply(&context);
                    if state != self.default_block || new_state.is_none() {
                        continue;
                    }
                    if let Some(state) = new_state {
                        self.set_block_state(&pos, state);
                    }
                }
            }
        }
    }

    fn start_cell_x(&self) -> i32 {
        self.start_block_x() / self.sampler.horizontal_cell_block_count() as i32
    }

    fn start_cell_z(&self) -> i32 {
        self.start_block_z() / self.sampler.horizontal_cell_block_count() as i32
    }

    fn start_block_x(&self) -> i32 {
        start_block_x(&self.chunk_pos)
    }

    fn start_block_z(&self) -> i32 {
        start_block_z(&self.chunk_pos)
    }
}

#[cfg(test)]
mod test {
    use std::sync::LazyLock;

    use pumpkin_util::math::vector2::Vector2;

    use crate::{
        generation::{
            GlobalRandomConfig,
            noise_router::{
                density_function::{NoiseFunctionComponentRange, PassThrough},
                proto_noise_router::{GlobalProtoNoiseRouter, ProtoNoiseFunctionComponent},
            },
            settings::{GENERATION_SETTINGS, GeneratorSetting},
        },
        noise_router::{NOISE_ROUTER_ASTS, density_function_ast::WrapperType},
        read_data_from_file,
    };

    use super::ProtoChunk;

    const SEED: u64 = 0;
    static RANDOM_CONFIG: LazyLock<GlobalRandomConfig> =
        LazyLock::new(|| GlobalRandomConfig::new(SEED));
    static BASE_NOISE_ROUTER: LazyLock<GlobalProtoNoiseRouter> = LazyLock::new(|| {
        GlobalProtoNoiseRouter::generate(&NOISE_ROUTER_ASTS.overworld, &RANDOM_CONFIG)
    });

    #[test]
    fn test_no_blend_no_beard_only_cell_cache() {
        // We say no wrapper, but it technically has a top-level cell cache
        let expected_data: Vec<u16> =
            read_data_from_file!("../../assets/no_blend_no_beard_only_cell_cache_0_0.chunk");

        let mut base_router = BASE_NOISE_ROUTER.clone();
        base_router
            .component_stack
            .iter_mut()
            .for_each(|component| {
                if let ProtoNoiseFunctionComponent::Wrapper(wrapper) = component {
                    match wrapper.wrapper_type() {
                        WrapperType::CellCache => (),
                        _ => {
                            *component = ProtoNoiseFunctionComponent::PassThrough(PassThrough {
                                input_index: wrapper.input_index(),
                                min_value: wrapper.min(),
                                max_value: wrapper.max(),
                            });
                        }
                    }
                }
            });
        let surface_config = GENERATION_SETTINGS
            .get(&GeneratorSetting::Overworld)
            .unwrap();
        let mut chunk = ProtoChunk::new(
            Vector2::new(0, 0),
            &base_router,
            &RANDOM_CONFIG,
            surface_config,
        );
        chunk.populate_noise();

        expected_data
            .into_iter()
            .zip(chunk.flat_block_map)
            .enumerate()
            .for_each(|(index, (expected, actual))| {
                if expected != actual.state_id {
                    panic!("{} vs {} ({})", expected, actual.state_id, index);
                }
            });
    }

    #[test]
    fn test_no_blend_no_beard_only_cell_2d_cache() {
        // it technically has a top-level cell cache
        // should be the same as only cell_cache
        let expected_data: Vec<u16> =
            read_data_from_file!("../../assets/no_blend_no_beard_only_cell_cache_0_0.chunk");

        let mut base_router = BASE_NOISE_ROUTER.clone();
        base_router
            .component_stack
            .iter_mut()
            .for_each(|component| {
                if let ProtoNoiseFunctionComponent::Wrapper(wrapper) = component {
                    match wrapper.wrapper_type() {
                        WrapperType::CellCache => (),
                        WrapperType::Cache2D => (),
                        _ => {
                            *component = ProtoNoiseFunctionComponent::PassThrough(PassThrough {
                                input_index: wrapper.input_index(),
                                min_value: wrapper.min(),
                                max_value: wrapper.max(),
                            });
                        }
                    }
                }
            });
        let surface_config = GENERATION_SETTINGS
            .get(&GeneratorSetting::Overworld)
            .unwrap();
        let mut chunk = ProtoChunk::new(
            Vector2::new(0, 0),
            &base_router,
            &RANDOM_CONFIG,
            surface_config,
        );
        chunk.populate_noise();

        expected_data
            .into_iter()
            .zip(chunk.flat_block_map)
            .enumerate()
            .for_each(|(index, (expected, actual))| {
                if expected != actual.state_id {
                    panic!("{} vs {} ({})", expected, actual.state_id, index);
                }
            });
    }

    #[test]
    fn test_no_blend_no_beard_only_cell_flat_cache() {
        // it technically has a top-level cell cache
        let expected_data: Vec<u16> = read_data_from_file!(
            "../../assets/no_blend_no_beard_only_cell_cache_flat_cache_0_0.chunk"
        );

        let mut base_router = BASE_NOISE_ROUTER.clone();
        base_router
            .component_stack
            .iter_mut()
            .for_each(|component| {
                if let ProtoNoiseFunctionComponent::Wrapper(wrapper) = component {
                    match wrapper.wrapper_type() {
                        WrapperType::CellCache => (),
                        WrapperType::CacheFlat => (),
                        _ => {
                            *component = ProtoNoiseFunctionComponent::PassThrough(PassThrough {
                                input_index: wrapper.input_index(),
                                min_value: wrapper.min(),
                                max_value: wrapper.max(),
                            });
                        }
                    }
                }
            });
        let surface_config = GENERATION_SETTINGS
            .get(&GeneratorSetting::Overworld)
            .unwrap();
        let mut chunk = ProtoChunk::new(
            Vector2::new(0, 0),
            &base_router,
            &RANDOM_CONFIG,
            surface_config,
        );
        chunk.populate_noise();

        expected_data
            .into_iter()
            .zip(chunk.flat_block_map)
            .enumerate()
            .for_each(|(index, (expected, actual))| {
                if expected != actual.state_id {
                    panic!("{} vs {} ({})", expected, actual.state_id, index);
                }
            });
    }

    #[test]
    fn test_no_blend_no_beard_only_cell_once_cache() {
        // it technically has a top-level cell cache
        let expected_data: Vec<u16> = read_data_from_file!(
            "../../assets/no_blend_no_beard_only_cell_cache_once_cache_0_0.chunk"
        );

        let mut base_router = BASE_NOISE_ROUTER.clone();
        base_router
            .component_stack
            .iter_mut()
            .for_each(|component| {
                if let ProtoNoiseFunctionComponent::Wrapper(wrapper) = component {
                    match wrapper.wrapper_type() {
                        WrapperType::CellCache => (),
                        WrapperType::CacheOnce => (),
                        _ => {
                            *component = ProtoNoiseFunctionComponent::PassThrough(PassThrough {
                                input_index: wrapper.input_index(),
                                min_value: wrapper.min(),
                                max_value: wrapper.max(),
                            });
                        }
                    }
                }
            });
        let surface_config = GENERATION_SETTINGS
            .get(&GeneratorSetting::Overworld)
            .unwrap();
        let mut chunk = ProtoChunk::new(
            Vector2::new(0, 0),
            &base_router,
            &RANDOM_CONFIG,
            surface_config,
        );
        chunk.populate_noise();

        expected_data
            .into_iter()
            .zip(chunk.flat_block_map)
            .enumerate()
            .for_each(|(index, (expected, actual))| {
                if expected != actual.state_id {
                    panic!("{} vs {} ({})", expected, actual.state_id, index);
                }
            });
    }

    #[test]
    fn test_no_blend_no_beard_only_cell_interpolated() {
        // it technically has a top-level cell cache
        let expected_data: Vec<u16> = read_data_from_file!(
            "../../assets/no_blend_no_beard_only_cell_cache_interpolated_0_0.chunk"
        );

        let mut base_router = BASE_NOISE_ROUTER.clone();
        base_router
            .component_stack
            .iter_mut()
            .for_each(|component| {
                if let ProtoNoiseFunctionComponent::Wrapper(wrapper) = component {
                    match wrapper.wrapper_type() {
                        WrapperType::CellCache => (),
                        WrapperType::Interpolated => (),
                        _ => {
                            *component = ProtoNoiseFunctionComponent::PassThrough(PassThrough {
                                input_index: wrapper.input_index(),
                                min_value: wrapper.min(),
                                max_value: wrapper.max(),
                            });
                        }
                    }
                }
            });
        let surface_config = GENERATION_SETTINGS
            .get(&GeneratorSetting::Overworld)
            .unwrap();
        let mut chunk = ProtoChunk::new(
            Vector2::new(0, 0),
            &base_router,
            &RANDOM_CONFIG,
            surface_config,
        );
        chunk.populate_noise();

        expected_data
            .into_iter()
            .zip(chunk.flat_block_map)
            .enumerate()
            .for_each(|(index, (expected, actual))| {
                if expected != actual.state_id {
                    panic!("{} vs {} ({})", expected, actual.state_id, index);
                }
            });
    }

    #[test]
    fn test_no_blend_no_beard() {
        let expected_data: Vec<u16> =
            read_data_from_file!("../../assets/no_blend_no_beard_0_0.chunk");
        let surface_config = GENERATION_SETTINGS
            .get(&GeneratorSetting::Overworld)
            .unwrap();
        let mut chunk = ProtoChunk::new(
            Vector2::new(0, 0),
            &BASE_NOISE_ROUTER,
            &RANDOM_CONFIG,
            surface_config,
        );
        chunk.populate_noise();

        assert_eq!(
            expected_data,
            chunk
                .flat_block_map
                .into_iter()
                .map(|state| state.state_id)
                .collect::<Vec<u16>>()
        );
    }

    #[test]
    fn test_no_blend_no_beard_aquifer() {
        let expected_data: Vec<u16> =
            read_data_from_file!("../../assets/no_blend_no_beard_7_4.chunk");
        let surface_config = GENERATION_SETTINGS
            .get(&GeneratorSetting::Overworld)
            .unwrap();
        let mut chunk = ProtoChunk::new(
            Vector2::new(7, 4),
            &BASE_NOISE_ROUTER,
            &RANDOM_CONFIG,
            surface_config,
        );
        chunk.populate_noise();

        assert_eq!(
            expected_data,
            chunk
                .flat_block_map
                .into_iter()
                .map(|state| state.state_id)
                .collect::<Vec<u16>>()
        );
    }
}
