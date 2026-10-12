//! FP64 Vulkan zone samples. The CPU preflight owns decimal camera arithmetic and
//! the zone validity gate; a rejected GPU path retains the exact CPU renderer.
use ash::{vk, Device, Entry, Instance};
use fd_kernel::{Params, Zone};
use fd_samples::{Class, Column, Evidence, Header, Kind, Samples, View};
use gpu_allocator::vulkan::{
    Allocation, AllocationCreateDesc, AllocationScheme, Allocator, AllocatorCreateDesc,
};
use gpu_allocator::MemoryLocation;
use std::ffi::CStr;
use std::io::Cursor;
use std::mem::ManuallyDrop;
use std::time::Instant;

struct Buffer {
    handle: vk::Buffer,
    allocation: Allocation,
}

struct Gpu {
    _entry: Entry,
    instance: Instance,
    device: Device,
    allocator: ManuallyDrop<Allocator>,
    queue: vk::Queue,
    pool: vk::CommandPool,
    descriptor_pool: vk::DescriptorPool,
    set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    command: vk::CommandBuffer,
    constants: Option<Buffer>,
    output: Option<Buffer>,
    name: String,
}

impl Gpu {
    fn new(count: usize, values: usize) -> Result<Self, String> {
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
            .map_err(|e| format!("Vulkan enumerate: {e}"))?
            .into_iter()
            .find_map(|physical| {
                let props = unsafe { instance.get_physical_device_properties(physical) };
                if vk::api_version_major(props.api_version) < 1
                    || (vk::api_version_major(props.api_version) == 1
                        && vk::api_version_minor(props.api_version) < 2)
                    || unsafe { instance.get_physical_device_features(physical) }.shader_float64
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
                let bytes = count.checked_mul(32)?;
                let const_bytes = values.checked_mul(8)?;
                if bytes > props.limits.max_storage_buffer_range as usize
                    || const_bytes > props.limits.max_storage_buffer_range as usize
                    || count.div_ceil(64) > props.limits.max_compute_work_group_count[0] as usize
                {
                    return None;
                }
                Some((physical, family as u32, props))
            });
        let Some((physical, family, props)) = selected else {
            unsafe { instance.destroy_instance(None) };
            return Err(
                "no Vulkan 1.2 compute device with shaderFloat64 and sufficient buffer limits"
                    .into(),
            );
        };
        let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
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
        .map_err(|e| format!("Vulkan allocator: {e}"))?;
        let mut gpu = Self {
            _entry: entry,
            instance,
            device,
            allocator: ManuallyDrop::new(allocator),
            queue,
            pool: vk::CommandPool::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            set_layout: vk::DescriptorSetLayout::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
            command: vk::CommandBuffer::null(),
            constants: None,
            output: None,
            name,
        };
        gpu.init(family, count, values)?;
        Ok(gpu)
    }

    fn buffer(
        &mut self,
        len: usize,
        location: MemoryLocation,
        label: &str,
    ) -> Result<Buffer, String> {
        let handle = unsafe {
            self.device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(len as u64)
                    .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )
        }
        .map_err(|e| format!("Vulkan buffer: {e}"))?;
        let requirements = unsafe { self.device.get_buffer_memory_requirements(handle) };
        let allocation = self
            .allocator
            .allocate(&AllocationCreateDesc {
                name: label,
                requirements,
                location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("Vulkan allocation: {e}"))?;
        unsafe {
            self.device
                .bind_buffer_memory(handle, allocation.memory(), allocation.offset())
        }
        .map_err(|e| format!("Vulkan bind memory: {e}"))?;
        Ok(Buffer { handle, allocation })
    }

