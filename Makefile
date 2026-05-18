# Lemon — common development commands
# Run `make` or `make help` for targets.

CARGO := cargo

.PHONY: help check ci fmt fmt-check clippy test build build-examples doc doc-open clean

.DEFAULT_GOAL := help

help: ## Show available targets
	@printf "Lemon — useful commands:\n\n"
	@grep -E '^[a-zA-Z0-9_.-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

check: fmt-check clippy test build-examples ## CI gates: fmt, clippy, test, examples

ci: check ## Alias for check

fmt: ## Format all Rust code
	$(CARGO) fmt --all

fmt-check: ## Check formatting (no changes)
	$(CARGO) fmt --all -- --check

clippy: ## Lint with warnings denied
	$(CARGO) clippy --all-targets -- -D warnings

test: ## Run all tests (lib, widget, doctests)
	$(CARGO) test

build: ## Build library and binaries
	$(CARGO) build

build-examples: ## Build all examples
	$(CARGO) build --examples

doc: ## Build rustdoc
	$(CARGO) doc --no-deps

doc-open: doc ## Build and open docs in the browser
	$(CARGO) doc --no-deps --open

clean: ## Remove build artifacts
	$(CARGO) clean
