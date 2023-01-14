# general guidelines

focus on fast iteration! **avoid premature optimization** quick and dirty first.

# big todo

1. objects refactor
	1. object struct - DONE
	2. shaders/buffers
	3. gui/engine
2. organise shaders directory structure and content
3. raster bounding boxes

# todo

- weak references or something for Operations e.g. Union
- switch scene_geometry.frag to hlsl, delete circle (https://alain.xyz/blog/a-review-of-shader-languages, but also https://github.com/KhronosGroup/glslang/wiki/HLSL-FAQ)
- smooth union op (curved combination)
- hemisphere (circle) clamps on looking too far up/down (quaternions?)
- surface noise modifiers
- render outline on selected object
- output render png (write tests using this?)
- save model as file
- curl noise field

## cleanup

- Primitive/Operation -> convert from enum to boxed dyn trait collections. have vecs for each type and collection points to them
- try rust analyzer extract method in render_manager.rs. what would uncle bob do?
- update gui buffers pub fn
- GuiRenderer::create_texture -> create_textures batch texture creation
- gui_renderer unwraps/error handling
- for loops to map
- support/handle VulkanError cases...

## always

- unwrap/except/assert
- clippy

## bugz

- CursorState not initialized properly! e.g. cursor position 0,0 at start so start dragging before moving it and a big jump occurs. also check latest winit in case querying was made better?

## low priority

- smallvecs
- clickable primitives
- coordinate overlay z-buffer
- attempt to restart renderer on error: e.g. SurfaceLost attempt reinitialization. pop-up dialogue "renderer has crashes. attempt re-initialization? report bug here..."
- test anyhow dereferencing e.g. SurfaceSizeUnsupported (see bottom of render_manager.rs)
- preview new primitive, greyed out/transparent until add?
- highlight changed primitive values in gui (to indicate what hasn't been updated)
- error and warn log messages in gui (popups?)
- draw sdf sphere with AABB
- double buffering (2 frames in flight) -> double up futures/per-frame resources, reduce cpu-gpu sync blocking
- Camera::rotate quaternions https://www.3dgep.com/understanding-quaternions/
- shaderStorageImageExtendedFormats
	- https://registry.khronos.org/vulkan/specs/1.3-extensions/html/chap43.html#features-required-format-support
	- VK_FORMAT_A2B10G10R10_UNORM_PACK32 or VK_FORMAT_B10G11R11_UFLOAT_PACK32
	- https://stackoverflow.com/questions/72548476/whats-the-best-practice-for-handling-format-qualifiers-for-images-in-compute-sh
- tests for Primitives data
- if using compute shader, use shader_storage_image_write_without_format

- Bang Wong color palette

### optimize

- decrease MIN_DIST as distance progresses (need less resolution)
- don't recreate buffer pools each frame in geometry_pass.rs

# Code Guidelines

- Consider commenting with structure of 'action' followed by 'object' e.g. 'transition
	image layout (action) for depth buffer (object)'. This makes it easier to search for
comments by action or object e.g. a search for 'transition image layout' wouldn't find
	the comment 'transition depth buffer image layout'
- avoid Box<dyn Error> if possible, just create an enum https://fettblog.eu/rust-enums-wrapping-errors/
- https://rust-lang.github.io/api-guidelines yeet

## logging

- all 'spammy' logging should go in trace, e.g. per-frame states, per-input states, to keep debug and higher reasonably clean and readable

# design decisions

create objects and coloring from editor, set to vary against variables etc
possibilities:
- sequence of primitives, transformations and combinations in storage buffer
	e.g. buffer: Vec<u32> = { num primitives, SPHERE, center, radius, UNION, SPHERE, center, radius... }
- color?
- define uv functions and associate textures
- editor generates shaders. real time feedback?
- live feedback modes e.g. sculpting mode just has primitives and normals
- **world space**: z up; right handed (x forward, y left), camera space: z depth

## ideas

- defer shading to raster pass? render to g-buffer, including shadow info (e.g. bitmap of light sources for primitive?)

# Resources

- HIP instead of vulkan compute? https://github.com/charles-r-earp/hip-sys
- ray marching intro https://michaelwalczyk.com/blog-ray-marching.html
- vulkan format types https://stackoverflow.com/questions/59628956/what-is-the-difference-between-normalized-scaled-and-integer-vkformats
	- format properties https://registry.khronos.org/vulkan/specs/1.3-extensions/html/chap45.html#features-required-format-support

# References

- UX: https://asktog.com/atc/principles-of-interaction-design/

- https://www.shadertoy.com/view/NddSWs
- https://imgur.com/a/YZ3p7Ce

- https://en.wikipedia.org/wiki/T-spline
- https://en.wikipedia.org/wiki/Freeform_surface_modelling

# Debugging:

## SUBOPTIMAL_KHR bug:

- ash-0.37.0/src/prelude.rs:13 > `result_with_success` converts `ash::vk::Result` to `Result<T, ash::vk::Result>` and turns all VkResult variations other than VkSuccess into Err
	- eeeeeeeeeehhh? vk.xml `Return codes (positive values)` alone defines a bunch of non-error codes, let alone the extensions e.g. SUBOPTIMAL_KHR
	- to be fair though, you don't want VkSuccess returned because default behaviour should be to ignore it and results other than VkSuccess should be processed/handled...
- `std::Result::map_err` converts one error type to another using...
- target/debug/build/vulkano/out/errors.rs:38 converts from `ash::vk::Result` to `vulkano::VulkanError`
- vulkano-0.31.0/src/command_buffer/submit/queue_present.rs:308 converts from `vulkano::VulkanError` to `SubmitPresentError`... and PANICs in the default match arm!!! so any error not listed is a panic. great...
	- same thing happens in:
		- vulkano-0.31.0/src/lib.rs:183 (`OomError`)
- we can't pattern match SUBOPTIMAL in `SubmitPresentError::from` because it's not a error and thus not part of generated VulkanError (see vulkano/autogen/errors.rs)
- options:
	- have VulkanError varient for positive return codes
		- NO would have to expose `ash::vk::Result` for pattern matching
	- need to generate `vulkano::VulkanSuccess` as well as `vulkano::VulkanError` todo...
	- `from(val: ash::vk::Result) -> VulkanError` is inheritly flawed. vulkano 0.30.0 had it right (see below...)

vulkano 0.30.0?
- src/lib.rs:167 defines `vulkano::Success` (private)
- src/lib.rs:207 defines `check_errors` to convert `ash::vk::Result` to `Result<Success, Error>` which is always combined with error propogatino meaning positive results were simply ignored... not ideal really, Success should be exposed for the library user right?

## build times

stable 27.35s
nightly 27.99s
ramdisk 26.05s
lld 25.34s
mold 26.29s

## stack debugging:

- bt -> backtrace
- p $sp -> stack pointer
- p &variable -> variable address
- p variable -> variable contents
- step -> step into
- next -> step over
- b function:line_num -> breakpoint
- b module::path::function -> breakpoint
- print sizeof(variable/type) -> sizeof
- info frame [args] -> info about stack frame

## p $sp

debug:
main.rs:30 (start of main)						-> 0x7fffffffdae0
render_manager.rs:73 (start of new) 			-> 0x7fffffff9d20
vulkano::shader::ShaderModule::from_words		-> 0x7fffffff96f0
spirv.rs:53 (start of Spirv::new)				-> 0x7fffffff6f60
spirv_parse.rs Instruction::parse				-> 0x7fffffefa4c0
(gdb) info frame
Stack level 0, frame at 0x7fffffff6f60:
 rip = 0x555555815f4e in vulkano::shader::spirv::Instruction::parse
    (/home/david/Documents/source/DEV/Goshenite/target/debug/build/vulkano-04304a039d33c327/out/spirv_parse.rs:3977); saved rip = 0x555555811b52
 called by frame at 0x7fffffff96f0
 source language rust.
 Arglist at 0x7fffffefa4b8, args: reader=0x7fffffff7590
 Locals at 0x7fffffefa4b8, Previous frame's sp is 0x7fffffff6f60
 Saved registers:
  rip at 0x7fffffff6f58

release-with-debug-info:
main.rs:30 (start of main)						-> 0x7fffffffdc20
render_manager.rs:73 (start of new) 			-> 0x7fffffffc640
spirv.rs:53 (start of Spirv::new)				-> 0x7fffffffbec0
b spirv.rs:87; step
spirv_parse.rs Instruction::parse				-> 0x7fffffffbc40

nightly debug:
spirv.rs:53 (start of Spirv::new)				-> 0x7fffffff6f70
spirv_parse.rs Instruction::parse				-> 0x7fffffefa4d0

# splash
```
	     ___     
	    /\  \    
	   /  \  \   
	  / /\ \  \  
	 / /  \ \  \ 
	/ /__/ \ \__\
	\ \  /\ \/__/
	 \ \ \ \__\  
	  \ \/ /  /  
	   \  /  /   
	    \/__/    
	     ___     
	    /\  \    
	   /  \  \   
	  / /\ \  \  
	 / /  \ \  \ 
	/ /__/ \ \__\
	\ \  \ / /  /
	 \ \  / /  / 
	  \ \/ /  /  
	   \  /  /   
	    \/__/    
	     ___     
	    /\  \    
	   /  \  \   
	  / /\ \  \  
	 _\ \ \ \  \ 
	/\ \ \ \ \__\
	\ \ \ \ \/__/
	 \ \ \ \__\  
	  \ \/ /  /  
	   \  /  /   
	    \/__/    
	     ___     
	    /\__\    
	   / /  /    
	  / /__/     
	 /  \  \ ___ 
	/ /\ \  /\__\
	\/__\ \/ /  /
	     \  /  / 
	     / /  /  
	    / /  /   
	    \/__/    
	     ___     
	    /\  \    
	   /  \  \   
	  / /\ \  \  
	 /  \ \ \  \ 
	/ /\ \ \ \__\
	\ \ \ \ \/__/
	 \ \ \ \__\  
	  \ \ \/__/  
	   \ \__\    
	    \/__/    
	     ___     
	    /\__\    
	   / /  /    
	  / /  /     
	 / /__/_____ 
	/  _____ \__\
	\/__/  / /  /
	      / /  / 
	     / /  /  
	    / /  /   
	    \/__/    
	     ___     
	    /\  \    
	    \ \  \   
	     \ \  \  
	 ___ /  \  \ 
	/\  / /\ \__\
	\ \/ /  \/__/
	 \  /__/     
	  \ \  \     
	   \ \__\    
	    \/__/    
	     ___     
        /\  \    
	    \ \  \   
	     \ \  \  
	     /  \  \ 
	    / /\ \__\
	   / /  \/__/
	  / /  /     
	  \/__/      

     ___        ___        ___        ___        ___        ___        ___       ___        ___     
    /\  \      /\  \      /\  \      /\__\      /\  \      /\__\      /\  \     /\  \      /\  \    
   /  \  \    /  \  \    /  \  \    / /  /     /  \  \    / /  /      \ \  \    \ \  \    /  \  \   
  / /\ \  \  / /\ \  \  / /\ \  \  / /__/     / /\ \  \  / /  /        \ \  \    \ \  \  / /\ \  \  
 / /  \ \  \/ /  \ \  \_\ \ \ \  \/  \  \ ___/  \ \ \  \/ /__/_____ __ /  \  \   /  \  \/  \ \ \  \ 
/ /__/ \ \__\/__/ \ \__\ \ \ \ \__\/\ \  /\__\/\ \ \ \__\ _____ \__\  / /\ \__\ / /\ \__\/\ \ \ \__\
\ \  /\ \/__/\  \ / /  /\ \ \ \/__/__\ \/ /  /\ \ \ \/__/__/  / /  /\/ /  \/__// /  \/__/\ \ \ \/__/
 \ \ \ \__\ \ \  / /  /\ \ \ \__\     \  /  /\ \ \ \__\      / /  /\  /__/    / /  /    \ \ \ \__\  
  \ \/ /  /  \ \/ /  /  \ \/ /  /     / /  /  \ \ \/__/     / /  /  \ \  \    \/__/      \ \ \/__/  
   \  /  /    \  /  /    \  /  /     / /  /    \ \__\      / /  /    \ \__\               \ \__\    
    \/__/      \/__/      \/__/      \/__/      \/__/      \/__/      \/__/                \/__/    

¯\_(ツ)_/¯

```