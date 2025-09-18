																																																																																																																																																																																																																																																																																																																																																											# Makefile for local development

.PHONY: up build down clean

up: build
	@echo "Building release binary and bringing up docker compose..."
	cargo build --release && docker compose up --build

build:
	@echo "Building release binary..."
	cargo build --release

down:
	@echo "Tearing down docker compose"
	docker compose down

clean:
	@echo "Cleaning target and docker resources"
	rm -rf target
	docker compose down --rmi local -v --remove-orphans
