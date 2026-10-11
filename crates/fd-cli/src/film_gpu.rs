//! Vulkan 1.2 compute shading and tent resampling for keyframe films.
//! All camera and fractal arithmetic remains in the CPU renderer.
use ash::{vk, Device, Entry, Instance};
use fd_samples::Samples;
use fd_shade::{Appearance, Look};
use gpu_allocator::vulkan::{
    Allocation, AllocationCreateDesc, AllocationScheme, Allocator, AllocatorCreateDesc,
};
use gpu_allocator::MemoryLocation;
use std::ffi::CStr;
use std::io::Cursor;
use std::mem::size_of;
use std::mem::ManuallyDrop;

const GROUP: u32 = 8;

type Rect = (usize, usize, usize, usize);
type Source<'a> = (&'a Samples, (usize, usize), Rect);

fn groups(n: u32) -> u32 {
    n.div_ceil(GROUP)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ShadePush {
    dims: [u32; 4],
    rect: [u32; 4],
    tone: [f32; 4],
    light: [f32; 4],
    color: [f32; 4],
    extra: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ResamplePush {
    dims: [u32; 4],
    more: [u32; 4],
    zoom: [f32; 4],
    blend: [f32; 4],
}

impl ResamplePush {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        output: (usize, usize),
        outer: (usize, usize),
        inner: (usize, usize),
        widths: (f32, f32),
        outer_offset: (f32, f32),
        inner_offset: (f32, f32),
        ramp: f32,
        margin: f32,
    ) -> Self {
        Self {
            dims: [
                output.0 as u32,
                output.1 as u32,
                outer.0 as u32,
                outer.1 as u32,
            ],
            more: [inner.0 as u32, inner.1 as u32, 0, 0],
            zoom: [widths.0, widths.1, outer_offset.0, outer_offset.1],
            blend: [inner_offset.0, inner_offset.1, ramp, margin],
        }
    }
}

fn linear(rgb: u32) -> [f32; 3] {
    [16, 8, 0].map(|shift| {
        let c = ((rgb >> shift) & 255) as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}

fn shade_push(
    l: &Look,
    a: &Appearance,
    full: (usize, usize),
    rect: (usize, usize, usize, usize),
) -> ShadePush {
    let stops: Vec<_> = l.stops.iter().map(|&s| linear(s)).collect();
    let mean: [f32; 3] =
        [0, 1, 2].map(|c| stops.iter().map(|v| v[c]).sum::<f32>() / stops.len() as f32);
    let inside = linear(l.interior);
    let (sin, cos) = l.light.to_radians().sin_cos();
    ShadePush {
        dims: [full.0 as u32, full.1 as u32, rect.0 as u32, rect.1 as u32],
        rect: [rect.2 as u32, rect.3 as u32, 0, 0],
        tone: [a.phase(), l.density as f32, l.terrace, l.slope],
        light: [cos, -sin, inside[0], inside[1]],
        color: [inside[2], mean[0], mean[1], mean[2]],
        extra: [
            if l.terrace > 0.0 {
                1.0 - l.terrace.max(0.0) * 0.4
            } else {
                1.0
            },
            stops.len() as f32,
            u8::from(a.aa) as f32,
            u8::from(a.unresolved_interior) as f32,
        ],
    }
}

/// Pack exactly the f64 phase base evaluated by PreparedStudio; do not cast
/// it to f32 before adding the animated palette phase (DEC-18).
/// Words 0..1 store the bitwise f64; word 2 stores DE and word 3 packs
/// u16 normal + class kind, preserving the 16-byte/sample SSBO layout.
fn pack_samples(s: &Samples, density: f64, out: &mut [u32]) -> Result<(), String> {
    let nu = s.nu.as_deref().ok_or("GPU needs nu")?;
    let de = s.de.as_deref().ok_or("GPU needs de")?;
    let normal = s.normal.as_deref().ok_or("GPU needs normal")?;
    if s.class.len() != nu.len()
        || nu.len() != de.len()
        || de.len() != normal.len()
        || out.len() != nu.len() * 4
    {
        return Err("GPU sample-column lengths differ".into());
    }
    for i in 0..nu.len() {
        let t = if s.class[i].escaped() {
            density * nu[i]
        } else {
            0.0
        };
        let bits = t.to_bits();
        out[i * 4..i * 4 + 4].copy_from_slice(&[
            bits as u32,
            (bits >> 32) as u32,
            de[i].to_bits(),
            u32::from(normal[i]) | (u32::from(s.class[i].0 & 3) << 16),
        ]);
    }
    Ok(())
}

struct Buffer {
    handle: vk::Buffer,
    allocation: Allocation,
    size: vk::DeviceSize,
}

struct Key {
    id: usize,
    buffer: Buffer,
    prepared: Buffer,
}

/// A GPU film keeps only the current pair's sample columns resident.
/// An unavailable device or unsupported look is a normal CPU fallback.
pub(crate) struct FilmGpu {
    _entry: Entry,
    instance: Instance,
    device: Device,
    allocator: ManuallyDrop<Allocator>,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    command: vk::CommandBuffer,
    descriptor_pool: vk::DescriptorPool,
    set_layout: vk::DescriptorSetLayout,
    sets: [vk::DescriptorSet; 4],
    pipeline_layout: vk::PipelineLayout,
    pipelines: [vk::Pipeline; 3],
    keys: Vec<Key>,
    shaded: Vec<Buffer>,
    output: Option<Buffer>,
    palette: Option<Buffer>,
    lut: Option<Buffer>,
    host_output: Vec<u8>,
    pub(crate) name: String,
    look: Look,
}

impl FilmGpu {
    pub(crate) fn supported(l: &Look, a: &Appearance) -> bool {
        l.lines == 0.0
            && l.haze.is_none() // CPU fallback: GPU has no dense-frame treatment
            && a.breathe == 0.0
            && (2..=32).contains(&l.stops.len())
            && l.density.is_finite()
            && l.density > 0.0
    }

    pub(crate) fn new(
        l: &Look,
        a: &Appearance,
        out: (usize, usize),
        multiple: usize,
    ) -> Result<Self, String> {
        if !Self::supported(l, a) {
            return Err(
                "studio look with lines, haze, breathing or >32 stops is not supported".into(),
            );
        }
        if multiple == 0 || out.0 == 0 || out.1 == 0 {
            return Err("invalid keyframe dimensions".into());
        }
        let entry = unsafe { Entry::load() }.map_err(|e| format!("Vulkan loader: {e}"))?;
        let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_2);
        let instance = unsafe {
            entry.create_instance(
                &vk::InstanceCreateInfo::default().application_info(&app),
                None,
            )
        }
        .map_err(|e| format!("Vulkan instance: {e}"))?;
        let selected = unsafe { instance.enumerate_physical_devices() }
            .map_err(|e| format!("Vulkan devices: {e}"))?
            .into_iter()
            .find_map(|physical| {
                let props = unsafe { instance.get_physical_device_properties(physical) };
                let version = props.api_version;
                if vk::api_version_major(version) < 1
                    || (vk::api_version_major(version) == 1 && vk::api_version_minor(version) < 2)
                {
                    return None;
                }
                // The palette coordinate needs IEEE binary64 rounding. If
                // unavailable, use the exact CPU film path (DEC-10).
                if unsafe { instance.get_physical_device_features(physical) }.shader_float64
                    != vk::TRUE
                {
                    return None;
                }
                let family =
                    unsafe { instance.get_physical_device_queue_family_properties(physical) }
                        .iter()
                        .position(|f| {
                            f.queue_count > 0 && f.queue_flags.contains(vk::QueueFlags::COMPUTE)
                        })?;
                Some((physical, family as u32, props))
            });
        let Some((physical, family, props)) = selected else {
            unsafe { instance.destroy_instance(None) };
            return Err("no Vulkan 1.2 compute device with shaderFloat64".into());
        };
        let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let size = out
            .0
            .checked_mul(multiple)
            .and_then(|w| w.checked_mul(out.1))
            .and_then(|n| n.checked_mul(multiple))
            .ok_or("keyframe size overflow")?;
        if size
            .checked_mul(16)
            .is_none_or(|bytes| bytes > props.limits.max_storage_buffer_range as usize)
        {
            unsafe { instance.destroy_instance(None) };
            return Err("sample buffer exceeds Vulkan maxStorageBufferRange".into());
        }
        let priorities = [1.0f32];
        let queues = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(family)
            .queue_priorities(&priorities)];
        let features = vk::PhysicalDeviceFeatures::default().shader_float64(true);
        let device = unsafe {
            instance.create_device(
                physical,
                &vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queues)
                    .enabled_features(&features),
                None,
            )
        }
        .map_err(|e| format!("Vulkan device: {e}"))?;
        let queue = unsafe { device.get_device_queue(family, 0) };
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: physical,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .map_err(|e| format!("GPU allocator: {e}"))?;
        let mut gpu = Self {
            _entry: entry,
            instance,
            device,
            allocator: ManuallyDrop::new(allocator),
            queue,
            command_pool: vk::CommandPool::null(),
            command: vk::CommandBuffer::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            set_layout: vk::DescriptorSetLayout::null(),
            sets: [vk::DescriptorSet::null(); 4],
            pipeline_layout: vk::PipelineLayout::null(),
            pipelines: [vk::Pipeline::null(); 3],
            keys: Vec::new(),
            shaded: Vec::new(),
            output: None,
            palette: None,
            lut: None,
            host_output: vec![0; out.0 * out.1 * 3],
            name,
            look: l.clone(),
        };
        gpu.init(family, out, multiple)?;
        Ok(gpu)
    }

    fn init(&mut self, family: u32, out: (usize, usize), multiple: usize) -> Result<(), String> {
        let dev = &self.device;
        self.command_pool = unsafe {
            dev.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(family)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )
        }
        .map_err(|e| format!("Vulkan command pool: {e}"))?;
        self.command = unsafe {
            dev.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
        }
        .map_err(|e| format!("Vulkan command buffer: {e}"))?[0];
        let bindings: Vec<_> = (0..4)
            .map(|i| {
                vk::DescriptorSetLayoutBinding::default()
                    .binding(i)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
            })
            .collect();
        self.set_layout = unsafe {
            dev.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                None,
            )
        }
        .map_err(|e| format!("Vulkan descriptor layout: {e}"))?;
        let sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 16,
        }];
        self.descriptor_pool = unsafe {
            dev.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .pool_sizes(&sizes)
                    .max_sets(4),
                None,
            )
        }
        .map_err(|e| format!("Vulkan descriptor pool: {e}"))?;
        let layouts = [self.set_layout; 4];
        let sets = unsafe {
            dev.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(self.descriptor_pool)
                    .set_layouts(&layouts),
            )
        }
        .map_err(|e| format!("Vulkan descriptors: {e}"))?;
        self.sets.copy_from_slice(&sets);
        let push = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .size(size_of::<ShadePush>() as u32)];
        self.pipeline_layout = unsafe {
            dev.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .set_layouts(&layouts[..1])
                    .push_constant_ranges(&push),
                None,
            )
        }
        .map_err(|e| format!("Vulkan pipeline layout: {e}"))?;
        for (i, data) in [
            &include_bytes!(concat!(env!("OUT_DIR"), "/film_prepare.spv"))[..],
            &include_bytes!(concat!(env!("OUT_DIR"), "/film_shade.spv"))[..],
            &include_bytes!(concat!(env!("OUT_DIR"), "/film_resample.spv"))[..],
        ]
        .into_iter()
        .enumerate()
        {
            let code =
                ash::util::read_spv(&mut Cursor::new(data)).map_err(|e| format!("SPIR-V: {e}"))?;
            let module = unsafe {
                dev.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&code), None)
            }
            .map_err(|e| format!("Vulkan shader module: {e}"))?;
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(module)
                .name(c"main");
            let pipeline = unsafe {
                dev.create_compute_pipelines(
                    vk::PipelineCache::null(),
                    &[vk::ComputePipelineCreateInfo::default()
                        .stage(stage)
                        .layout(self.pipeline_layout)],
                    None,
                )
            }
            .map_err(|(_, e)| format!("Vulkan compute pipeline: {e}"))?[0];
            unsafe { dev.destroy_shader_module(module, None) };
            self.pipelines[i] = pipeline;
        }
        let count = out.0 * out.1 * multiple * multiple;
        let outer = self.buffer(count * 4, MemoryLocation::GpuOnly, "film shade outer")?;
        self.shaded.push(outer);
        let inner = self.buffer(count * 4, MemoryLocation::GpuOnly, "film shade inner")?;
        self.shaded.push(inner);
        self.output = Some(self.buffer(
            out.0 * out.1 * 4,
            MemoryLocation::GpuToCpu,
            "film RGB readback",
        )?);
        let palette = self.buffer(
            self.look.stops.len() * 16,
            MemoryLocation::CpuToGpu,
            "film palette",
        )?;
        let mut colors = Vec::with_capacity(self.look.stops.len() * 16);
        for &stop in &self.look.stops {
            for v in linear(stop).into_iter().chain([0.0]) {
                colors.extend_from_slice(&v.to_ne_bytes());
            }
        }
        self.write_mapped(&palette, &colors)?;
        self.palette = Some(palette);
        // The exact CPU tent gamma-decode table, uploaded once per film.
        let lut = self.buffer(256 * 4, MemoryLocation::CpuToGpu, "film gamma decode LUT")?;
        let mut values = Vec::with_capacity(256 * 4);
        for i in 0..256 {
            values.extend_from_slice(&((i as f32 / 255.0).powf(2.2)).to_ne_bytes());
        }
        self.write_mapped(&lut, &values)?;
        self.lut = Some(lut);
        Ok(())
    }

    fn buffer(
        &mut self,
        len: usize,
        location: MemoryLocation,
        name: &str,
    ) -> Result<Buffer, String> {
        let size = len as vk::DeviceSize;
        let handle = unsafe {
            self.device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(size)
                    .usage(
                        vk::BufferUsageFlags::STORAGE_BUFFER
                            | vk::BufferUsageFlags::TRANSFER_DST
                            | vk::BufferUsageFlags::TRANSFER_SRC,
                    )
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )
        }
        .map_err(|e| format!("GPU buffer: {e}"))?;
        let requirements = unsafe { self.device.get_buffer_memory_requirements(handle) };
        let allocation = self
            .allocator
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("GPU allocation: {e}"))?;
        unsafe {
            self.device
                .bind_buffer_memory(handle, allocation.memory(), allocation.offset())
        }
        .map_err(|e| format!("GPU bind buffer: {e}"))?;
        Ok(Buffer {
            handle,
            allocation,
            size,
        })
    }

    fn free_buffer(&mut self, buf: Buffer) {
        unsafe { self.device.destroy_buffer(buf.handle, None) };
        let _ = self.allocator.free(buf.allocation);
    }

    fn mapped_ptr(buffer: &Buffer) -> Result<*mut u8, String> {
        buffer
            .allocation
            .mapped_ptr()
            .map(|p| p.as_ptr() as *mut u8)
            .ok_or_else(|| "GPU buffer is not host mapped".to_string())
    }

    fn mapped_range(buffer: &Buffer) -> vk::MappedMemoryRange<'static> {
        // The entire mapped allocation is flushed/invalidated. This also handles
        // non-coherent atom alignment when the allocator suballocates memory.
        vk::MappedMemoryRange::default()
            .memory(unsafe { buffer.allocation.memory() })
            .offset(0)
            .size(vk::WHOLE_SIZE)
    }

    fn write_mapped(&self, buffer: &Buffer, data: &[u8]) -> Result<(), String> {
        if data.len() as u64 > buffer.size {
            return Err("GPU upload exceeds buffer".into());
        }
        let ptr = Self::mapped_ptr(buffer)?;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
            self.device
                .flush_mapped_memory_ranges(&[Self::mapped_range(buffer)])
        }
        .map_err(|e| format!("GPU flush: {e}"))
    }

    fn start(&self) -> Result<(), String> {
        unsafe {
            self.device
                .reset_command_buffer(self.command, vk::CommandBufferResetFlags::empty())
                .map_err(|e| format!("Vulkan reset: {e}"))?;
            self.device
                .begin_command_buffer(
                    self.command,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .map_err(|e| format!("Vulkan begin: {e}"))?;
        }
        Ok(())
    }

    fn submit(&self) -> Result<(), String> {
        unsafe {
            self.device
                .end_command_buffer(self.command)
                .map_err(|e| format!("Vulkan end: {e}"))?;
            let commands = [self.command];
            self.device
                .queue_submit(
                    self.queue,
                    &[vk::SubmitInfo::default().command_buffers(&commands)],
                    vk::Fence::null(),
                )
                .map_err(|e| format!("Vulkan submit: {e}"))?;
            self.device
                .queue_wait_idle(self.queue)
                .map_err(|e| format!("Vulkan wait: {e}"))?;
        }
        Ok(())
    }

    fn ensure_key(
        &mut self,
        id: usize,
        s: &Samples,
        full: (usize, usize),
        a: &Appearance,
    ) -> Result<(), String> {
        if self.keys.iter().any(|k| k.id == id) {
            return Ok(());
        }
        if s.class.len() != full.0 * full.1 {
            return Err("GPU keyframe dimensions differ from samples".into());
        }
        let len = s
            .class
            .len()
            .checked_mul(size_of::<[f32; 4]>())
            .ok_or("GPU keyframe byte count overflow")?;
        let key = self.buffer(len, MemoryLocation::GpuOnly, "film keyframe")?;
        let prepared = self.buffer(len, MemoryLocation::GpuOnly, "film prepared keyframe")?;
        let staging = self.buffer(len, MemoryLocation::CpuToGpu, "film upload")?;
        // Directly fill mapped staging as f32: avoid allocating and copying a
        // second ~530 MiB CPU buffer per 4K keyframe.
        let ptr = Self::mapped_ptr(&staging)?;
        if !(ptr as usize).is_multiple_of(std::mem::align_of::<u32>()) {
            return Err("GPU mapped staging has insufficient alignment".into());
        }
        let dst = unsafe { std::slice::from_raw_parts_mut(ptr.cast::<u32>(), len / 4) };
        pack_samples(s, self.look.density, dst)?;
        unsafe {
            self.device
                .flush_mapped_memory_ranges(&[Self::mapped_range(&staging)])
        }
        .map_err(|e| format!("GPU staging flush: {e}"))?;
        // Preparation is time-invariant (breathe=0): compute relief, band AA,
        // and light once on the GPU instead of recomputing it for every frame.
        self.bind_prepare(&key, &prepared);
        self.start()?;
        unsafe {
            self.device.cmd_copy_buffer(
                self.command,
                staging.handle,
                key.handle,
                &[vk::BufferCopy::default().size(len as u64)],
            );
            let barrier = [vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)];
            self.device.cmd_pipeline_barrier(
                self.command,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &barrier,
                &[],
                &[],
            );
        }
        unsafe {
            self.device.cmd_bind_pipeline(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.pipelines[0],
            );
            self.device.cmd_bind_descriptor_sets(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                &self.sets[3..4],
                &[],
            );
        }
        self.push(&shade_push(&self.look, a, full, (0, 0, full.0, full.1)));
        unsafe {
            self.device.cmd_dispatch(
                self.command,
                groups(full.0 as u32),
                groups(full.1 as u32),
                1,
            );
            // Availability/visibility for the subsequent shading submissions.
            let barrier = [vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)];
            self.device.cmd_pipeline_barrier(
                self.command,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &barrier,
                &[],
                &[],
            );
        }
        self.submit()?;
        self.free_buffer(staging);
        self.keys.push(Key {
            id,
            buffer: key,
            prepared,
        });
        Ok(())
    }

    fn bind_prepare(&self, key: &Buffer, prepared: &Buffer) {
        let info = [
            vk::DescriptorBufferInfo::default()
                .buffer(key.handle)
                .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfo::default()
                .buffer(prepared.handle)
                .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfo::default()
                .buffer(self.palette.as_ref().expect("palette").handle)
                .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfo::default()
                .buffer(self.lut.as_ref().expect("lut").handle)
                .range(vk::WHOLE_SIZE),
        ];
        let writes: Vec<_> = (0..4)
            .map(|i| {
                vk::WriteDescriptorSet::default()
                    .dst_set(self.sets[3])
                    .dst_binding(i as u32)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&info[i..i + 1])
            })
            .collect();
        unsafe { self.device.update_descriptor_sets(&writes, &[]) };
    }

    fn bind(&self, outer: &Buffer, inner: &Buffer) {
        let lut = self.lut.as_ref().expect("lut").handle;
        let b = [
            outer.handle,
            self.shaded[0].handle,
            self.palette.as_ref().expect("palette").handle,
            lut,
            inner.handle,
            self.shaded[1].handle,
            self.palette.as_ref().expect("palette").handle,
            lut,
            self.shaded[0].handle,
            self.shaded[1].handle,
            self.output.as_ref().expect("output").handle,
            lut,
        ];
        let infos: Vec<_> = b
            .into_iter()
            .map(|buffer| {
                vk::DescriptorBufferInfo::default()
                    .buffer(buffer)
                    .offset(0)
                    .range(vk::WHOLE_SIZE)
            })
            .collect();
        let writes: Vec<_> = (0..12)
            .map(|i| {
                vk::WriteDescriptorSet::default()
                    .dst_set(self.sets[i / 4])
                    .dst_binding((i % 4) as u32)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&infos[i..i + 1])
            })
            .collect();
        unsafe { self.device.update_descriptor_sets(&writes, &[]) };
    }

    fn push<T: Copy>(&self, value: &T) {
        let bytes =
            unsafe { std::slice::from_raw_parts((value as *const T).cast::<u8>(), size_of::<T>()) };
        unsafe {
            self.device.cmd_push_constants(
                self.command,
                self.pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                bytes,
            );
        }
    }

    /// Shade both cached columns, tent blend, then read only RGB8 back for ffmpeg.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn frame(
        &mut self,
        outer_id: usize,
        outer: Source<'_>,
        inner_id: usize,
        inner: Source<'_>,
        a: &Appearance,
        resample: ResamplePush,
    ) -> Result<&[u8], String> {
        if !Self::supported(&self.look, a) {
            return Err("GPU look became unsupported".into());
        }
        self.ensure_key(outer_id, outer.0, outer.1, a)?;
        self.ensure_key(inner_id, inner.0, inner.1, a)?;
        let old = std::mem::take(&mut self.keys);
        for key in old {
            if key.id < outer_id {
                self.free_buffer(key.buffer);
                self.free_buffer(key.prepared);
            } else {
                self.keys.push(key);
            }
        }
        let ob = &self
            .keys
            .iter()
            .find(|k| k.id == outer_id)
            .expect("uploaded")
            .prepared;
        let ib = &self
            .keys
            .iter()
            .find(|k| k.id == inner_id)
            .expect("uploaded")
            .prepared;
        self.bind(ob, ib);
        self.start()?;
        for (i, (_, size, rect)) in [outer, inner].into_iter().enumerate() {
            unsafe {
                self.device.cmd_bind_pipeline(
                    self.command,
                    vk::PipelineBindPoint::COMPUTE,
                    self.pipelines[1],
                );
                self.device.cmd_bind_descriptor_sets(
                    self.command,
                    vk::PipelineBindPoint::COMPUTE,
                    self.pipeline_layout,
                    0,
                    &self.sets[i..i + 1],
                    &[],
                );
            }
            self.push(&shade_push(&self.look, a, size, rect));
            unsafe {
                self.device.cmd_dispatch(
                    self.command,
                    groups(rect.2 as u32),
                    groups(rect.3 as u32),
                    1,
                );
            }
        }
        unsafe {
            let barrier = [vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)];
            self.device.cmd_pipeline_barrier(
                self.command,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &barrier,
                &[],
                &[],
            );
            self.device.cmd_bind_pipeline(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.pipelines[2],
            );
            self.device.cmd_bind_descriptor_sets(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                &self.sets[2..3],
                &[],
            );
        }
        self.push(&resample);
        unsafe {
            self.device.cmd_dispatch(
                self.command,
                groups(resample.dims[0]),
                groups(resample.dims[1]),
                1,
            );
            let barrier = [vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ)];
            self.device.cmd_pipeline_barrier(
                self.command,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &barrier,
                &[],
                &[],
            );
        }
        self.submit()?;
        let output = self.output.as_ref().expect("allocated output");
        unsafe {
            self.device
                .invalidate_mapped_memory_ranges(&[Self::mapped_range(output)])
        }
        .map_err(|e| format!("GPU invalidate: {e}"))?;
        let ptr = Self::mapped_ptr(output)? as *const u32;
        let n = self.host_output.len() / 3;
        let data = unsafe { std::slice::from_raw_parts(ptr, n) };
        for (rgb, pixel) in self.host_output.chunks_exact_mut(3).zip(data) {
            rgb.copy_from_slice(&pixel.to_le_bytes()[..3]);
        }
        Ok(&self.host_output)
    }
}

