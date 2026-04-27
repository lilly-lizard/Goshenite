# new renderer

[sdf renderer optimizations](https://www.youtube.com/watch?v=il-TXbn5iMA)
- sparse grid of cached sdf evaluations
- only cache values on surfaces: blocks with positive and negative sdf evaluations
	- sparse grid (can't use coordinates for look up)
	- could use octrees (nvidia paper: Efficient Sparse Voxel Octrees)
	- brick map
		- each block contains 8 points (for each point of cube)
		- each point stores a 1 byte sdf evaluation (don't need f32, result will be withing unit square)
		- each pixel of 3d texture atlas is a point (brick map pointer grid)
		- brick size is 8x8x8
		- lookup (pointer) texture size = 4 bytes (brick map pointer size) * 1024^3 (full grid dimensions) / 8^3 (grid dimensions) = 8MB
		- pointers point to sparse buffer with actual values (brick map) in which blocks are allocated and de-allocated if surfaces change (10:25 in video)
		- naively store everything in big buffer (whole grid, not just surface) = 1 byte * 1024^3 = 1GB
- all primitive ops (which he calls "sdf edits") tracked spacially via a bounding volume heirarchy (BVH)
	- tree of AABBs (see 13:20 for visualization)
	- BVH can be used to find all intersecting AABBs for a given ray
	- buffer shared between cpu and gpu
	- also allows you to know which primitive ops affect a region of space
- LOD
	- paper: Geometry Clipmaps: terrain rendering using nested regular grids
	- 11:55 for visualization
	- set of grids nested on top of each other, each double in size
- unknowns...
	- how best to perform initial cached evaluations
	- how to determine which sparse grid surface blocks intersect the current ray
- design goals
	- have a number of primitives to choose from
	- also allow user to program custom sdf functions
	- default primitives are just pre-made shader functions
	- rename primitive ops to sdf edits
- physics?
	- mike using [jolt](https://github.com/jrouwe/JoltPhysics)
	- uses marching cubes to generate low res collision mesh which is then fed into jolt
	- simple and easily parallelised, done on multiple cpu threads
	- [rapier 3d for rust](https://crates.io/crates/rapier3d)

# renderer architecture

stages:
1. initial calculations of each point on grid to generate
	a. generate BVH for all primtives
		- option A (simple):
			- top down generation
			- get union of all AABBs to get top of tree AABB
			- then half along x axis for next layer (equal number of primitives in each branch)
			- next layer, half along y axis, then z, then x until you get to bottom of tree
		- option B (fast): ✔️
			- place AABB centers in interger grid of length 2^n
			- sort by sinlge value: morton code
				- morton code: interleave axes, interleave bits to get 3n bit uint
			- then half the list recursively to get tree
			- does the exact same thing as option A [visualization](https://youtu.be/LAxHQZ8RjQ4?si=6uQRbcwBTc_KXcMp&t=480)
			- good for regenerating entire BVH every nth frame to account for dynamic primitives
	b. sparse buffer cache atlas for blocks intersecting surfaces
		- cache sdf result
			- d, id
		- option A: cache geometry properties and calculate lighting per frame ✔️
			- albedo, specular, normal
			- requires 2x memory usage for vertex buffers
		- option B: cache lighting here ❌️
			- final color: albedo * illumination * specular * normal etc.
			- lighting is expensive! doing it for the whole grid seems excessive, expecially given how much of it will be occluded in a forest
	c. lookup pointer table for whole volume
		- for each block use BVH to determine relevant primitives to evaluate
		- only store pointers for blocks with both positive and negative values (indicating a surface)
2. each frame:
	- determine which blocks need regenerated because of... _(note: can be implimented later)_
		a. moved primitive op
		b. LOD changed (block increased or decreased in sparseness) due to camera origin moving (note: done to blocks outside viewpoint too to avoid spikes in regeneration load)
	- for each pixel/ray
		- determine what grid blocks the ray intersects
		- evaluate sdf result for each from closest to farthest
			- option A: ❌️
				- rasterize cubes for each grid block without backface culling
				- fragment invocations for entry and exit points
				- hit when first negative d is found
				- concerns: this doesn't utilize trilinear interpolation, just bilinear...
			- option B: ✔️
				- rasterize cubes for each grid block with backface culling
				- ray-march through the block
				- hit if d < epsilom
				- miss if d > block dimension
				- a lot more accurate not a lot more computation
				- only requires ray calc to determine point and then some stepping
		- for each block:
			- check if pointer resides in lookup table
			- for blocks with pointers:
				- perform trilinear interpolation of sdf result fields: d, albedo, specular, normal, illumination
				- closest interpolation for uint id (note when generating these point values, id = closest primitive as well)

pipeline:
1. cache gen
	- BVH (+ grid) -> surface pointers + vertex buffers
2. per frame
	- render blocks as instanced cubes
		- option A: use pointer lookup table cpu side to determine instances
		- option B: during surface block generation, create an indirect rendering buffer

gpu specifics:
- buffers:
	- BVH
		- shared between cpu and gpu
	- grid lookup: 128x128x128 x 4byte (1024 / 8 = 128) = 8MB
	- frame draw vertex buffer:
		1. vertex buffer
			- positions of cube
			- VK_FORMAT_R32G32B32_SFLOAT
		2. index buffer
			- indicies of cube
			- VK_INDEX_TYPE_UINT32
		3. instance buffer
			- per unit grid block
			- position, size multiple (grid id?)
			- per stride:
				a. VK_FORMAT_R32G32B32_SFLOAT position
				b. VK_FORMAT_R16_UINT size multiple
				c. VK_FORMAT_R16_UINT spare...
	- sparse cached result 3D image buffers: 3d textures, each block is a group of 8x8x8 points, ? blocks initially allocated (may allocate more memory as needed)
		- note: need to verify webgpu format features. need DeviceDescriptor.required_features: GPUFeatureName::float32-filterable for any 32bit float formats
		a. VK_FORMAT_R16_SFLOAT (sampled often during ray marching)
			- for evaluated d values
			- most often accessed as it is used during ray marching
			- trilinear interpolation
		b. VK_FORMAT_R16_UINT (only sampled upon hit)
			- id (uint)
			- closest interpolation
		- option A: ✔️
			c. VK_FORMAT_R8G8B8A8_SRGB (only sampled upon hit)
				- albedo (vec3)
				- specular (float)
				- trilinear interpolation
			d. VK_FORMAT_R8G8B8A8_SNORM (only sampled upon hit)
				- normals (vec3)
				- ? in A channel
				- trilinear interpolation
		- option B: ❌️
			- VK_FORMAT_R8G8B8A8_SRGB
				- color (vec3)
				- ? in A channel
				- trilinear interpolation
	- output framebuffers
		1. color
		2. VK_FORMAT_R16_UINT id

- ordering options:
	- primitive op order preserved within object
	- each object processed individually before being combined, otherwise a hole in a tree will also cut a hole into animals that crawl on it too and vice versa...
	- downside of this is that a ray only intercepts some primitive ops within an object so how do you query primitive op order in that scenario?
	- conclusion: no distinction between objects within rendering code. its an extra abstraction to keep track of that would significantly overcomplicate the pipeline...

# plan:

1. generate BVH and render BVH outlines
?. [panini projection slider](https://www.youtube.com/watch?v=LE9kxUQ-l14)
	- [math options comparison](https://en.wikipedia.org/wiki/Fisheye_lens#Mapping_function)
?. [fsr upscaling](https://github.com/EmbarkStudios/fsr-rs)

- gizmo arrow highlight upon hover
- click and drag gizmo arrows
- draw circle at selected object center
- make circle consistent size relative to screen space
- arrows for hovered object (block colors, no shading, with slight alpha)
- generate BVH and render BVH outlines

# webgpu?

[use wesl for shader imports](https://wesl-lang.dev/)
[cool collection of wgsl shaders](https://github.com/alphastrata/shadplay/tree/main)
shader language:
- [slang -> wgsl](https://shader-slang.org/)
- [slang rust bindings](https://github.com/FloatyMonkey/slang-rs)

# misc

- license (every source file)
	- https://kristoff.it/blog/the-open-source-game/
	- https://web.archive.org/web/20231030181609/https://redis.io/docs/about/governance/
	- https://en.wikipedia.org/wiki/Benevolent_dictator_for_life
	- https://opensource.org/license/bsd-3-clause/
	- https://redis.io/docs/about/license/
	- https://github.com/redis/redis/blob/unstable/src/acl.c
	- https://github.com/godotengine/godot/blob/master/main/main.cpp

wayland renderdoc:
`WAYLAND_DISPLAY= XDG_SESSION_TYPE=x11 qrenderdoc`

# design goals

- advantages of sdf:
	- blending makes things look organic and yucky
	- easy to animate warping geometry
- don't design for an end user, design for me to make something cool
- toolset:
	- curl noise field

_"Well, if I were to use an analogy for analog and digital, analog is like a calligrapher, and digital is like a craftsman. Doing something really precisely with tools versus kind of doing it based on feeling. It's that kind of difference. Digital is really focused on working on the details, so a lot of the time is spent on those details and sometimes you lose sight of other things. Analog, on the other hand, sometimes you can even use accidents to complete the drawing. It's like leaving it up to your own 'energy', I think that's interesting."_ - [Kentaro Miura on drawing](https://www.youtube.com/watch?v=GmJjLy2i3Zg)

# resources

- ray marching intro https://michaelwalczyk.com/blog-ray-marching.html
- [sdfs and op functions](https://iquilezles.org/articles/distfunctions/)

- UX: https://asktog.com/atc/principles-of-interaction-design/

- https://www.shadertoy.com/view/NddSWs
- https://imgur.com/a/YZ3p7Ce

- https://en.wikipedia.org/wiki/T-spline
- https://en.wikipedia.org/wiki/Freeform_surface_modelling

- (cube marching)[https://curved-ruler.github.io/webgl-sketches/marching/mc.html]

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

# DEBUGGING...

09:57:31.022185 run-command.c:740            | d0 | main                     | child_start  |     |  0.003988 |           |              | [ch0] class:? argv:[gpg --status-fd=2 -bsau B5857277126B5D1C]
_long wait..._
09:58:02.862556 run-command.c:996            | d0 | main                     | child_exit   |     | 31.844359 | 31.840371 |              | [ch0] pid:23811 code:0