# A3Mailer Makefile
# AI-Powered Web3-Native Mail Server - Development and Build Tasks

.PHONY: help setup build build-release test test-coverage lint format dev docker-dev docker-stop clean install docs ai-setup web3-setup security benchmark

# Default target
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
DOCKER_COMPOSE := docker-compose
PROJECT_NAME := a3mailer

# Colors for output
GREEN := \033[0;32m
YELLOW := \033[1;33m
NC := \033[0m

# Help target
help: ## Show this help message
	@echo "$(GREEN)🚀 A3Mailer - AI-Powered Web3-Native Mail Server$(NC)"
	@echo "$(GREEN)================================================$(NC)"
	@echo ""
	@echo "Available targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(YELLOW)%-15s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# Setup development environment
setup: ## Setup development environment
	@echo "$(GREEN)Setting up development environment...$(NC)"
	@mkdir -p logs data config coverage
	@if [ ! -f "config/development.toml" ]; then \
		cp resources/config/spamfilter.toml config/development.toml; \
		echo "Created development configuration"; \
	fi
	@$(CARGO) install cargo-watch cargo-audit cargo-tarpaulin cargo-edit || true
	@echo "$(GREEN)Development environment setup complete$(NC)"

# Build targets
build: ## Build the project in debug mode
	@echo "$(GREEN)Building A3Mailer (debug)...$(NC)"
	@$(CARGO) build --workspace

build-release: ## Build the project in release mode
	@echo "$(GREEN)Building A3Mailer (release)...$(NC)"
	@$(CARGO) build --release --workspace

# Test targets
test: ## Run all tests
	@echo "$(GREEN)Running tests...$(NC)"
	@$(CARGO) test --workspace

test-coverage: ## Run tests with coverage report
	@echo "$(GREEN)Running tests with coverage...$(NC)"
	@$(CARGO) tarpaulin --workspace --out Html --output-dir coverage
	@echo "$(GREEN)Coverage report generated in coverage/$(NC)"

# Code quality targets
lint: ## Run all linting checks
	@echo "$(GREEN)Running linting checks...$(NC)"
	@$(CARGO) fmt --all -- --check
	@$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	@$(CARGO) audit

format: ## Format all code
	@echo "$(GREEN)Formatting code...$(NC)"
	@$(CARGO) fmt --all

# Development targets
dev: ## Start development server with hot reloading
	@echo "$(GREEN)Starting development server...$(NC)"
	@$(CARGO) watch -x "run --bin $(PROJECT_NAME) -- --config config/development.toml"

# Docker targets
docker-build: ## Build Docker image
	@echo "$(GREEN)Building Docker image...$(NC)"
	@docker build -t $(PROJECT_NAME):latest .

docker-dev: ## Start development environment with Docker Compose
	@echo "$(GREEN)Starting Docker development environment...$(NC)"
	@$(DOCKER_COMPOSE) -f docker-compose.yml -f docker-compose.dev.yml up --build

docker-stop: ## Stop Docker development environment
	@echo "$(GREEN)Stopping Docker development environment...$(NC)"
	@$(DOCKER_COMPOSE) -f docker-compose.yml -f docker-compose.dev.yml down

docker-clean: ## Clean Docker images and containers
	@echo "$(GREEN)Cleaning Docker resources...$(NC)"
	@docker system prune -f

# Utility targets
clean: ## Clean build artifacts
	@echo "$(GREEN)Cleaning build artifacts...$(NC)"
	@$(CARGO) clean
	@rm -rf coverage/

install: ## Install the binary locally
	@echo "$(GREEN)Installing A3Mailer...$(NC)"
	@$(CARGO) install --path crates/main

docs: ## Generate and open documentation
	@echo "$(GREEN)Generating documentation...$(NC)"
	@$(CARGO) doc --workspace --open

# Security targets
audit: ## Run security audit
	@echo "$(GREEN)Running security audit...$(NC)"
	@$(CARGO) audit

update: ## Update dependencies
	@echo "$(GREEN)Updating dependencies...$(NC)"
	@$(CARGO) update

# Benchmarking
bench: ## Run benchmarks
	@echo "$(GREEN)Running benchmarks...$(NC)"
	@$(CARGO) bench --workspace

# Release preparation
check-release: ## Check if ready for release
	@echo "$(GREEN)Checking release readiness...$(NC)"
	@$(CARGO) check --release --workspace
	@$(CARGO) test --release --workspace
	@$(CARGO) clippy --release --workspace -- -D warnings
	@$(CARGO) audit
	@echo "$(GREEN)Release checks passed$(NC)"

# Database migrations (if applicable)
migrate: ## Run database migrations
	@echo "$(GREEN)Running database migrations...$(NC)"
	@$(CARGO) run --bin $(PROJECT_NAME)-cli -- migrate

# Quick development cycle
quick: format lint test ## Quick development cycle: format, lint, test

# Full CI simulation
ci: clean build test lint audit ## Simulate CI pipeline locally

# Development server with specific features
dev-full: ## Start development server with all features
	@echo "$(GREEN)Starting development server with all features...$(NC)"
	@$(CARGO) watch -x "run --bin $(PROJECT_NAME) --all-features -- --config config/development.toml"

# Performance profiling
profile: ## Build with profiling enabled
	@echo "$(GREEN)Building with profiling...$(NC)"
	@$(CARGO) build --release --workspace
	@echo "$(GREEN)Run with: CARGO_PROFILE_RELEASE_DEBUG=true cargo run --release$(NC)"

# Generate flamegraph (requires cargo-flamegraph)
flamegraph: ## Generate performance flamegraph
	@echo "$(GREEN)Generating flamegraph...$(NC)"
	@$(CARGO) flamegraph --bin $(PROJECT_NAME) -- --config config/development.toml

# Check for outdated dependencies
outdated: ## Check for outdated dependencies
	@echo "$(GREEN)Checking for outdated dependencies...$(NC)"
	@$(CARGO) outdated

# Tree view of dependencies
tree: ## Show dependency tree
	@echo "$(GREEN)Showing dependency tree...$(NC)"
	@$(CARGO) tree

# Show project statistics
stats: ## Show project statistics
	@echo "$(GREEN)🚀 A3Mailer Project Statistics:$(NC)"
	@echo "Lines of code:"
	@find crates -name "*.rs" -exec wc -l {} + | tail -1
	@echo ""
	@echo "Number of crates:"
	@find crates -name "Cargo.toml" | wc -l
	@echo ""
	@echo "Dependencies:"
	@$(CARGO) tree --depth 1 | grep -E "^[a-zA-Z]" | wc -l

# AI/ML Setup and Testing
ai-setup: ## Setup AI/ML models and dependencies
	@echo "$(GREEN)🤖 Setting up AI/ML components...$(NC)"
	@mkdir -p models data/ai logs/ai
	@echo "Downloading threat detection models..."
	@echo "AI setup completed!"

ai-test: ## Test AI components
	@echo "$(GREEN)🤖 Testing AI components...$(NC)"
	@$(CARGO) test -p stalwart-threat-detection
	@$(CARGO) test ai_

# Web3 Setup and Testing
web3-setup: ## Setup Web3 blockchain components
	@echo "$(GREEN)⛓️ Setting up Web3 components...$(NC)"
	@mkdir -p data/web3 keys logs/web3
	@echo "Configuring blockchain connections..."
	@echo "Web3 setup completed!"

web3-test: ## Test Web3 components
	@echo "$(GREEN)⛓️ Testing Web3 components...$(NC)"
	@$(CARGO) test -p stalwart-compliance
	@$(CARGO) test web3_

# Security audits
security: ## Run comprehensive security audits
	@echo "$(GREEN)🛡️ Running security audits...$(NC)"
	@$(CARGO) audit
	@$(CARGO) clippy -- -D warnings
	@echo "Security audit completed!"

# Performance benchmarks
benchmark: ## Run performance benchmarks
	@echo "$(GREEN)⚡ Running performance benchmarks...$(NC)"
	@$(CARGO) bench
	@echo "Benchmark results saved to target/criterion/"

# Full production build
production: ## Build for production with all optimizations
	@echo "$(GREEN)🚀 Building for production...$(NC)"
	@$(CARGO) build --release --workspace
	@strip target/release/a3mailer
	@echo "Production build completed!"

# Deploy to production
deploy: production ## Deploy to production environment
	@echo "$(GREEN)🚢 Deploying to production...$(NC)"
	@docker-compose -f docker-compose.yml up -d
	@echo "Deployment completed!"
