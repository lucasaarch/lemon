# Lemon workspace — common development commands
# Run `make` or `make help` for targets.

CARGO := cargo
EXAMPLES := counter signals memo effects keys components layout form

.PHONY: help \
	check ci fmt fmt-check clippy test build build-examples doc doc-open \
	run-counter run-signals run-memo run-effects run-keys run-components run-layout run-form run \
	clean

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

run-counter: ## Run the counter quick-start example
	$(CARGO) run -p counter

run-signals: ## Run the use_signal example
	$(CARGO) run -p signals

run-memo: ## Run the use_memo example
	$(CARGO) run -p memo

run-effects: ## Run the use_effect example
	$(CARGO) run -p effects

run-keys: ## Run the keyed list example
	$(CARGO) run -p keys

run-components: ## Run the Component example
	$(CARGO) run -p components

run-layout: ## Run the layout example
	$(CARGO) run -p layout

run-form: ## Run the widgets / form example
	$(CARGO) run -p form

run: run-counter ## Default example (counter)

clean: ## Remove build artifacts
	$(CARGO) clean
