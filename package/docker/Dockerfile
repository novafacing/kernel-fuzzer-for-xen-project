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
    pkg-config \
    libssl-dev \
    git \
    curl

# Install rust to run setup scripts
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH "/$USER/.cargo/bin:$PATH"

COPY . /kfx/
WORKDIR /kfx/
RUN git submodule update --init --recursive

# Sanity check that we just copied the correct directory
RUN grep -q "Tamas K Lengyel" AUTHORS \
    || { echo "docker build run directory not KF/x repo root!"; exit 1; }

# Speed up cargo registry clone
ENV CARGO_NET_GIT_FETCH_WITH_CLI true
# Export the manifest path for running scripts
ENV MANIFEST /kfx/package/scripts/Cargo.toml

# Install dependencies
RUN cargo run --manifest-path=$MANIFEST --bin dependencies

FROM base AS xen-builder

# Build Xen
RUN cp -a /kfx/xen /xen
WORKDIR /xen
RUN cargo run --manifest-path=$MANIFEST --bin xen

FROM base as kfx-builder

ARG KFX_VERSION

# The deb directory is the build directory for the xen deb package, which the kfx
# package is a superset of
COPY --from=xen-builder /deb /deb

WORKDIR /kfx
RUN cargo run --manifest-path=$MANIFEST --bin kfx


