#!/bin/bash

# stop on first error
set -e

FRAG_SRC="../../src/renderer/shader_source/*.frag"
VERT_SRC="../../src/renderer/shader_source/*.vert"

for src in $FRAG_SRC; do
	output_file="$(basename $src)"
	echo "$output_file"
	glslangValidator -V -o "$output_file.spv" "$src"
	spirv-opt -O $output_file.spv -o "$output_file.spv"
done

for src in $VERT_SRC; do
	output_file="$(basename $src)"
	echo "$output_file"
	glslangValidator -V -o "$output_file.spv" "$src"
	spirv-opt -O "$output_file.spv" -o "$output_file.spv"
done