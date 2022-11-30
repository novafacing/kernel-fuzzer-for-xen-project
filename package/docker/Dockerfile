# KF/x Package Dockerfile
# This Dockerfile builds a deb package for Xen and KF/x
# for a given distro.

# IMAGE is an argument of the distro to build for, e.g. 'ubuntu:jammy'
ARG IMAGE

FROM $IMAGE AS base

# Set with ARG instead of ENV to avoid issues at runtime
ARG DEBIAN_FRONTEND=noninteractive

# Everything is run as root
ENV USER root

# Install basic dependencies for running the rest of the setup
# process
RUN apt-get -y update && apt-get -y install \
    bash \
    build-essential \
    lsb-release \
    curl

# Install rust to run setup scripts
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH "/$USER/.cargo/bin:$PATH"

COPY . /kfx
# Sanity check that we just copied the correct directory
RUN grep -q "Tamas K Lengyel" /kfx/AUTHORS \
    || { echo "docker build run directory not KF/x repo root!"; exit 1; }

# Build the install scripts and 
ENV MANIFEST /kfx/package/scripts/Cargo.toml
RUN cargo build --manifest-path=$MANIFEST

# Install dependencies
RUN cargo run --manifest-path=$MANIFEST --bin dependencies

FROM base AS xen-builder

RUN cp -a /kfx/xen /xen

WORKDIR /xen

# Build Xen
RUN cargo run --manifest-path=$MANIFEST --bin xen


