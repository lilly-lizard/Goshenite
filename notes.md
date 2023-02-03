# general guidelines

focus on fast iteration! **avoid premature optimization** quick and dirty first.

# big todo

1. objects refactor
	1. object struct - DONE
	2. shaders - DONE
	3. buffer/renderer code
		1. single object - DONE
		2. multiple objects - DONE
	4. gui/engine
		1. gui
			- object list - DONE
			- edit primitives - DONE
			- add/remove objects/primitives - DONE
		2. overlay pass
2. organise shaders directory structure and content
	- would be nice to structure as per how I conceptualize the code i.e. - DONE
		1. user interface -> intuitive, fast and clear feedback. ux/gui
		2. backend -> optimized. rendering code
		3. engine -> abstraction. connecting glue between user interface and backend
	- clearly defined design goals for each section of the code
3. raster bounding boxes (hard-coded AABB for now)

# todo

- frame time display
- checks in object_collection to make sure that you don't have the same primitive ids across multiple primitive ops. put primitive_references inside object_collection?
- init renderer test
- smooth union op (curved combination)
- hemisphere (circle) clamps on looking too far up/down (quaternions?)
- surface noise modifiers
- render outline on selected object
- output render png (write tests using this?)
- can we do without Rc<RefCell<>>?
- save model as file
- curl noise field
- scroll zoom proportional to distance (try scrolling on trackpad) and don't want to scroll out other side
- anti-aliased ui
- replace UniqueId type aliases with structs or enums to have explicit conversions and rules

## cleanup

- primitive_references to dos
- support/handle VulkanError cases...

## always

- unwrap/except/assert
- clippy

## bugz

- subtraction op broken. record bug before fixing tho lol
- CursorState not initialized properly! e.g. cursor position 0,0 at start so start dragging before moving it and a big jump occurs. also check latest winit in case querying was made better?
- Gui::_bug_test_window

## optimize

- user_interface stuff in its own thread e.g. wait cursor
- decrease MIN_DIST as distance progresses (need less resolution)
- FastMemoryAllocator for frequent (per frame) memory allocations (see StandardMemoryAllocator description)
- gui performance hit when list becomes too big (https://github.com/emilk/egui#cpu-usage) try only laying out part of list in view
- GuiRenderer::create_texture -> create_textures batch texture creation

## low priority

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

## gui additions

- see egui_demo_app for ideas
- button for light/dark theme

## commands

- flip to other side of focused object

# Code Guidelines

- Consider commenting with structure of 'action' followed by 'object' e.g. 'transition
	image layout (action) for depth buffer (object)'. This makes it easier to search for
comments by action or object e.g. a search for 'transition image layout' wouldn't find
	the comment 'transition depth buffer image layout'
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

## descriptor indexing

https://jorenjoestar.github.io/post/vulkan_bindless_texture/
https://community.arm.com/arm-community-blogs/b/graphics-gaming-and-vr-blog/posts/vulkan-descriptor-indexing

whitebox:
goshenite::renderer::geometry_pass::create_desc_set
vulkano::descriptor_set::pool::DescriptorPool::new -> pool_sizes empty
because vulkano::descriptor_set_allocator::FixedPool::new -> layout.descriptor_counts() is empty
because frag shader (EntryPoint::info: EntryPointInfo).descriptor_requirements: HashMap<(u32, u32), DescriptorRequirements> (in vulkano/src/shader/mod.rs) is empty
set in vulkano/src/shader/mod.rs ShaderModule::from_words_with_data

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

# gpu hardware

- https://github.com/VerticalResearchGroup/miaow
- https://arxiv.org/pdf/2111.06166.pdf
- https://github.com/openhwgroup/cv32e40p
- https://github.com/malkadi/FGPU

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