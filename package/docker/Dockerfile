# KF/x Package Dockerfile
# This Dockerfile builds a deb package for Xen and KF/x
# for a given distro.

# IMAGE is an argument of the distro to build for, e.g. 'ubuntu:jammy'
ARG IMAGE

FROM $IMAGE AS base

ARG DEBIAN_FRONTEND=noninteractive

ENV USER root
ENV PATH "/$USER/.cargo/bin:$PATH"

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

COPY . /kfx
WORKDIR /kfx

RUN git submodule update --init --recursive

# Sanity check that we just copied the correct directory
RUN grep -q "Tamas K Lengyel" AUTHORS \
    || { echo "docker build run directory not KF/x repo root!"; exit 1; }



FROM base AS scripts

COPY package /package
# Export the manifest path for running scripts
ENV MANIFEST /package/Cargo.toml
# Speed up cargo registry clone
ENV CARGO_NET_GIT_FETCH_WITH_CLI true

RUN cargo build --release --manifest-path=$MANIFEST

FROM base AS deps

WORKDIR /kfx

COPY --from=scripts /package/target/release/dependencies /scripts/dependencies

# Install dependencies
RUN /scripts/dependencies

FROM deps AS xen-builder

WORKDIR /kfx

COPY --from=scripts /package/target/release/xen /scripts/xen

# Build Xen
RUN cp -a /kfx/xen /xen
WORKDIR /xen
RUN /scripts/xen

FROM deps AS kfx-builder

ARG KFX_VERSION

WORKDIR /kfx

COPY --from=scripts /package/target/release/kfx /scripts/kfx

# The deb directory is the build directory for the xen deb package, which the kfx
# package is a superset of
COPY --from=xen-builder /deb /deb
COPY --from=xen-builder /out /out

WORKDIR /kfx
RUN /scripts/kfx