impl Drop for FilmGpu {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
        }
        for key in std::mem::take(&mut self.keys) {
            self.free_buffer(key.buffer);
            self.free_buffer(key.prepared);
        }
        for buffer in std::mem::take(&mut self.shaded) {
            self.free_buffer(buffer);
        }
        if let Some(buf) = self.palette.take() {
            self.free_buffer(buf);
        }
        if let Some(buf) = self.lut.take() {
            self.free_buffer(buf);
        }
        if let Some(buf) = self.output.take() {
            self.free_buffer(buf);
        }
        unsafe {
            for p in self.pipelines {
                if p != vk::Pipeline::null() {
                    self.device.destroy_pipeline(p, None);
                }
            }
            if self.pipeline_layout != vk::PipelineLayout::null() {
                self.device
                    .destroy_pipeline_layout(self.pipeline_layout, None);
            }
            if self.descriptor_pool != vk::DescriptorPool::null() {
                self.device
                    .destroy_descriptor_pool(self.descriptor_pool, None);
            }
            if self.set_layout != vk::DescriptorSetLayout::null() {
                self.device
                    .destroy_descriptor_set_layout(self.set_layout, None);
            }
            if self.command_pool != vk::CommandPool::null() {
                self.device.destroy_command_pool(self.command_pool, None);
            }
            ManuallyDrop::drop(&mut self.allocator);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_layout_and_dispatch_are_portable() {
        assert_eq!(size_of::<ShadePush>(), 96);
        assert_eq!(size_of::<ResamplePush>(), 64);
        assert_eq!(groups(1), 1);
        assert_eq!(groups(8), 1);
        assert_eq!(groups(16), 2);
        assert_eq!(groups(17), 3);
        assert_eq!((groups(3840), groups(2160)), (480, 270));
        assert_eq!((groups(7680), groups(4320)), (960, 540));
    }

    #[test]
    fn source_packing_and_look_guard() {
        use fd_samples::{Class, Evidence, Kind};
        let s = Samples {
            class: vec![
                Class::new(Kind::Escaped, Evidence::Heuristic),
                Class::new(Kind::Interior, Evidence::Certified),
            ],
            nu: Some(vec![2000.25, 0.0]),
            de: Some(vec![1.5, 0.0]),
            normal: Some(vec![32768, 0]),
            ..Default::default()
        };
        let mut words = vec![0u32; 8];
        pack_samples(&s, 0.05, &mut words).unwrap();
        assert_eq!(words.len(), 8);
        let phase = f64::from_bits(u64::from(words[0]) | (u64::from(words[1]) << 32));
        assert_eq!(phase, 2000.25 * 0.05);
        assert_eq!(f32::from_bits(words[2]), 1.5);
        assert_eq!(words[3], 32768);
        assert_eq!(words[7] >> 16, 1);
        let mut look = Look::builtin("ice").unwrap();
        let a = Appearance::STILL;
        assert!(!FilmGpu::supported(&look, &a));
        look.lines = 0.0;
        assert!(FilmGpu::supported(&look, &a));
        assert!(!FilmGpu::supported(
            &look,
            &Appearance { breathe: 0.1, ..a }
        ));
    }

    #[test]
    fn palette_phase_wrap_regression() {
        // Animated master-ice terrace: the old GPU reduction rounds a value
        // just *below* a whole palette cycle up to 1.0, then fract() makes it
        // 0.0. CPU x is 8.0 (clamped to stop 7, fr=1), not stop 0, fr=0.
        let phase = (-0.11_f64 * (2.0 / 30.0)).rem_euclid(1.0) as f32;
        let t = 2048.0_f64 + 1.0 - f64::from(phase) - 1e-8;
        let cpu = ((t + f64::from(phase)).rem_euclid(1.0) * 8.0) as f32;
        let truncated = (t.rem_euclid(1.0) as f32 + phase).fract() * 8.0;
        assert_eq!(cpu, 8.0);
        assert_eq!(truncated, 0.0);
        let mut packed = vec![0u32; 4];
        let s = fd_samples::Samples {
            class: vec![fd_samples::Class::new(
                fd_samples::Kind::Escaped,
                fd_samples::Evidence::Heuristic,
            )],
            nu: Some(vec![t / 0.05]),
            de: Some(vec![1.0]),
            normal: Some(vec![1234]),
            ..Default::default()
        };
        pack_samples(&s, 0.05, &mut packed).unwrap();
        let reconstructed = f64::from_bits(u64::from(packed[0]) | (u64::from(packed[1]) << 32));
        assert_eq!(reconstructed, 0.05 * s.nu.as_ref().unwrap()[0]);
        assert_eq!(packed[3], 1234);
    }

    #[test]
    fn rounded_f32_stop_index_covers_palette_edges() {
        // Equivalent to the shader's positive-f32 integer-part decode.
        for n in 2usize..=32 {
            for whole in 0..=n {
                let bits = (whole as f32).to_bits();
                for b in [bits.saturating_sub(1), bits, bits + 1] {
                    let exponent = (b >> 23) & 255;
                    let shift = 31u32.min(150u32.wrapping_sub(exponent));
                    let k = (((b & 0x7fffff) | 0x800000) >> shift) as usize;
                    let expected = (f32::from_bits(b) as usize).min(n - 1);
                    assert_eq!(k.min(n - 1), expected, "n={n}, x={:08x}", b);
                }
            }
        }
    }

    #[test]
    fn gpu_phase_transition_matches_prepared_cpu_tent() {
        // Optional hardware test; CI also executes it on Mesa llvmpipe.
        // Samples straddle whole-cycle and individual stop/terrace edges
        // at actual animated master-ice phase. The test intentionally gives
        // the shader every opportunity to choose the wrong band.
        use fd_samples::{Class, Column, ColumnSet, Evidence, Header, Kind, View, MINOR};
        use fd_shade::{PreparedStudio, Rgb8};
        let look = Look::parse(include_str!("../../../looks/master-ice.look")).unwrap();
        let appearance = Appearance {
            time: 2.0 / 30.0,
            flow: 0.11,
            aa: true,
            dither: true,
            unresolved_interior: true,
            ..Appearance::STILL
        };
        let Ok(mut gpu) = FilmGpu::new(&look, &appearance, (8, 8), 2) else {
            eprintln!("GPU phase edge test skipped (no float64 Vulkan compute device)");
            return;
        };
        let (w, h) = (16usize, 16usize);
        let columns = ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal]);
        let mut samples = Samples::alloc(w * h, columns);
        let phase = f64::from(appearance.phase());
        let offsets = [-2e-7, -5e-8, -1e-8, -1e-10, 0.0, 1e-10, 1e-8, 5e-8];
        for i in 0..w * h {
            let kind = if i % 53 == 0 {
                Kind::Interior
            } else if i % 59 == 0 {
                Kind::Unresolved
            } else {
                Kind::Escaped
            };
            samples.class[i] = Class::new(kind, Evidence::Heuristic);
            let target = if i % 2 == 0 { 1.0 } else { 0.125 };
            let t = 2048.0 + target - phase + offsets[i % offsets.len()];
            samples.nu.as_mut().unwrap()[i] = t / look.density;
            samples.de.as_mut().unwrap()[i] = 1.5;
            samples.normal.as_mut().unwrap()[i] = (i * 137) as u16;
        }
        let header = Header {
            minor: MINOR,
            columns,
            nx: w as u32,
            ny: h as u32,
            ss: 1,
            max_iter: 100_000,
            escape_radius: 1e10,
            view: View {
                center_re: "0".into(),
                center_im: "0".into(),
                width: "1".into(),
                rotation: 0.0,
            },
            kernel: "test".into(),
        };
        let prepared = PreparedStudio::new(&look, &header, &samples, &appearance).unwrap();
        let mut cpu = Rgb8 {
            w: 0,
            h: 0,
            data: Vec::new(),
        };
        prepared.shade_rect_film_into((0, 0, w, h), &appearance, &mut cpu);
        let push = ResamplePush::new(
            (8, 8),
            (w, h),
            (w, h),
            (1.0, 2.0),
            (0.0, 0.0),
            (0.0, 0.0),
            0.0,
            0.1,
        );
        let got = gpu
            .frame(
                0,
                (&samples, (w, h), (0, 0, w, h)),
                1,
                (&samples, (w, h), (0, 0, w, h)),
                &appearance,
                push,
            )
            .expect("Vulkan synthetic phase-boundary frame")
            .to_vec();

        // DEC-18 diagnostic: compare shaded keyframe texels BEFORE the tent.
        // A one-sample palette discontinuity must not be confused with a
        // resampler or encoding error.
        let capture = gpu
            .buffer(
                w * h * 4,
                MemoryLocation::GpuToCpu,
                "phase regression readback",
            )
            .unwrap();
        gpu.start().unwrap();
        unsafe {
            let barrier = [vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)];
            gpu.device.cmd_pipeline_barrier(
                gpu.command,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &barrier,
                &[],
                &[],
            );
            gpu.device.cmd_copy_buffer(
                gpu.command,
                gpu.shaded[0].handle,
                capture.handle,
                &[vk::BufferCopy::default().size((w * h * 4) as u64)],
            );
        }
        gpu.submit().unwrap();
        unsafe {
            gpu.device
                .invalidate_mapped_memory_ranges(&[FilmGpu::mapped_range(&capture)])
        }
        .unwrap();
        let source_gpu: Vec<u32> = unsafe {
            std::slice::from_raw_parts(FilmGpu::mapped_ptr(&capture).unwrap().cast::<u32>(), w * h)
        }
        .to_vec();
        gpu.free_buffer(capture);

        // Source pixels must agree before the tent can hide stop-boundary errors.
        for (i, &pixel) in source_gpu.iter().enumerate() {
            for (channel, &actual) in pixel.to_le_bytes()[..3].iter().enumerate() {
                let expected = cpu.data[i * 3 + channel];
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "source texel ({},{}) channel {channel}: CPU={expected} GPU={actual}",
                    i % w,
                    i / w,
                );
            }
        }
        let lut: [f32; 256] = std::array::from_fn(|i| (i as f32 / 255.0).powf(2.2));
        for y in 0..8 {
            for x in 0..8 {
                for channel in 0..3 {
                    let linear: f32 = (0..2)
                        .flat_map(|dy| (0..2).map(move |dx| (dy, dx)))
                        .map(|(dy, dx)| {
                            let i = ((2 * y + dy) * w + 2 * x + dx) * 3 + channel;
                            lut[cpu.data[i] as usize] * 0.25
                        })
                        .sum();
                    let reference = (linear.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
                    let actual = got[(y * 8 + x) * 3 + channel];
                    assert!(
                        reference.abs_diff(actual) <= 1,
                        "CPU/GPU palette edge at output ({x},{y}) channel {channel}: CPU={reference}, GPU={actual}"
                    );
                }
            }
        }
    }

    #[test]
    fn optional_device_skips_cleanly() {
        if let Ok(gpu) = FilmGpu::new(
            &Look {
                lines: 0.0,
                ..Look::builtin("ice").unwrap()
            },
            &Appearance::STILL,
            (8, 8),
            2,
        ) {
            assert!(!gpu.name.is_empty());
        }
    }
}
