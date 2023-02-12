# Bort

Is a pretty unambitious, lightweight vulkan wrapper aiming to reduce boilerplate, call destructors with `Drop`, reference count resource dependencies, store create-info properties etc. Makes some assumptions of use-case to simplify things (e.g. merging image and image view into one struct) but it should all be pretty easy to modify.
