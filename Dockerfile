# Base image with build tools
FROM --platform=$BUILDPLATFORM rust:1.88-alpine AS base

# Dependencies:
# bash, curl: Installing cargo-binstall
# git: Getting version and build information during build
# musl-dev: Required for linking
RUN apk add bash curl git musl-dev

# `wasm-bindgen-cli` for `aarch64` only compiled for glibc; install `gcompat`
RUN if [[ "$(uname -m)" == "aarch64" ]]; then apk add gcompat; fi

# Copy rust-toolchain to install and use the specified toolchain
COPY rust-toolchain.toml ./

# Rust toolchain for wasm
RUN rustup target add wasm32-unknown-unknown

# cargo chef and trunk
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall cargo-chef trunk

WORKDIR /app


# Plan required dependencies with cargo chef
FROM base AS plan
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


# Build the project
FROM base AS build

# Build dependencies first
# This allows caching built dependencies and reusing them when no dependency changed
COPY --from=plan /app/recipe.json recipe.json
RUN cargo chef cook --release --target wasm32-unknown-unknown --bin naviz-gui --recipe-path recipe.json

# Build the gui website using trunk
COPY . .
WORKDIR /app/gui
RUN trunk build --release


# The container that will be deployed
FROM nginx:stable-alpine AS deployment

COPY --from=build /app/gui/dist /usr/share/nginx/html
