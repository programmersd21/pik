.PHONY: help build release test clean install uninstall fmt check clippy lint ci demo all

# Detect OS - works on Windows (no uname) and Unix
UNAME := $(shell uname -s 2>NUL)
ifneq ($(UNAME),)
	DETECTED_OS := $(UNAME)
	BINARY := target/release/pik
	DEBUG_BINARY := target/debug/pik
	NULL_DEVICE := /dev/null
	WHICH := which
else
	DETECTED_OS := Windows
	BINARY := target\release\pik.exe
	DEBUG_BINARY := target\debug\pik.exe
	NULL_DEVICE := NUL
	WHICH := where
endif

# Default target
help:
	@echo "pik - Makefile targets (OS: $(DETECTED_OS))"
	@echo ""
	@echo "Development:"
	@echo "  make build      Build debug binary"
	@echo "  make release    Build optimized binary"
	@echo "  make test       Run all tests"
	@echo "  make fmt        Format code"
	@echo "  make clippy     Run clippy lints"
	@echo "  make check      Run fmt + clippy + test"
	@echo "  make ci         Full CI check (fmt check + clippy + test)"
	@echo ""
	@echo "Installation:"
	@echo "  make install    Install binary to cargo bin"
	@echo "  make uninstall  Remove installed binary"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean      Remove build artifacts"
	@echo ""
	@echo "Demo:"
	@echo "  make demo       Generate demo.gif (requires VHS)"
	@echo ""
	@echo "Shortcuts:"
	@echo "  make all        Same as: make check"
	@echo "  make lint       Same as: make clippy"

# Build debug binary
build:
	@echo "Building debug binary..."
	@cargo build
	@echo "✓ Debug build complete: $(DEBUG_BINARY)"

# Build optimized release binary
release:
	@echo "Building release binary..."
	@cargo build --release
	@echo "✓ Release build complete: $(BINARY)"

# Run all tests
test:
	@echo "Running tests..."
	@cargo test --all-targets
	@echo "✓ All tests passed"

# Format code
fmt:
	@echo "Formatting code..."
	@cargo fmt
	@echo "✓ Code formatted"

# Check formatting without modifying files
fmt-check:
	@echo "Checking code format..."
	@cargo fmt -- --check
	@echo "✓ Code format verified"

# Run clippy
clippy:
	@echo "Running clippy..."
	@cargo clippy --all-targets -- -D warnings
	@echo "✓ Clippy checks passed"

# Alias for clippy
lint: clippy

# Quick check (format + lint + test)
check: fmt clippy test
	@echo ""
	@echo "✓✓✓ All checks passed ✓✓✓"

# Full CI check (non-modifying format check + clippy + test)
ci: fmt-check clippy test
	@echo ""
	@echo "✓✓✓ CI checks passed ✓✓✓"

# Install binary
install:
	@echo "Installing pik..."
	@cargo install --path .
	@echo "✓ Installed to cargo bin directory"
	@$(WHICH) pik >$(NULL_DEVICE) 2>&1 && $(WHICH) pik || echo "Note: Make sure cargo bin is in PATH"

# Uninstall binary
uninstall:
	@echo "Uninstalling pik..."
	@cargo uninstall pik
	@echo "✓ Uninstalled"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@echo "✓ Cleaned"

# Generate demo (requires VHS)
demo:
	@echo "Generating demo.gif..."
ifdef OS
	ifeq ($(OS),Windows_NT)
		@where vhs >$(NULL_DEVICE) 2>&1 || (echo Error: VHS not found. Install from https://github.com/charmbracelet/vhs && exit 1)
	else
		@command -v vhs >$(NULL_DEVICE) 2>&1 || { echo "Error: VHS not found. Install from https://github.com/charmbracelet/vhs"; exit 1; }
	endif
else
	@command -v vhs >$(NULL_DEVICE) 2>&1 || { echo "Error: VHS not found. Install from https://github.com/charmbracelet/vhs"; exit 1; }
endif
	@vhs demo.tape
	@echo "✓ Demo generated: demo.gif"

# Default: run all checks
all: check

# Development workflow: watch and test (requires cargo-watch)
watch:
ifdef OS
	ifeq ($(OS),Windows_NT)
		@where cargo-watch >$(NULL_DEVICE) 2>&1 || (echo Installing cargo-watch... && cargo install cargo-watch)
	else
		@command -v cargo-watch >$(NULL_DEVICE) 2>&1 || { echo "Installing cargo-watch..."; cargo install cargo-watch; }
	endif
else
	@command -v cargo-watch >$(NULL_DEVICE) 2>&1 || { echo "Installing cargo-watch..."; cargo install cargo-watch; }
endif
	@cargo watch -x test

# Show binary size
size: release
	@echo ""
	@echo "Binary size:"
ifdef OS
	ifeq ($(OS),Windows_NT)
		@powershell -Command "Get-Item $(BINARY) | Select-Object Name, @{Name='Size';Expression={'{0:N2} KB' -f ($_.Length / 1KB)}}"
	else
		@ls -lh $(BINARY) | awk '{print $$5 "\t" $$NF}'
	endif
else
	@ls -lh $(BINARY) | awk '{print $$5 "\t" $$NF}'
endif

# Benchmark build time
bench-build:
	@echo "Benchmarking clean release build..."
	@cargo clean
ifeq ($(DETECTED_OS),Windows)
	@powershell -Command "Measure-Command { cargo build --release | Out-Default } | Select-Object TotalSeconds"
else
	@time cargo build --release
endif

# Run a quick smoke test
smoke: release
	@echo "Running smoke test..."
	@echo "Note: This requires manual interaction - press Ctrl+C to cancel"
ifeq ($(DETECTED_OS),Windows)
	@echo option1& echo option2& echo option3 | $(BINARY) || echo "Manual verification needed"
else
	@echo -e "option1\noption2\noption3" | $(BINARY) || echo "Manual verification needed"
endif
	@echo "✓ Smoke test setup complete"

