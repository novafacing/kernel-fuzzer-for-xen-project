# KF/x Package Dockerfile
# This Dockerfile builds a deb package for Xen and KF/x
# for a given distro.

# IMAGE is an argument of the distro to build for, e.g. 'ubuntu:jammy'
ARG IMAGE

FROM $IMAGE AS base

ARG DEBIAN_FRONTEND=noninteractive
ARG KFX_VERSION

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

RUN mkdir -p /kfx
WORKDIR /kfx

COPY AFL /kfx/AFL/
COPY .astylerc /kfx/.astylerc
COPY AUTHORS /kfx/AUTHORS
COPY capstone /kfx/capstone
COPY configure.ac /kfx/configure.ac
COPY COPYING /kfx/COPYING
COPY .dockerignore /kfx/.dockerignore
COPY dwarf2json /kfx/dwarf2json
COPY .git /kfx/.git
COPY .github /kfx/.github
COPY .gitignore /kfx/.gitignore
COPY .gitmodules /kfx/.gitmodules
COPY libvmi /kfx/libvmi
COPY libxdc /kfx/libxdc
COPY Makefile.am /kfx/Makefile.am
COPY patches /kfx/patches
COPY README /kfx/README
COPY README.md /kfx/README.md
COPY scripts /kfx/scripts
COPY security.md /kfx/security.md
COPY src /kfx/src
COPY testmodule /kfx/testmodule
COPY testmodule_win /kfx/testmodule_win
COPY tpp.txt /kfx/tpp.app
COPY xen /kfx/xen


RUN git submodule update --init --recursive

# Sanity check that we just copied the correct directory
RUN grep -q "Tamas K Lengyel" AUTHORS \
    || { echo "docker build run directory not KF/x repo root!"; exit 1; }

ENV MANIFEST /kfx/package/Cargo.toml

COPY package /kfx/package

RUN cargo build --release --manifest-path=$MANIFEST

RUN /kfx/package/target/release/package dependencies

RUN /kfx/package/target/release/package build-xen /kfx/xen /output

# RUN /kfx/package/target/release/package build-kfx /kfx /output $KFX_VERSION /xen-output/xen_*.deb