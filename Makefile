# JumpServer Client Tauri Project Makefile
# Author: ZhaoJiSen
# Version: 1.4.0

# Color definitions
RED=\033[0;31m
GREEN=\033[0;32m
YELLOW=\033[1;33m
BLUE=\033[0;34m
NC=\033[0m # No Color

# Project info
PROJECT_NAME=jumpserver-client
VERSION=1.4.0
NODE_VERSION=23
PNPM_VERSION=10.17.0

# Default target
.DEFAULT_GOAL := help

# Help info
.PHONY: help
help: ## Show help info
	@echo "$(GREEN)JumpServer Client Tauri Project$(NC)"
	@echo "$(BLUE)Version: $(VERSION)$(NC)"
	@echo ""
	@echo "$(YELLOW)Available commands:$(NC)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# Environment check
.PHONY: check-env
check-env: ## Check the dev environment
	@echo "$(BLUE)Checking dev environment...$(NC)"
	@command -v node >/dev/null 2>&1 || { echo "$(RED)Error: Node.js is not installed$(NC)"; exit 1; }
	@command -v pnpm >/dev/null 2>&1 || { echo "$(RED)Error: pnpm is not installed$(NC)"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)Error: Rust/Cargo is not installed$(NC)"; exit 1; }
	@command -v tauri >/dev/null 2>&1 || { echo "$(RED)Error: Tauri CLI is not installed$(NC)"; exit 1; }
	@echo "$(GREEN)✓ Environment check passed$(NC)"

# Install dependencies
.PHONY: install
install: check-env ## Install project dependencies
	@echo "$(BLUE)Installing frontend dependencies...$(NC)"
	pnpm install
	@echo "$(GREEN)✓ Dependency install complete$(NC)"

# Run dev mode
.PHONY: dev
dev: install ## Start dev mode (hot reload)
	@echo "$(BLUE)Starting Tauri dev mode...$(NC)"
	pnpm run tauri:dev

# Frontend dev only
.PHONY: dev-frontend
dev-frontend: install ## Start only the frontend dev server
	@echo "$(BLUE)Starting frontend dev server...$(NC)"
	pnpm run dev

# Build the project
.PHONY: build
build: install ## Build the production version
	@echo "$(BLUE)Building production version...$(NC)"
	pnpm run tauri:build
	@echo "$(GREEN)✓ Build complete$(NC)"

# Debug build
.PHONY: build-debug
build-debug: install ## Build the debug version
	@echo "$(BLUE)Building debug version...$(NC)"
	pnpm run tauri:build:debug
	@echo "$(GREEN)✓ Debug build complete$(NC)"

# Lint code
.PHONY: lint
lint: ## Run lint checks
	@echo "$(BLUE)Running lint checks...$(NC)"
	pnpm run lint
	@echo "$(GREEN)✓ Lint check complete$(NC)"

# Clean the project
.PHONY: clean
clean: ## Clean build files and dependencies
	@echo "$(BLUE)Cleaning project...$(NC)"
	pnpm run reset
	@echo "$(GREEN)✓ Clean complete$(NC)"

# Full fresh start
.PHONY: fresh-start
fresh-start: clean install dev ## Full fresh start (clean + install + dev)

# Version management
.PHONY: bump
bump: ## Bump the version number
	@echo "$(BLUE)Bumping version number...$(NC)"
	pnpm run bump
	@echo "$(GREEN)✓ Version bump complete$(NC)"

# Generate static files
.PHONY: generate
generate: ## Generate static files
	@echo "$(BLUE)Generating static files...$(NC)"
	pnpm run generate
	@echo "$(GREEN)✓ Static file generation complete$(NC)"

# Run cleanup script
.PHONY: cleanup
cleanup: ## Run the cleanup script
	@echo "$(BLUE)Running cleanup script...$(NC)"
	pnpm run cleanup
	@echo "$(GREEN)✓ Cleanup script complete$(NC)"

# Check Rust dependencies
.PHONY: check-rust
check-rust: ## Check Rust dependencies
	@echo "$(BLUE)Checking Rust dependencies...$(NC)"
	cd src-tauri && cargo check
	@echo "$(GREEN)✓ Rust dependency check complete$(NC)"

# Update Rust dependencies
.PHONY: update-rust
update-rust: ## Update Rust dependencies
	@echo "$(BLUE)Updating Rust dependencies...$(NC)"
	cd src-tauri && cargo update
	@echo "$(GREEN)✓ Rust dependency update complete$(NC)"

# Run Rust tests
.PHONY: test-rust
test-rust: ## Run Rust tests
	@echo "$(BLUE)Running Rust tests...$(NC)"
	cd src-tauri && cargo test
	@echo "$(GREEN)✓ Rust tests complete$(NC)"

# Format Rust code
.PHONY: fmt-rust
fmt-rust: ## Format Rust code
	@echo "$(BLUE)Formatting Rust code...$(NC)"
	cd src-tauri && cargo fmt
	@echo "$(GREEN)✓ Rust code formatting complete$(NC)"

# Check Rust code
.PHONY: clippy
clippy: ## Run Rust clippy checks
	@echo "$(BLUE)Running Rust clippy checks...$(NC)"
	cd src-tauri && cargo clippy
	@echo "$(GREEN)✓ Rust clippy check complete$(NC)"

# Set up the dev environment
.PHONY: setup-dev
setup-dev: check-env install ## Set up the dev environment
	@echo "$(GREEN)✓ Dev environment setup complete$(NC)"
	@echo "$(YELLOW)Tip: run 'make dev' to start developing$(NC)"

# Show project info
.PHONY: info
info: ## Show project info
	@echo "$(GREEN)Project info:$(NC)"
	@echo "  Name: $(PROJECT_NAME)"
	@echo "  Version: $(VERSION)"
	@echo "  Node.js: $(NODE_VERSION)"
	@echo "  pnpm: $(PNPM_VERSION)"
	@echo ""
	@echo "$(BLUE)Tech stack:$(NC)"
	@echo "  Frontend: Nuxt 3 + Vue 3 + TypeScript"
	@echo "  Backend: Tauri + Rust"
	@echo "  Package manager: pnpm"
	@echo "  Build tool: Vite"

# Quick dev command
.PHONY: quick-dev
quick-dev: ## Quick dev (skip environment check)
	@echo "$(BLUE)Quickly starting dev mode...$(NC)"
	pnpm run tauri:dev

# Watch mode
.PHONY: watch
watch: ## Watch for file changes and auto-restart
	@echo "$(BLUE)Starting watch mode...$(NC)"
	pnpm run dev -- --watch

# Production preview
.PHONY: preview
preview: build ## Preview the production build
	@echo "$(BLUE)Starting production preview...$(NC)"
	pnpm run tauri:dev -- --mode production
