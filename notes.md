# TODO

- window resize handling
- draw sdf sphere with AABB

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