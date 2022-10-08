#!/bin/bash

# exit on first error
set -e

# check for required binaries
echo "will use programs:"
which circle
which spirv-dis
which spirv-as
echo ""

spv_out_dir="../../assets/shader_binaries"

for shader in *.cxx ;
do
	echo "compiling ${shader}..." ;

	shader_spv="${shader}.spv" ;
	# use circle compiler (https://www.circle-lang.org/)
	circle -shader -c -emit-spirv -o "/tmp/${shader_spv}" "${shader}" ;
	# circle version 170 adds GL_EXT_scalar_block_layout which needs to be removed
	spirv-dis "/tmp/${shader_spv}" > "/tmp/${shader_spv}.dis"
	sed -i '/GL_EXT_scalar_block_layout/d' "/tmp/${shader_spv}.dis"
	spirv-as "/tmp/${shader_spv}.dis" -o "${spv_out_dir}/${shader_spv}"

	echo "compiled ${shader_spv}" ;
done
