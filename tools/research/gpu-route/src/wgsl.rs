//! Hand-written WGSL recurrence arm; validation failures are errors, never passes.
use cubecl::prelude::*;

pub struct Runner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
}

impl Runner {
    pub fn new(expected_adapter: &str, mixed: bool) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = cubecl::future::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
            .into_iter()
            .find(|a| {
                a.get_info().name == expected_adapter
                    && a.get_info().device_type != wgpu::DeviceType::Cpu
            })
            .ok_or("WGSL arm cannot find the same hardware as CubeCL")?;
        let mut required = wgpu::Features::SHADER_F64
            | wgpu::Features::SHADER_INT64
            | wgpu::Features::TIMESTAMP_QUERY;
        if mixed {
            required |= wgpu::Features::SUBGROUP;
        }
        if !adapter.features().contains(required) {
            return Err(
                "WGSL f64 encoding arm ineligible: SHADER_F64/SHADER_INT64 unavailable".into(),
            );
        }
        let (device, queue) =
            cubecl::future::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                required_features: required,
                ..Default::default()
            }))
            .map_err(|e| format!("WGSL device: {e}"))?;
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut source = include_str!("finish.wgsl").to_owned();
        if mixed {
            source = format!("{source}\n{}", include_str!("mixed.wgsl"));
            source = source.replace("var cls = 2u;", "var cls = 2u; var try32 = true;");
            source = source.replacen("while n < 20000u {", r#"while n < 20000u {
                if try32 && (n & 3u) == 0u {
                    let ready = (dr * dr + di * di) * camera[0] * camera[0] >= f64(0.059604644775390625) * max(zr * zr + zi * zi, f64(1.0));
                    if subgroupAll(ready) {
                        let attempt = try_f32(zr, zi, dr, di, cr, ci, n, escape_sq);
                        if attempt.accepted {
                            zr = attempt.z.x; zi = attempt.z.y; dr = attempt.z.z; di = attempt.z.w; n = attempt.n;
                            if attempt.escaped { cls = 0u; }
                            break;
                        }
                        try32 = false;
                    }
                }
            "#, 1);
        }
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GPU-12 hand-written finish"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GPU-12 hand-written finish"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(e) = cubecl::future::block_on(errors.pop()) {
            return Err(format!("WGSL finish pipeline rejected: {e}"));
        }
        Ok(Self {
            device,
            queue,
            pipeline,
        })
    }

    fn input(&self, bytes: &[u8]) -> Result<wgpu::Buffer, String> {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-12 checkpoints"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: true,
        });
        buffer
            .get_mapped_range_mut(..)
            .map_err(|e| format!("WGSL mapped input: {e}"))?
            .copy_from_slice(bytes);
        buffer.unmap();
        Ok(buffer)
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
        if camera.len() != 6 {
            return Err("expected pixel/cos/sin camera".into());
        }
        if count == 0 || input.len() != count * 4 || count > 1920 * 1080 {
            return Err("WGSL smoke needs 1..144 packed checkpoints".into());
        }
        let errors = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let inputs = self.input(f64::as_bytes(input))?;
        let iterations_buffer = self.input(u32::as_bytes(iterations))?;
        let camera_buffer = self.input(f64::as_bytes(camera))?;
        let terminal_bytes = count * size_of::<f64>();
        let de_bytes = count * size_of::<f32>();
        let word_bytes = count * size_of::<u32>();
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-12 terminal state"),
            size: terminal_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let words = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-12 terminal words"),
            size: word_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let de = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-12 DE"),
            size: de_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let query = self.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("GPU-12 timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let query_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-12 timestamp resolve"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-12 terminal readback"),
            size: (terminal_bytes + de_bytes + word_bytes + if timing { 16 } else { 0 }) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bindings = [
            &inputs,
            &iterations_buffer,
            &output,
            &words,
            &camera_buffer,
            &de,
        ];
        let entries = bindings
            .iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GPU-12 finish"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let timestamps = if timing {
                Some(wgpu::ComputePassTimestampWrites {
                    query_set: &query,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                })
            } else {
                None
            };
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: timestamps,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups((count as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, terminal_bytes as u64);
        encoder.copy_buffer_to_buffer(&de, 0, &staging, terminal_bytes as u64, de_bytes as u64);
        encoder.copy_buffer_to_buffer(
            &words,
            0,
            &staging,
            (terminal_bytes + de_bytes) as u64,
            word_bytes as u64,
        );
        if timing {
            encoder.resolve_query_set(&query, 0..2, &query_buffer, 0);
            encoder.copy_buffer_to_buffer(
                &query_buffer,
                0,
                &staging,
                (terminal_bytes + de_bytes + word_bytes) as u64,
                16,
            );
        }
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = tx.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("WGSL poll: {e}"))?;
        rx.recv()
            .map_err(|e| format!("WGSL callback: {e}"))?
            .map_err(|e| format!("WGSL map: {e}"))?;
        if let Some(e) = cubecl::future::block_on(errors.pop()) {
            return Err(format!("WGSL dispatch rejected: {e}"));
        }
        let bytes = staging
            .get_mapped_range(..)
            .map_err(|e| format!("WGSL mapped output: {e}"))?;
        let frame_ms = started.elapsed().as_secs_f64() * 1000.0;
        let decoded = super::columns::decoded(
            &bytes[..terminal_bytes],
            &bytes[terminal_bytes..terminal_bytes + de_bytes],
            &bytes[terminal_bytes + de_bytes..terminal_bytes + de_bytes + word_bytes],
        );
        let gpu_ms = if timing {
            let at = terminal_bytes + de_bytes + word_bytes;
            let start = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
            let end = u64::from_le_bytes(bytes[at + 8..at + 16].try_into().unwrap());
            Some(
                end.wrapping_sub(start) as f64 * f64::from(self.queue.get_timestamp_period()) / 1e6,
            )
        } else {
            None
        };
        drop(bytes);
        staging.unmap();
        let (terminal, words) = decoded?;
        Ok(super::columns::Measured {
            terminal,
            words,
            gpu_ms,
            frame_ms,
            stages: None,
        })
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
