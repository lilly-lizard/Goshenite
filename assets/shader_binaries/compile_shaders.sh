#!/bin/bash

# stop on first error
set -e

cd ../../src/renderer/glsl

FRAG_SRC="*.frag"
VERT_SRC="*.vert"

for src in $FRAG_SRC; do
	glslc $src -o "../../../assets/shader_binaries/$src.spv"
done
for src in $VERT_SRC; do
	glslc $src -o "../../../assets/shader_binaries/$src.spv"
done