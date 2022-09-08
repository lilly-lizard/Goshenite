# TODO

- clippy
- cpu writes to buffer with spheres
- draw sdf sphere with AABB
- have multiple spheres
- Camera::rotate quaternions
- credit unwrap-or gitlab for unwrap_or_exec
- credit cgmath for radians/degrees idea
- shaderStorageImageExtendedFormats
	- https://registry.khronos.org/vulkan/specs/1.3-extensions/html/chap43.html#features-required-format-support
	- VK_FORMAT_A2B10G10R10_UNORM_PACK32 or VK_FORMAT_B10G11R11_UFLOAT_PACK32
	- https://stackoverflow.com/questions/72548476/whats-the-best-practice-for-handling-format-qualifiers-for-images-in-compute-sh

focus on fast iteration! **avoid premature optimization** quick and dirty first.

## cleanup

- validation layers
- unwrap error handling

## low priority

- pipeline cache (and write)
- curl noise field
- Bang Wong color palette

# Code Guidelines

- Consider commenting with structure of 'action' followed by 'object' e.g. 'transition
	image layout (action) for depth buffer (object)'. This makes it easier to search for
comments by action or object e.g. a search for 'transition image layout' wouldn't find
	the comment 'transition depth buffer image layout'
- avoid Box<dyn Error> if possible, just create an enum https://fettblog.eu/rust-enums-wrapping-errors/

# design decisions

create objects and coloring from editor, set to vary against variables etc
possibilities:
- sequence of primitives, transformations and combinations in storage buffer
	e.g. buffer: Vec<u32> = { num primitives, SPHERE, center, radius, UNION, SPHERE, center, radius... }
- color?
- define uv functions and associate textures
- editor generates shaders. real time feedback?
- live feedback modes e.g. sculpting mode just has primitives and normals
- ignore color and render normals to start off with
- **sworld space**: z up; right handed (x forward, y left), camera space: z depth

# Resources

- HIP instead of vulkan compute? https://github.com/charles-r-earp/hip-sys

# References

- UX: https://asktog.com/atc/principles-of-interaction-design/

- https://www.shadertoy.com/view/NddSWs
- https://imgur.com/a/YZ3p7Ce

- https://en.wikipedia.org/wiki/T-spline
- https://en.wikipedia.org/wiki/Freeform_surface_modelling

# build times

stable 27.35s
nightly 27.99s
ramdisk 26.05s
lld 25.34s
mold 26.29s

# stack debugging:

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
	  \ \ \ \  \ 
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
 / /  \ \  \/ /  \ \  \ \ \ \ \  \/  \  \ ___/  \ \ \  \/ /__/_____ __ /  \  \   /  \  \/  \ \ \  \ 
/ /__/ \ \__\/__/ \ \__\ \ \ \ \__\/\ \  /\__\/\ \ \ \__\ _____ \__\  / /\ \__\ / /\ \  \/\ \ \ \__\
\ \  /\ \/__/\  \ / /  /\ \ \ \/__/__\ \/ /  /\ \ \ \/__/__/  / /  /\/ /  \/__// /  \/__/\ \ \ \/__/
 \ \ \ \__\ \ \  / /  /\ \ \ \__\     \  /  /\ \ \ \__\      / /  /\  /__/    / /  /    \ \ \ \__\  
  \ \/ /  /  \ \/ /  /  \ \/ /  /     / /  /  \ \ \/__/     / /  /  \ \  \    \/__/      \ \ \/__/  
   \  /  /    \  /  /    \  /  /     / /  /    \ \__\      / /  /    \ \__\               \ \__\    
    \/__/      \/__/      \/__/      \/__/      \/__/      \/__/      \/__/                \/__/    
```