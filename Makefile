# Lemon workspace — common development commands
# Run `make` or `make help` for targets.

CARGO := cargo
EXAMPLES := counter signals memo effects keys components layout form rich slider images select scroll

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

clippy: ## Lint workspace with warnings denied
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test: ## Run all workspace tests
	$(CARGO) test --workspace

build: ## Build entire workspace
	$(CARGO) build --workspace

build-examples: ## Build all example binaries
	@for ex in $(EXAMPLES); do \
		echo "==> building $$ex"; \
		$(CARGO) build -p $$ex || exit $$?; \
	done

doc: ## Build rustdoc for lemon (library)
	$(CARGO) doc -p lemon --no-deps

doc-open: doc ## Build and open lemon docs in the browser
	$(CARGO) doc -p lemon --no-deps --open

clean: ## Remove build artifacts
	$(CARGO) clean
