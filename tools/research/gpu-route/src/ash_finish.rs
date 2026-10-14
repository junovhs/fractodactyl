//! Existing GLSL finish body on ash, fed the identical captured checkpoints.
//! No shader math is reimplemented: replace only the original shader's main.
use ash::{Device, Entry, Instance, vk};
use cubecl::prelude::*;
use std::{
    ffi::CStr,
    io::Write,
    process::{Command, Stdio},
};

struct Buffer {
    device: Device,
    handle: vk::Buffer,
    memory: vk::DeviceMemory,
    size: usize,
}
impl Buffer {
    unsafe fn new(
        device: &Device,
        properties: &vk::PhysicalDeviceMemoryProperties,
        size: usize,
    ) -> Result<Self, String> {
        let handle = unsafe {
            device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(size as u64)
                    .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )
        }
        .map_err(|e| e.to_string())?;
        let requirements = unsafe { device.get_buffer_memory_requirements(handle) };
        let index = (0..properties.memory_type_count).find(|&i| {
            requirements.memory_type_bits & (1 << i) != 0
                && properties.memory_types[i as usize].property_flags.contains(
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )
        });
        let Some(index) = index else {
            unsafe {
                device.destroy_buffer(handle, None);
            }
            return Err("ash smoke needs coherent host-visible storage".into());
        };
        let memory = match unsafe {
            device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(index),
                None,
            )
        } {
            Ok(m) => m,
            Err(e) => {
                unsafe {
                    device.destroy_buffer(handle, None);
                }
                return Err(e.to_string());
            }
        };
        if let Err(e) = unsafe { device.bind_buffer_memory(handle, memory, 0) } {
            unsafe {
                device.destroy_buffer(handle, None);
                device.free_memory(memory, None);
            }
            return Err(e.to_string());
        }
        Ok(Self {
            device: device.clone(),
            handle,
            memory,
            size,
        })
    }
    fn write(&self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() != self.size {
            return Err("ash input byte length mismatch".into());
        }
        let ptr = unsafe {
            self.device.map_memory(
                self.memory,
                0,
                self.size as u64,
                vk::MemoryMapFlags::empty(),
            )
        }
        .map_err(|e| e.to_string())?;
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast(), bytes.len());
            self.device.unmap_memory(self.memory);
        }
        Ok(())
    }
    fn read(&self) -> Result<Vec<u8>, String> {
        let ptr = unsafe {
            self.device.map_memory(
                self.memory,
                0,
                self.size as u64,
                vk::MemoryMapFlags::empty(),
            )
        }
        .map_err(|e| e.to_string())?;
        let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), self.size) }.to_vec();
        unsafe {
            self.device.unmap_memory(self.memory);
        }
        Ok(bytes)
    }
}
impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.handle, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

fn compile(mixed: bool) -> Result<Vec<u32>, String> {
    let original = include_str!("../../../../crates/fd-cli/shaders/zone_f64.comp");
    let (body, _) = original
        .split_once("void main() {")
        .ok_or("original GLSL main marker changed")?;
    let source = format!(
        "{body}\nlayout(std430, binding=4) readonly buffer Checkpoints {{ double v[]; }} states;\nlayout(std430, binding=5) readonly buffer Iterations {{ uint v[]; }} iterations;\nvoid main() {{ int gid=int(gl_GlobalInvocationID.x); if (gid>=iterations.v.length()) return; int at=gid*4; finish(gid,dvec2(states.v[at],states.v[at+1]),dvec2(states.v[at+2],states.v[at+3]),int(iterations.v[gid]),0.0lf); }}\n"
    );
    let compiler = std::env::var_os("FD_GPU_ROUTE_GLSLC").unwrap_or_else(|| "glslc".into());
    let mut command = Command::new(compiler);
    if mixed {
        command.arg("-DZONE_F32_FINISH=1");
    }
    let mut child = command
        .args([
            "-x",
            "glsl",
            "-fshader-stage=compute",
            "-O",
            "--target-env=vulkan1.2",
            "-o",
            "-",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("GLSL compiler (set FD_GPU_ROUTE_GLSLC): {e}"))?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "original finish shader compile failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if output.stdout.len() % 4 != 0 {
        return Err("invalid SPIR-V byte count".into());
    }
    Ok(output
        .stdout
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect())
}

