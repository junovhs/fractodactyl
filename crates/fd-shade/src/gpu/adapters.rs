//! GPU-30: stable indices shared by zone and studio, including viewer FD_ADAPTER.
use std::sync::OnceLock;

static CHOICE: OnceLock<usize> = OnceLock::new();

/// Override FD_ADAPTER before opening any device (CLI flag takes precedence).
pub fn select(index: usize) -> Result<(), String> {
    selected(&enumerate(), index)?;
    CHOICE
        .set(index)
        .map_err(|_| "adapter already selected".to_string())
}

/// Explicit process choice, else the viewer/child-process environment choice.
pub fn choice() -> Result<Option<usize>, String> {
    if let Some(index) = CHOICE.get() {
        return Ok(Some(*index));
    }
    match std::env::var("FD_ADAPTER") {
        Ok(value) => value.parse().map(Some).map_err(|_| {
            format!("FD_ADAPTER: expected a non-negative adapter index, got {value:?}")
        }),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => Err("FD_ADAPTER must be a non-negative adapter index".into()),
    }
}

/// Enumerate every backend considered by fd, with deterministic identity ordering.
/// Ranking remains the responsibility of each caller when no choice is supplied.
pub fn native() -> Vec<wgpu::Adapter> {
    let backends = wgpu::Backends::VULKAN | wgpu::Backends::DX12;
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::StaticDxc.with_env();
    let instance = wgpu::Instance::new(descriptor);
    pollster::block_on(instance.enumerate_adapters(backends))
}

/// Inventory order used by public indices, independent of capability filtering.
pub fn enumerate() -> Vec<wgpu::Adapter> {
    let mut adapters = native();
    adapters.sort_by_key(|a| {
        let i = a.get_info();
        (format!("{:?}", i.backend), i.vendor, i.device, i.name)
    });
    adapters
}

/// Validate a public index and include usable choices in errors.
pub fn selected(adapters: &[wgpu::Adapter], index: usize) -> Result<&wgpu::Adapter, String> {
    adapters.get(index).ok_or_else(|| {
        format!(
            "adapter {index} is invalid; valid indices: {}",
            if adapters.is_empty() {
                "none (no wgpu adapters)".into()
            } else {
                adapters
                    .iter()
                    .enumerate()
                    .map(|(n, a)| format!("{n} ({})", a.get_info().name))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
    })
}

/// User-facing inventory rows; f64 reports the native SHADER_F64 feature.
pub fn rows() -> Vec<String> {
    enumerate()
        .iter()
        .enumerate()
        .map(|(n, a)| {
            let i = a.get_info();
            format!(
                "{n} {:?} {:04x}:{:04x} {} f64={}",
                i.backend,
                i.vendor,
                i.device,
                i.name,
                a.features().contains(wgpu::Features::SHADER_F64)
            )
        })
        .collect()
}
