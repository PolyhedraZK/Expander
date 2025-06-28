# Makefile for Expander-GPU project
PROFILE_LEVEL ?= 0
DO_ALL ?= 1
FIELD_TYPE ?= m31ext3

# Supported field types
FIELDS := m31ext3 goldilocksext2 bn254

# Generate circuit file names for all supported field types
CIRCUIT_FILES := $(addprefix data/keccak_,$(addsuffix .gpu.circuit,$(FIELDS)))

# Base command template
# Usage: $(call run_field,FIELD,MPI_LEN,RUN_FLAGS)
define run_field
./expander-gpu --field $(1) --circuit data/keccak_$(1).gpu.circuit --log_level 0 --mpi_len $(2) $(3)
endef

# Helper function to run expander with different field types
# Usage: $(call run_expander,MPI_LEN,RUN_FLAGS)
define run_expander
	$(if $(filter 1,$(DO_ALL)), \
		$(call run_field,m31ext3,$(1),$(2)); \
		$(call run_field,goldilocksext2,$(1),$(2)); \
		$(call run_field,bn254,$(1),$(2)), \
		$(if $(filter $(FIELD_TYPE),$(FIELDS)), \
			$(call run_field,$(FIELD_TYPE),$(1),$(2)), \
			$(error Invalid FIELD_TYPE '$(FIELD_TYPE)'. Must be one of: $(FIELDS))))
endef

.PHONY: clean test prepare-data profile mpi-profile mpi-test

# When circuit files are missing, guide user to run the prepare script.
data/keccak_%.gpu.circuit:
	@echo "Error: Circuit file '$@' is missing."
	@echo "Please run './prepare-data.sh' to generate necessary circuit files."
	@exit 1

# Prepare all circuit data files (only if they don't exist)
data: $(CIRCUIT_FILES)
	@echo "All circuit data files are ready."

test: data
	$(call run_expander,128,)

profile: data
	$(call run_expander,8192,--enable-same-input --profile $(PROFILE_LEVEL))

mpi-test: data
	$(call run_expander,128,--enable-mpi-merge)

mpi-profile: data
	$(call run_expander,16384,--enable-same-input --profile $(PROFILE_LEVEL) --enable-mpi-merge)

clean:
	rm -rf data