pub struct Runner {
    _entry: Entry,
    instance: Instance,
    device: Device,
    properties: vk::PhysicalDeviceMemoryProperties,
    queue: vk::Queue,
    pool: vk::CommandPool,
    descriptor_pool: vk::DescriptorPool,
    set_layout: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    set: vk::DescriptorSet,
    command: vk::CommandBuffer,
    query_pool: vk::QueryPool,
    timestamp_period: f64,
    timestamp_bits: u32,
}
impl Runner {
    pub fn new(expected_adapter: &str, mixed: bool) -> Result<Self, String> {
        let spirv = compile(mixed)?;
        unsafe {
            let entry = Entry::load().map_err(|e| e.to_string())?;
            let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_2);
            let instance = entry
                .create_instance(
                    &vk::InstanceCreateInfo::default().application_info(&app),
                    None,
                )
                .map_err(|e| e.to_string())?;
            let selected = instance
                .enumerate_physical_devices()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find_map(|physical| {
                    let props = instance.get_physical_device_properties(physical);
                    let name = CStr::from_ptr(props.device_name.as_ptr()).to_string_lossy();
                    if name != expected_adapter
                        || instance
                            .get_physical_device_features(physical)
                            .shader_float64
                            == vk::FALSE
                    {
                        return None;
                    }
                    if mixed {
                        let mut subgroup = vk::PhysicalDeviceSubgroupProperties::default();
                        let mut properties =
                            vk::PhysicalDeviceProperties2::default().push_next(&mut subgroup);
                        instance.get_physical_device_properties2(physical, &mut properties);
                        if !subgroup
                            .supported_operations
                            .contains(vk::SubgroupFeatureFlags::VOTE)
                            || !subgroup
                                .supported_stages
                                .contains(vk::ShaderStageFlags::COMPUTE)
                        {
                            return None;
                        }
                    }
                    instance
                        .get_physical_device_queue_family_properties(physical)
                        .iter()
                        .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
                        .map(|q| (physical, q as u32))
                });
            let Some((physical, family)) = selected else {
                instance.destroy_instance(None);
                return Err("ash cannot find the same f64 hardware adapter".into());
            };
            let queues = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family)
                .queue_priorities(&[1.0])];
            let features = vk::PhysicalDeviceFeatures::default().shader_float64(true);
            let device_result = instance.create_device(
                physical,
                &vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queues)
                    .enabled_features(&features),
                None,
            );
            let device = match device_result {
                Ok(device) => device,
                Err(e) => {
                    instance.destroy_instance(None);
                    return Err(e.to_string());
                }
            };
            let timestamp_period = f64::from(
                instance
                    .get_physical_device_properties(physical)
                    .limits
                    .timestamp_period,
            );
            let timestamp_bits = instance.get_physical_device_queue_family_properties(physical)
                [family as usize]
                .timestamp_valid_bits;
            let mut runner = Self {
                properties: instance.get_physical_device_memory_properties(physical),
                queue: device.get_device_queue(family, 0),
                _entry: entry,
                instance,
                device: device.clone(),
                pool: vk::CommandPool::null(),
                descriptor_pool: vk::DescriptorPool::null(),
                set_layout: vk::DescriptorSetLayout::null(),
                layout: vk::PipelineLayout::null(),
                pipeline: vk::Pipeline::null(),
                set: vk::DescriptorSet::null(),
                command: vk::CommandBuffer::null(),
                query_pool: vk::QueryPool::null(),
                timestamp_period,
                timestamp_bits,
            };
            let bindings = (0..6)
                .map(|i| {
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(i)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::COMPUTE)
                })
                .collect::<Vec<_>>();
            runner.set_layout = device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                    None,
                )
                .map_err(|e| e.to_string())?;
            let layouts = [runner.set_layout];
            runner.layout = device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts),
                    None,
                )
                .map_err(|e| e.to_string())?;
            let module = device
                .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&spirv), None)
                .map_err(|e| e.to_string())?;
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(module)
                .name(c"main");
            let pipelines = device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[vk::ComputePipelineCreateInfo::default()
                    .stage(stage)
                    .layout(runner.layout)],
                None,
            );
            device.destroy_shader_module(module, None);
            runner.pipeline = match pipelines {
                Ok(pipelines) => pipelines[0],
                Err((pipelines, e)) => {
                    for pipeline in pipelines {
                        device.destroy_pipeline(pipeline, None);
                    }
                    return Err(e.to_string());
                }
            };
            runner.pool = device
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::default().queue_family_index(family),
                    None,
                )
                .map_err(|e| e.to_string())?;
            runner.command = device
                .allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::default()
                        .command_pool(runner.pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1),
                )
                .map_err(|e| e.to_string())?[0];
            let sizes = [vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(6)];
            runner.descriptor_pool = device
                .create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default()
                        .max_sets(1)
                        .pool_sizes(&sizes),
                    None,
                )
                .map_err(|e| e.to_string())?;
            runner.set = device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(runner.descriptor_pool)
                        .set_layouts(&layouts),
                )
                .map_err(|e| e.to_string())?[0];
            if runner.timestamp_bits > 0 {
                runner.query_pool = device
                    .create_query_pool(
                        &vk::QueryPoolCreateInfo::default()
                            .query_type(vk::QueryType::TIMESTAMP)
                            .query_count(2),
                        None,
                    )
                    .map_err(|e| e.to_string())?;
            }
            Ok(runner)
        }
    }

    pub fn run_measured(
        &self,
        input: &[f64],
        iterations: &[u32],
        camera: &[f64],
        timing: bool,
    ) -> Result<super::columns::Measured, String> {
        let started = std::time::Instant::now();
        let count = iterations.len();
        if count == 0 || count > 1920 * 1080 || input.len() != count * 4 || camera.len() != 6 {
            return Err("ash smoke needs 1..144 checkpoints and pixel/cos/sin camera".into());
        }
        let mut constants = vec![0.0; 32];
        constants[5] = 20_000.0;
        constants[11] = camera[3];
        constants[12] = camera[4];
        constants[21] = camera[5];
        constants[28] = camera[1];
        constants[29] = camera[2];
        constants[30] = camera[0];
        let states = input;
        unsafe {
            let sizes = [
                constants.len() * 8,
                count * 8,
                count * 4,
                count * 4,
                states.len() * 8,
                count * 4,
            ];
            let buffers = sizes
                .into_iter()
                .map(|size| Buffer::new(&self.device, &self.properties, size))
                .collect::<Result<Vec<_>, _>>()?;
            buffers[0].write(f64::as_bytes(&constants))?;
            buffers[4].write(f64::as_bytes(&states))?;
            buffers[5].write(u32::as_bytes(iterations))?;
            let infos = buffers
                .iter()
                .map(|b| {
                    [vk::DescriptorBufferInfo::default()
                        .buffer(b.handle)
                        .range(b.size as u64)]
                })
                .collect::<Vec<_>>();
            let writes = infos
                .iter()
                .enumerate()
                .map(|(i, info)| {
                    vk::WriteDescriptorSet::default()
                        .dst_set(self.set)
                        .dst_binding(i as u32)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(info)
                })
                .collect::<Vec<_>>();
            self.device.update_descriptor_sets(&writes, &[]);
            self.device
                .reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
                .map_err(|e| e.to_string())?;
            self.device
                .begin_command_buffer(self.command, &vk::CommandBufferBeginInfo::default())
                .map_err(|e| e.to_string())?;
            if timing && self.timestamp_bits > 0 {
                self.device
                    .cmd_reset_query_pool(self.command, self.query_pool, 0, 2);
                self.device.cmd_write_timestamp(
                    self.command,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    self.query_pool,
                    0,
                );
            }
            self.device.cmd_bind_pipeline(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline,
            );
            self.device.cmd_bind_descriptor_sets(
                self.command,
                vk::PipelineBindPoint::COMPUTE,
                self.layout,
                0,
                &[self.set],
                &[],
            );
            self.device
                .cmd_dispatch(self.command, (count as u32).div_ceil(64), 1, 1);
            if timing && self.timestamp_bits > 0 {
                self.device.cmd_write_timestamp(
                    self.command,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    self.query_pool,
                    1,
                );
            }
            self.device.cmd_pipeline_barrier(
                self.command,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[vk::MemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                    .dst_access_mask(vk::AccessFlags::HOST_READ)],
                &[],
                &[],
            );
            self.device
                .end_command_buffer(self.command)
                .map_err(|e| e.to_string())?;
            let fence = self
                .device
                .create_fence(&Default::default(), None)
                .map_err(|e| e.to_string())?;
            let commands = [self.command];
            let submit = self.device.queue_submit(
                self.queue,
                &[vk::SubmitInfo::default().command_buffers(&commands)],
                fence,
            );
            let waited =
                submit.and_then(|_| self.device.wait_for_fences(&[fence], true, 10_000_000_000));
            if waited.is_err() {
                let _ = self.device.device_wait_idle();
            }
            self.device.destroy_fence(fence, None);
            waited.map_err(|e| format!("ash submit/wait: {e}"))?;
            let nu = buffers[1].read()?;
            let de = buffers[2].read()?;
            let packed = buffers[3].read()?;
            let frame_ms = started.elapsed().as_secs_f64() * 1000.0;
            let gpu_ms = if timing && self.timestamp_bits > 0 {
                let mut ticks = [0u64; 2];
                self.device
                    .get_query_pool_results(
                        self.query_pool,
                        0,
                        &mut ticks,
                        vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                    )
                    .map_err(|e| e.to_string())?;
                let mask = if self.timestamp_bits == 64 {
                    u64::MAX
                } else {
                    (1u64 << self.timestamp_bits) - 1
                };
                Some((ticks[1].wrapping_sub(ticks[0]) & mask) as f64 * self.timestamp_period / 1e6)
            } else {
                None
            };
            let (terminal, words) = super::columns::decoded(&nu, &de, &packed)?;
            Ok(super::columns::Measured {
                terminal,
                words,
                gpu_ms,
                frame_ms,
            })
        }
    }
    pub fn run(
        &self,
        input: &[f64],
        iterations: &[u32],
        camera: &[f64],
    ) -> Result<(Vec<f64>, Vec<u32>), String> {
        self.run_measured(input, iterations, camera, false)
            .map(|r| (r.terminal, r.words))
    }
}
impl Drop for Runner {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_query_pool(self.query_pool, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.set_layout, None);
            self.device.destroy_command_pool(self.pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
