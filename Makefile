# Rust development tasks (similar to Rake tasks in Ruby)

.PHONY: help format lint fix check test build run clean all security-audit

help: ## Show this help message
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$1, $$2}'

format: ## Format code with rustfmt (like rubocop --auto-correct)
	cargo fmt

lint: ## Run clippy linter (like rubocop)
	cargo clippy -- -W clippy::all -W clippy::pedantic

fix: ## Auto-fix clippy issues (like rubocop --auto-correct)
	cargo clippy --fix --allow-dirty -- -W clippy::uninlined-format-args

check: ## Check code without building
	cargo check

test: ## Run tests
	cargo test

build: ## Build the project
	cargo build

run: ## Run the main application
	cargo run

clean: ## Clean build artifacts
	cargo clean

security-audit: ## Run security audit (like bundler-audit in Ruby)
	cargo audit

all: format fix check test build ## Run format, fix, check, test, and build

# Development workflow (run this often!)
dev: format fix check ## Quick development check (format + fix + check)

# Strict linting (like rubocop with all cops enabled)
strict-lint: ## Run all clippy lints including pedantic and restriction
	cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::restriction

# Generate documentation
docs: ## Generate and open documentation
	cargo doc --open --no-deps
