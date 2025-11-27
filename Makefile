# Makefile para CODE_MANIFEST
MANIFEST_SCRIPT := scripts/generate_clean_manifest.sh
MANIFEST_FILE := CODE_MANIFEST.csv

.PHONY: manifest

manifest:
	@echo "Generating CODE_MANIFEST.csv..."
	@bash $(MANIFEST_SCRIPT)
