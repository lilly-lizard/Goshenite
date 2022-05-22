use shaderc::{Compiler, ShaderKind};
use std::fs::File;
use std::io::{Read, Write};

/// Compile glsl shaders in src/shaders and output spirv binaries to assets/shader_binares.
/// Requirements:
/// - Entry point must be "main".
/// - File extensions must be in the format FILE_NAME.SHADER_STAGE
fn gen_shader_spirv() {
    println!("Generating spirv shaders...");

    // shader source directory
    let mut shader_dir = std::env::current_dir().expect("shouldn't panic: pwd must exist");
    shader_dir.push("src");
    shader_dir.push("shaders");

    // output spirv directory
    let mut spirv_dir = std::env::current_dir().expect("shouldn't panic: pwd must exist");
    spirv_dir.push("assets");
    spirv_dir.push("shader_binaries");
    assert!(spirv_dir.is_dir(), "invalid spirv destination path");

    // spirv compiler
    let compiler = Compiler::new().expect("failed to initialize shaderc compiler");

    // iterate over files in shaders directory
    for dir_entry in std::fs::read_dir(shader_dir).expect("invalid shader source path") {
        let dir_entry = dir_entry.expect("fs::ReadDir io error during iteration");
        let shader_path = dir_entry.path();

        // file name (skips if it contains invalid utf-8)
        let file_name = match dir_entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };

        // determine shader type
        let file_ext = match shader_path.extension() {
            Some(e) => e,
            None => continue,
        }
        .to_str()
        .expect("shouldn't panic: already done the utf check on file_name");
        let shader_stage = match file_ext {
            "vert" => ShaderKind::Vertex,
            "frag" => ShaderKind::Fragment,
            "comp" => ShaderKind::Compute,
            "geom" => ShaderKind::Geometry,
            "tesc" => ShaderKind::TessControl,
            "tese" => ShaderKind::TessEvaluation,
            "mesh" => ShaderKind::Mesh,
            "task" => ShaderKind::Task,
            "rgen" => ShaderKind::RayGeneration,
            "rint" => ShaderKind::Intersection,
            "rahit" => ShaderKind::AnyHit,
            "rchit" => ShaderKind::ClosestHit,
            "rmiss" => ShaderKind::Miss,
            "rcall" => ShaderKind::Callable,
            _ => continue,
        };

        // no more 'continue's
        println!("Compiling {:?}...", file_name);

        // read shader source
        let mut shader_text = String::new();
        let mut shader_file =
            File::open(&shader_path).expect("failed to open shader source file for reading");
        shader_file
            .read_to_string(&mut shader_text)
            .expect("invalid utf-8 in shader source code");

        // compile spirv
        let spirv_comp =
            match compiler.compile_into_spirv(&shader_text, shader_stage, &file_name, "main", None)
            {
                Ok(comp) => comp,
                Err(e) => panic!("failed to compile {} to spirv:\n{}", file_name, e),
            };
        let spirv_bin = spirv_comp.as_binary_u8();

        // write spirv to file
        let mut spirv_path = spirv_dir.clone();
        spirv_path.push(file_name + ".spv");
        let mut spirv_file =
            File::create(spirv_path).expect("failed to open spirv file for writing");
        spirv_file
            .write_all(&spirv_bin)
            .expect("failed to write spirv data to output file");
    }
}

fn main() {
    gen_shader_spirv();

    // rerun when shaders change
    println!("cargo:rerun-if-changed=src/shaders/*");
}
