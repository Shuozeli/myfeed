# syntax=docker/dockerfile:1
#
# Multi-stage Dockerfile for myfeed testing
#
# Build stages:
#   base     - Runtime dependencies (protoc, sqlite, etc.)
#   builder  - Full Rust build environment
#   test     - Test execution environment
#
# Usage:
#   docker build -t myfeed:test .
#   docker run --rm myfeed:test cargo test --lib
#   docker compose -f docker-compose.test.yml run integration-test

# ============================================================================
# Stage 1: Base image with system dependencies
# ============================================================================
FROM rust:1.86-slim-bookworm AS base

# Install system dependencies required for building
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libsqlite3-dev \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Verify protoc is installed
RUN protoc --version

# Create app directory
WORKDIR /app

# Copy manifests for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto

# Pre-compile dependencies (cached layer)
# This compiles all dependencies without the actual application code
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null
RUN rm -rf src target/debug/.fingerprint/myfeed-*

# ============================================================================
# Stage 2: Full build
# ============================================================================
FROM base AS builder

# Copy full source
COPY . .

# Build the application
RUN cargo build --release

# ============================================================================
# Stage 3: Test environment
# ============================================================================
FROM base AS test

# Copy source and build
COPY . .
RUN cargo build --release

# Default: run all tests
CMD ["cargo", "test"]

# ============================================================================
# Alternative: Run only unit tests (no browser needed)
# ============================================================================
# FROM base
# COPY . .
# RUN cargo test --lib
