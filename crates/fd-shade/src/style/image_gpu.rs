//! Executes the compiled image DAG with one shared kernel and liveness-planned buffers.
use super::{
    images::{Kind, Plan},
    render::{buffer, pipeline},
};
use std::num::NonZeroU64;

struct Dispatch {
    group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
}
pub(super) struct ImagesGpu {
    pub pipelines: u32,
    dispatches: Vec<Dispatch>,
    buffers: Vec<wgpu::Buffer>,
    pub bytes: u64,
    pub allocations: u32,
    pub plan: Plan,
    layout: Vec<(u64, u64)>,
    image_bytes: u64,
}
impl ImagesGpu {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: &wgpu::Buffer,
        plan: Plan,
        w: usize,
        h: usize,
    ) -> Self {
        let shared = plan.steps.iter().any(|s| s.fused.is_none()).then(|| {
            pipeline(
                device,
                "style shared spatial operations",
                include_str!("image_ops.wgsl"),
                "apply",
            )
        });
        let mut pipelines = u32::from(shared.is_some());
        let image_bytes = w as u64 * h as u64 * 16;
        let (layout, _) = plan.layout(
            device.limits().min_storage_buffer_offset_alignment as u64,
            w,
            h,
        );
        let buffers: Vec<_> = (0..plan.slots)
            .map(|_| {
                buffer(
                    device,
                    "style reused image slot",
                    image_bytes,
                    wgpu::BufferUsages::STORAGE,
                )
            })
            .collect();
        let resource = |id: usize| {
            let image = &plan.images[id];
            let (buffer, offset) = if let Some(source) = image.source {
                (atlas, layout[source].0)
            } else {
                (&buffers[image.slot], 0)
            };
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset,
                size: NonZeroU64::new(image.source.map_or(image_bytes, |s| layout[s].1)),
            })
        };
        let mut dispatches = Vec::new();
        for step in &plan.steps {
            let kind = |id: usize| match plan.images[id].kind {
                Kind::Scalar => 0,
                Kind::Vector => 1,
                Kind::Color => 2,
            };
            let params = [
                w as u32,
                h as u32,
                step.op,
                0,
                kind(step.inputs[0]),
                kind(step.inputs[1]),
                kind(step.inputs[2]),
                step.inputs.iter().enumerate().fold(0u32, |bits, (i, &id)| {
                    bits | if plan.images[id]
                        .source
                        .is_some_and(|s| !plan.sources[s].spatial)
                    {
                        1u32 << i
                    } else {
                        0
                    }
                }),
            ];
            let uniform = buffer(
                device,
                "style immutable image pass",
                32,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            );
            queue.write_buffer(&uniform, 0, &crate::gpu::testkit::bytes(&params));
            let (pipeline, entries) = if let Some(fused) = &step.fused {
                pipelines += 1;
                let pipeline = pipeline(
                    device,
                    "style fused image expressions",
                    &plan.fused_shader(fused),
                    "apply",
                );
                let mut entries: Vec<_> = fused
                    .inputs
                    .iter()
                    .enumerate()
                    .map(|(i, &id)| wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: resource(id),
                    })
                    .collect();
                entries.push(wgpu::BindGroupEntry {
                    binding: fused.inputs.len() as u32,
                    resource: resource(step.output),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: fused.inputs.len() as u32 + 1,
                    resource: uniform.as_entire_binding(),
                });
                (pipeline, entries)
            } else {
                let pipeline = shared.as_ref().unwrap().clone();
                let entries = vec![
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: resource(step.inputs[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: resource(step.inputs[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: resource(step.inputs[2]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: resource(step.output),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: uniform.as_entire_binding(),
                    },
                ];
                (pipeline, entries)
            };
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("style connected image pass"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            });
            dispatches.push(Dispatch { group, pipeline });
        }
        let bytes = plan.slots as u64 * image_bytes + plan.steps.len() as u64 * 32;
        let allocations = (plan.slots + plan.steps.len()) as u32;
        Self {
            pipelines,
            dispatches,
            buffers,
            bytes,
            allocations,
            plan,
            layout,
            image_bytes,
        }
    }
    pub fn finish_group(
        &self,
        device: &wgpu::Device,
        finish: &wgpu::ComputePipeline,
        atlas: &wgpu::Buffer,
        output: &wgpu::Buffer,
        uniform: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let image = &self.plan.images[self.plan.output];
        let (buffer, offset) = if let Some(source) = image.source {
            (atlas, self.layout[source].0)
        } else {
            (&self.buffers[image.slot], 0)
        };
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("style graph output"),
            layout: &finish.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset,
                        size: NonZeroU64::new(
                            image.source.map_or(self.image_bytes, |s| self.layout[s].1),
                        ),
                    }),
                },
            ],
        })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder, w: u32, h: u32) -> u32 {
        for dispatch in &self.dispatches {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&dispatch.pipeline);
            pass.set_bind_group(0, &dispatch.group, &[]);
            pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
        }
        self.dispatches.len() as u32
    }
}
