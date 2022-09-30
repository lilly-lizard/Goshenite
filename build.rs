#[cfg(feature = "shader-compile")]
use shaderc::{CompileOptions, Compiler, IncludeType, ResolvedInclude, ShaderKind};
#[cfg(feature = "shader-compile")]
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

/// Compile glsl shaders in src/shaders and output spirv binaries to assets/shader_binares.
/// Requirements:
/// - Entry point must be "main".
/// - File extensions must be in the format FILE_NAME.SHADER_STAGE
/// If you install the shaderc libraries on your system you can avoid compiling them in
/// your rust builds. See https://github.com/google/shaderc-rs#setup for more info.
#[cfg(feature = "shader-compile")]
fn gen_shader_spirv() {
    // rerun when shaders change
    println!("cargo:rerun-if-changed=src/shaders/*");

    println!("Generating spirv shaders...");

    // shader source directory
    let shader_dir = get_shader_dir();

    // output spirv directory {source_root}/assets/shader_binaries
    let spirv_dir = get_spirv_dir();
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

        // no more `continue`s
        println!("Compiling {:?}...", file_name);

        // read shader source
        let shader_text =
            read_shader(&shader_path).expect(&format!("failed to read shader file {}", file_name));

        // how to handle included files
        let mut preprocess_options =
            CompileOptions::new().expect("error initializing compile options object");
        preprocess_options.set_include_callback(
            |included_file_name: &str,
             _include_type: IncludeType,
             includer_file_name: &str,
             _include_depth: usize| {
                // determine included file path
                let mut path = get_shader_dir();
                path.push(included_file_name);
                // read included file
                let shader_text = match read_shader(&path) {
                    Ok(x) => x,
                    Err(e) => {
                        return Err(format!(
                            "failed to read shader file {} included in {}, due to error: {}",
                            includer_file_name, included_file_name, e
                        ))
                    }
                };
                // return included source
                Ok(ResolvedInclude {
                    resolved_name: path.to_str().expect("invalid unicode").into(),
                    content: shader_text,
                })
            },
        );

        // preprocess source to support including other shader files
        let entry_point = "main";
        let prep = match compiler.preprocess(
            &shader_text,
            &file_name,
            &entry_point,
            Some(&preprocess_options),
        ) {
            Ok(prep) => prep,
            Err(e) => panic!("failed to preprocess {}:\n{}", file_name, e),
        };

        // compile to spirv
        let spirv_comp = match compiler.compile_into_spirv(
            &prep.as_text(),
            shader_stage,
            &file_name,
            &entry_point,
            None,
        ) {
            Ok(comp) => comp,
            Err(e) => panic!("failed to compile {} to spirv:\n{}", file_name, e),
            // alternatively just warn and carry on:
            //println!("cargo:warning=failed to compile {} to spirv:\n{}", file_name, e);
            //continue;
        };
        let spirv_bin = spirv_comp.as_binary_u8();

        // write spirv to file
        let mut spirv_path = spirv_dir.clone();
        spirv_path.push(file_name + ".spv");
        let mut spirv_file =
            File::create(spirv_path).expect("failed to open spirv file for writing");
        spirv_file
            .write_all(spirv_bin)
            .expect("failed to write spirv data to output file");
    }
}

/// Attempts to read the source file at `shader_path` and returns its contents as a String
#[cfg(feature = "shader-compile")]
fn read_shader(shader_path: &PathBuf) -> std::io::Result<String> {
    let mut shader_text = String::new();
    let mut shader_file = File::open(shader_path)?;
    shader_file.read_to_string(&mut shader_text)?;
    Ok(shader_text)
}

/// Returns the directory containing the shader source files
#[cfg(feature = "shader-compile")]
fn get_shader_dir() -> PathBuf {
    let mut shader_dir = std::env::current_dir().expect("cannot access pwd");
    shader_dir.push("src");
    shader_dir.push("shaders");
    shader_dir
}

/// Returns the directory to output spirv binaries
#[cfg(feature = "shader-compile")]
fn get_spirv_dir() -> PathBuf {
    let mut spirv_dir = std::env::current_dir().expect("cannot access pwd");
    spirv_dir.push("assets");
    spirv_dir.push("shader_binaries");
    spirv_dir
}

fn main() {
    #[cfg(feature = "shader-compile")]
    gen_shader_spirv();
}
