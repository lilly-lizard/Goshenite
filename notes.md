# general guidelines

focus on fast iteration! **avoid premature optimization** quick and dirty first.

# todo

- switch to back-face culling beacuse only the misses benefit from the max-dist cutoff, where as the more expensive hit frags will benefit from the initial min-distance
- add support for mouse button numbers using maps

- license (every source file)
	- https://kristoff.it/blog/the-open-source-game/
	- https://web.archive.org/web/20231030181609/https://redis.io/docs/about/governance/
	- https://en.wikipedia.org/wiki/Benevolent_dictator_for_life
	- https://opensource.org/license/bsd-3-clause/
	- https://redis.io/docs/about/license/
	- https://github.com/redis/redis/blob/unstable/src/acl.c
	- https://github.com/godotengine/godot/blob/master/main/main.cpp
- code of conduct
- contributing

- undo (ctrl z)

- compute dispatch
- line surface with cube meshes
- 1 primitive op per instance and use depth test to blend
- make AllocationCreateInfo similar to BufferProperties. unintuitive usage of bort_vma structs, too many options

- object file format (e.g. gltf)
- object surface octree cpu code (use file format to have tests/benchmarks)

- memory allocation write with bytemuck slice i.e. copy all at once rather than with iter

- get rid of curved egui borders
- change egui font
- object/primitive op ids in gui
- gui code that edits engine stuff e.g. camera, objects, primitives -> funcitons that could be called via command interface!
- json save theme setting
- surface patterns (natural looking noise stuff)
- path tracing heat map
- geometry pass depth buffer
- frame time display
- init renderer test
- smooth union op (curved combination)
- surface noise modifiers
- render outline on selected object
- curl noise field
- scroll zoom proportional to distance (try scrolling on trackpad) and don't want to scroll out other side
- replace UniqueId type aliases with structs or enums to have explicit conversions and rules
- bounding box viewer
- custom bounding boxes
- anti-aliased gui
- 'waiting' cursor when other code taking time...

## ui additions

- comand palette and keyboard shortcuts
- undo (egui::undoer)
- serde save gui state (start with theme)
- see egui_demo_app for ideas
- Bang Wong color palette

## bugz!

## optimize

- log depth function
- user_interface stuff in its own thread e.g. wait cursor
- decrease MIN_DIST as distance progresses (need less resolution)
- FastMemoryAllocator for frequent (per frame) memory allocations (see StandardMemoryAllocator description)
- gui performance hit when list becomes too big (https://github.com/emilk/egui#cpu-usage) try only laying out part of list in view
- GuiRenderer::create_texture -> create_textures batch texture creation
- double buffering

## shader optimize

compare instructions and frame time before/after
https://www.marshallplan.at/images/All-Papers/MP-2017/Mroz+Michael_746.pdf
- minimum step cap

## low priority

- [scripting language](https://github.com/rhaiscript/rhai) + glsl for extensions!
	- install extension -> select new background pattern, texture pattern etc. 
	- similar capabilities to shadertoy i.e. 2, passes, preset inputs
	- installation includes rhai file, glsl file(s) and possible png/jpgs
- hemisphere (circle) clamps on looking too far up/down (quaternions?)
- clickable primitives
- preview new primitive, greyed out/transparent until add?
- error and warn log messages in gui (popups?)
- Camera::rotate quaternions https://www.3dgep.com/understanding-quaternions/
- tests for Primitives data

## commands

- flip to other side of focused object

slotmap
specs: ECS
rayon: parallelize iterators etc

# Code Guidelines

- Consider commenting with structure of 'action' followed by 'object' e.g. 'transition
	image layout (action) for depth buffer (object)'. This makes it easier to search for
comments by action or object e.g. a search for 'transition image layout' wouldn't find
	the comment 'transition depth buffer image layout'
- https://rust-lang.github.io/api-guidelines yeet

## logging

- all 'spammy' logging should go in trace, e.g. per-frame states, per-input states, to keep debug and higher reasonably clean and readable

# design decisions

- create objects and coloring from editor, set to vary against variables etc
- define uv functions and associate textures
- editor generates shaders. real time feedback?
- live feedback modes e.g. sculpting mode just has primitives and normals
- world space: z up; right handed (x forward, y left), camera space: z depth
- front or back-face culling?
	- going with front which gives us the optimised far distance cutoff
	- far is more important because it allows us to cut lots of threads short when camera is close to the object and it requires more fragments
	- at longer distances where the near is more important, there are less fragments anyway
	- can have push constant or something to describe optimised near using camera pos, object pos, object aabb abs max (sphere around it)
- bounding mesh or aabb?
	- bounding mesh is tighter meaning less frag invocations that miss
	- however aabbs are easy to combine, where as bounding meshes for an object would result in overlapping back face fragments, lots if the object has lots of primitives
	- big bottle-neck is map() fn but calls to map in miss case are less because the jumps are bigger and the there's an optimised far condition
	- if bounding meshes could be combined then it will become optimal

## ideas

- defer shading to raster pass? render to g-buffer, including shadow info (e.g. bitmap of light sources for primitive?)
- file storage (and memory arragement too?) https://github.com/quelsolaar/HxA

_"Well, if I were to use an analogy for analog and digital, analog is like a calligrapher, and digital is like a craftsman. Doing something really precisely with tools versus kind of doing it based on feeling. It's that kind of difference. Digital is really focused on working on the details, so a lot of the time is spent on those details and sometimes you lose sight of other things. Analog, on the other hand, sometimes you can even use accidents to complete the drawing. It's like leaving it up to your own 'energy', I think that's interesting."_ - [Kentaro Miura on drawing](https://www.youtube.com/watch?v=GmJjLy2i3Zg)
What are we aiming for? Where is the market opening?


# Resources

- HIP instead of vulkan compute? https://github.com/charles-r-earp/hip-sys
- ray marching intro https://michaelwalczyk.com/blog-ray-marching.html
- vulkan format types https://stackoverflow.com/questions/59628956/what-is-the-difference-between-normalized-scaled-and-integer-vkformats
	- format properties https://registry.khronos.org/vulkan/specs/1.3-extensions/html/chap45.html#features-required-format-support
- [sdfs and op functions](https://iquilezles.org/articles/distfunctions/)

# References

- UX: https://asktog.com/atc/principles-of-interaction-design/

- https://www.shadertoy.com/view/NddSWs
- https://imgur.com/a/YZ3p7Ce

- https://en.wikipedia.org/wiki/T-spline
- https://en.wikipedia.org/wiki/Freeform_surface_modelling

- (cube marching)[https://curved-ruler.github.io/webgl-sketches/marching/mc.html]

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
