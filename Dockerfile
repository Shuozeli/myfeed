# syntax=docker/dockerfile:1
#
# Multi-stage Dockerfile for myfeed testing
#
# Build stages:
#   base     - Runtime dependencies (protoc, sqlite, etc.) + rustup
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
FROM debian:bookworm-slim AS base

# Install system dependencies required for building
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libsqlite3-dev \
    pkg-config \
    libssl-dev \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

# Verify tools are installed
RUN protoc --version
RUN /root/.cargo/bin/rustc --version

# Set up PATH for rustup
ENV PATH="/root/.cargo/bin:${PATH}"

# Create app directory
WORKDIR /app

# Copy manifests for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto

# Pre-fetch dependencies (cargo fetch downloads but doesn't compile)
RUN /root/.cargo/bin/cargo fetch

# ============================================================================
# Stage 2: Full build
# ============================================================================
FROM base AS builder

# Copy full source
COPY . .

# Build the application
RUN /root/.cargo/bin/cargo build --release

# ============================================================================
# Stage 3: Test environment
# ============================================================================
FROM base AS test

# Copy source and build
COPY . .
RUN /root/.cargo/bin/cargo build --release

# Default: run all tests
CMD ["/root/.cargo/bin/cargo", "test"]

# ============================================================================
# Alternative: Run only unit tests (no browser needed)
# ============================================================================
# FROM base
# COPY . .
# RUN cargo test --lib
