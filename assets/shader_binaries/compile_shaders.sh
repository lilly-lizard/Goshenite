#!/bin/bash

# stop on first error
set -e

FRAG_SRC="../../src/renderer/glsl/*.frag"
VERT_SRC="../../src/renderer/glsl/*.vert"

for src in $FRAG_SRC; do
	glslc "$src" -o "$src.spv"
	spirv-opt -O $src.spv -o "$src.spv"
done
for src in $VERT_SRC; do
	glslc "$src" -o "$src.spv"
	spirv-opt -O "$src.spv" -o "$src.spv"
done