    fn init(&mut self, family: u32, count: usize, values: usize) -> Result<(), String> {
        let dev = self.device.clone();
        self.pool = unsafe {
            dev.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(family),
                None,
            )
        }
        .map_err(|e| format!("Vulkan command pool: {e}"))?;
        self.command = unsafe {
            dev.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
        }
        .map_err(|e| format!("Vulkan commands: {e}"))?[0];
        let bindings: Vec<_> = (0..2)
            .map(|binding| {
                vk::DescriptorSetLayoutBinding::default()
                    .binding(binding)
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
        .map_err(|e| format!("Vulkan set layout: {e}"))?;
        let sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 2,
        }];
        self.descriptor_pool = unsafe {
            dev.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .pool_sizes(&sizes)
                    .max_sets(1),
                None,
            )
        }
        .map_err(|e| format!("Vulkan descriptor pool: {e}"))?;
        let layouts = [self.set_layout];
        let set = unsafe {
            dev.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(self.descriptor_pool)
                    .set_layouts(&layouts),
            )
        }
        .map_err(|e| format!("Vulkan descriptor set: {e}"))?[0];
        self.pipeline_layout = unsafe {
            dev.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts),
                None,
            )
        }
        .map_err(|e| format!("Vulkan pipeline layout: {e}"))?;
        let binary = include_bytes!(concat!(env!("OUT_DIR"), "/zone_f64.spv"));
        let words = ash::util::read_spv(&mut Cursor::new(binary))
            .map_err(|e| format!("Vulkan SPIR-V: {e}"))?;
        let module = unsafe {
            dev.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&words), None)
        }
        .map_err(|e| format!("Vulkan shader: {e}"))?;
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(module)
            .name(c"main");
        let result = unsafe {
            dev.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[vk::ComputePipelineCreateInfo::default()
                    .stage(stage)
                    .layout(self.pipeline_layout)],
                None,
            )
        };
        unsafe { dev.destroy_shader_module(module, None) };
        self.pipeline = result.map_err(|(_, e)| format!("Vulkan pipeline: {e}"))?[0];
        self.constants =
            Some(self.buffer(values * 8, MemoryLocation::CpuToGpu, "zone f64 constants")?);
        self.output =
            Some(self.buffer(count * 32, MemoryLocation::GpuToCpu, "zone f64 columns")?);
        let descriptors = [
            vk::DescriptorBufferInfo::default()
                .buffer(self.constants.as_ref().unwrap().handle)
                .range((values * 8) as u64),
            vk::DescriptorBufferInfo::default()
                .buffer(self.output.as_ref().unwrap().handle)
                .range((count * 32) as u64),
        ];
        let writes: Vec<_> = (0..2)
            .map(|i| {
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(i as u32)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&descriptors[i]))
            })
            .collect();
        unsafe { dev.update_descriptor_sets(&writes, &[]) };
        // One dispatch for all samples; each invocation writes a complete four-column tuple.
        unsafe {
            dev.begin_command_buffer(self.command, &vk::CommandBufferBeginInfo::default())
                .map_err(|e| format!("Vulkan begin: {e}"))?;
            let host_to_shader = [vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::HOST_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)];
            dev.cmd_pipeline_barrier(
                self.command,
                vk::PipelineStageFlags::HOST,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &host_to_shader,
                &[],
                &[],
            );
            dev.cmd_bind_pipeline(self.command, vk::PipelineBindPoint::COMPUTE, self.pipeline);
            dev.cmd_bind_descriptor_sets(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                &[set],
                &[],
            );
            dev.cmd_dispatch(self.command, count.div_ceil(64) as u32, 1, 1);
            dev.end_command_buffer(self.command)
                .map_err(|e| format!("Vulkan end: {e}"))?;
        }
        Ok(())
    }

    fn mapped(buffer: &Buffer) -> Result<*mut u8, String> {
        buffer
            .allocation
            .mapped_ptr()
            .map(|p| p.as_ptr() as *mut u8)
            .ok_or_else(|| "Vulkan buffer is not host-mapped".to_string())
    }

    fn range(buffer: &Buffer) -> vk::MappedMemoryRange<'static> {
        vk::MappedMemoryRange::default()
            .memory(unsafe { buffer.allocation.memory() })
            .offset(0)
            .size(vk::WHOLE_SIZE)
    }

    fn dispatch(&self, constants: &[f64], count: usize) -> Result<Vec<f64>, String> {
        let input = self.constants.as_ref().unwrap();
        let output = self.output.as_ref().unwrap();
        unsafe {
            std::ptr::copy_nonoverlapping(
                constants.as_ptr().cast::<u8>(),
                Self::mapped(input)?,
                std::mem::size_of_val(constants),
            );
            self.device
                .flush_mapped_memory_ranges(&[Self::range(input)])
                .map_err(|e| format!("Vulkan flush: {e}"))?;
        }
        let commands = [self.command];
        unsafe {
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
            self.device
                .invalidate_mapped_memory_ranges(&[Self::range(output)])
                .map_err(|e| format!("Vulkan invalidate: {e}"))?;
            Ok(std::slice::from_raw_parts(Self::mapped(output)?.cast::<f64>(), count * 4).to_vec())
        }
    }

    fn free(&mut self, buffer: Buffer) {
        unsafe { self.device.destroy_buffer(buffer.handle, None) };
        let _ = self.allocator.free(buffer.allocation);
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
        }
        if let Some(buffer) = self.constants.take() {
            self.free(buffer);
        }
        if let Some(buffer) = self.output.take() {
            self.free(buffer);
        }
        unsafe {
            if self.pipeline != vk::Pipeline::null() {
                self.device.destroy_pipeline(self.pipeline, None);
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
            if self.pool != vk::CommandPool::null() {
                self.device.destroy_command_pool(self.pool, None);
            }
            ManuallyDrop::drop(&mut self.allocator);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

/// Probe the FP64 Vulkan stack; unavailable devices are not correctness failures.
#[cfg(test)]
fn probe() -> Result<String, String> {
    Ok(Gpu::new(1, 32)?.name.clone())
}

/// Sample all columns on the FP64 compute device.
pub(crate) fn render(view: &View, p: &Params, zone: &Zone) -> Result<(Header, Samples), String> {
    let start = Instant::now();
    let (header, constants) = zone.gpu_f64_input(view, p)?;
    let prep = start.elapsed();
    let count = (p.nx as usize)
        .checked_mul(p.ny as usize)
        .ok_or("GPU grid overflow")?;
    let gpu = Gpu::new(count, constants.len())?;
    let init = start.elapsed() - prep;
    let t = Instant::now();
    let raw = gpu.dispatch(&constants, count)?;
    let execute = t.elapsed();
    let t = Instant::now();
    let cols = p.columns.with(Column::Class);
    let mut samples = Samples::alloc(count, cols);
    for i in 0..count {
        let (nu, de, angle, kind) = (raw[4 * i], raw[4 * i + 1], raw[4 * i + 2], raw[4 * i + 3]);
        let kind = match kind as u32 {
            0 => Kind::Escaped,
            1 => Kind::Interior,
            2 => Kind::Unresolved,
            _ => return Err(format!("GPU returned invalid class for pixel {i}")),
        };
        if kind == Kind::Escaped && !(nu.is_finite() && de.is_finite() && angle.is_finite()) {
            return Err(format!("GPU returned non-finite columns at pixel {i}"));
        }
        samples.class[i] = Class::new(kind, Evidence::Heuristic);
        if let Some(col) = &mut samples.nu {
            col[i] = nu;
        }
        if let Some(col) = &mut samples.de {
            col[i] = de as f32;
        }
        if let Some(col) = &mut samples.normal {
            col[i] = (angle * (65536.0 / std::f64::consts::TAU)).round() as i64 as u16;
        }
    }
    let unpack = t.elapsed();
    eprintln!(
        "fd: Vulkan FP64 zone on {}: constants {:.3} ms, setup {:.3} ms, dispatch/readback {:.3} ms, unpack {:.3} ms, whole-frame {:.3} ms",
        gpu.name,
        prep.as_secs_f64()*1e3,
        init.as_secs_f64()*1e3,
        execute.as_secs_f64()*1e3,
        unpack.as_secs_f64()*1e3,
        start.elapsed().as_secs_f64()*1e3,
    );
    Ok((header, samples))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_samples::Column;
    use std::time::Instant;

    #[test]
    fn gpu_zone_f64_parity() {
        match probe() {
            Ok(name) => eprintln!("GPU-03 Vulkan FP64 device: {name}"),
            Err(e)
                if e.contains("no Vulkan")
                    || e.contains("Vulkan loader")
                    || e.contains("Vulkan instance")
                    || e.contains("Vulkan enumerate") =>
            {
                eprintln!("GPU-03 SKIP: {e}");
                return;
            }
            Err(e) => panic!("GPU-03 Vulkan init failed: {e}"),
        }
        let zone = Zone::parse(include_str!("../../../bench/zones/v0-core.zone")).unwrap();
        let centre_re =
            "-0.743291890852430202931624158508904039462544013087723088341335644672284698593566";
        let centre_im =
            "0.131240552308797604770845873815964848019374249266625134372668873249132305318161";
        let p = Params {
            nx: 480,
            ny: 270,
            ss: 1,
            max_iter: 20_000,
            escape_radius: 1e10,
            columns: fd_samples::ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
            threads: 4,
            tier: None,
        };
        for width in ["1e-35", "1e-38", "1e-40", "1e-43", "1e-46", "2e-48"] {
            let view = View {
                center_re: centre_re.into(),
                center_im: centre_im.into(),
                width: width.into(),
                rotation: 0.0,
            };
            let t = Instant::now();
            let (_, cpu, _, _) = fd_kernel::render_zone(&view, &p, &zone).unwrap();
            let cpu_ms = t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            let (_, gpu) = render(&view, &p, &zone).unwrap();
            let gpu_ms = t.elapsed().as_secs_f64() * 1e3;
            let (a, b) = (cpu.nu.as_ref().unwrap(), gpu.nu.as_ref().unwrap());
            let (ad, bd) = (cpu.de.as_ref().unwrap(), gpu.de.as_ref().unwrap());
            let (an, bn) = (cpu.normal.as_ref().unwrap(), gpu.normal.as_ref().unwrap());
            let mut maxima = [0.0f64; 3];
            for i in 0..cpu.class.len() {
                assert_eq!(
                    cpu.class[i].kind(),
                    gpu.class[i].kind(),
                    "{width}: class mismatch at ({},{})",
                    i % p.nx as usize,
                    i / p.nx as usize
                );
                if !cpu.class[i].escaped() {
                    continue;
                }
                let de = f64::from(ad[i]);
                let errors = [
                    (a[i] - b[i]).abs() * std::f64::consts::LN_2 * de / 2.0,
                    (f64::from(ad[i]) - f64::from(bd[i])).abs(),
                    f64::from(an[i].wrapping_sub(bn[i]).min(bn[i].wrapping_sub(an[i])))
                        * std::f64::consts::TAU
                        / 65536.0
                        * de,
                ];
                for c in 0..3 {
                    maxima[c] = maxima[c].max(errors[c]);
                    assert!(
                        errors[c].is_finite() && errors[c] <= 1e-3,
                        "{width}: pixel {i}, column {c}, error {} px (CPU/GPU DE {} / {})",
                        errors[c],
                        ad[i],
                        bd[i]
                    );
                }
            }
            eprintln!("GPU-03 width {width}: CPU {cpu_ms:.2} ms/frame, GPU {gpu_ms:.2} ms/frame; max nu/de/normal px = {maxima:?}");
        }
    }
}
