use std::{env, fs, path::Path};

fn main() {
    let out = env::var("OUT_DIR").expect("OUT_DIR");
    let compiler = shaderc::Compiler::new().expect("shaderc compiler");
    for name in [
        "film_prepare",
        "film_shade",
        "film_resample",
        "zone_f64",
        "zone_f32",
        "zone_mixed",
    ] {
        let mut options = shaderc::CompileOptions::new().expect("shaderc options");
        options.set_target_env(
            shaderc::TargetEnv::Vulkan,
            shaderc::EnvVersion::Vulkan1_2 as u32,
        );
        if name == "zone_f32" {
            options.add_macro_definition("ZONE_F32_FRONT", None);
        }
        if name == "zone_f32" || name == "zone_mixed" {
            options.add_macro_definition("ZONE_F32_FINISH", None);
        }
        let source_name = if name.starts_with("zone_") {
            "zone_f64"
        } else {
            name
        };
        let input = format!("shaders/{source_name}.comp");
        println!("cargo:rerun-if-changed={input}");
        let source = fs::read_to_string(&input).expect("shader source");
        let binary = compiler
            .compile_into_spirv(
                &source,
                shaderc::ShaderKind::Compute,
                &input,
                "main",
                Some(&options),
            )
            .expect("compile film shader");
        fs::write(
            Path::new(&out).join(format!("{name}.spv")),
            binary.as_binary_u8(),
        )
        .expect("write SPIR-V");
    }
}
