# KF/x Package Dockerfile
# This Dockerfile builds a deb package for Xen and KF/x
# for a given distro.

# IMAGE is an argument of the distro to build for, e.g. 'ubuntu:jammy'
ARG IMAGE

FROM $IMAGE AS base

ENV USER root

RUN cp /etc/apt/sources.list /etc/apt/sources.list.d/sources-src.list && \
    sed -i 's|deb http|deb-src http|g' /etc/apt/sources.list.d/sources-src.list

FROM base as deps

ARG DEBIAN_FRONTEND=noninteractive
ENV TZ=America/New_York

# Install basic dependencies for running the rest of the setup
# process
RUN apt-get -y update && apt-get -y install \
    autoconf \
    autoconf-archive \
    automake \
    bash \
    bc \
    bcc \
    bin86 \
    binutils \
    bison \
    bridge-utils \
    build-essential \
    bzip2 \
    cabextract \
    cmake \
    e2fslibs-dev \
    flex \
    gawk \
    gcc-multilib \
    gettext \
    git \
    iasl \
    iproute2 \
    kpartx \
    libaio-dev \
    libbz2-dev \
    libc6-dev \
    libc6-dev-i386 \
    libcurl4-openssl-dev \
    libfdt-dev \
    libfuse-dev \
    libglib2.0-dev \
    libgnutls28-dev \
    libjson-c-dev \
    liblzma-dev \
    libncurses5-dev \
    libpci-dev \
    libpixman-1-dev \
    # libsdl-dev \
    libsdl1.2-dev \
    libssl-dev \
    libsystemd-dev \
    libtool \
    libunwind-dev \
    libvncserver-dev \
    libx11-dev \
    libyajl-dev \
    linux-libc-dev \
    lsb-release \
    nasm \
    ninja-build \
    ocaml \
    ocaml-findlib \
    patch \
    pkg-config \
    # python-is-python2 \
    python3-dev \
    python3-pip \
    snap \
    tightvncserver \
    uuid-dev \
    uuid-runtime \
    wget \
    x11vnc \
    xtightvncviewer \
    xz-utils \
    zlib1g-dev

# This is required for buster, but doesn't hurt other distros
RUN echo 'APT::Cache-Limit "100000000";' >> /etc/apt/apt.conf.d/70debconf && \
    echo 'APT::Cache-Start "100000000";' >> /etc/apt/apt.conf.d/70debconf && \
    apt-get -y update && apt-get -y build-dep xen

RUN update-ca-certificates

FROM deps AS xen-builder

COPY xen /build/xen

WORKDIR /build/xen

RUN echo CONFIG_EXPERT=y > xen/.config && \
    echo CONFIG_MEM_SHARING=y >> xen/.config

# --enable-ovmf on all but 22.04
RUN ./configure --disable-pvshim --enable-systemd \
    --enable-githttp

RUN make -C xen olddefconfig && \
    make -j$(nproc) dist-xen && \
    make -j$(nproc) dist-tools && \
    make -j$(nproc) install-xen && \
    make -j$(nproc) install-tools

RUN mkdir -p dist/install/etc/ld.so.conf.d && \
    echo "/usr/local/lib" > dist/install/etc/ld.so.conf.d/xen.conf

RUN cp tools/misc/xen-vmtrace dist/install/usr/local/sbin

RUN mkdir -p /debs && \
    make -j$(nproc) debball && \
    cp dist/*.deb /debs

RUN if [ $(ls -A "/debs/" | wc -l) -eq 0 ]; then \
    echo "No deb files found in /debs/, Xen build failed!"; \
    exit 1; \
fi

FROM deps as kfx-deps

# Install go
RUN wget https://go.dev/dl/go1.19.3.linux-amd64.tar.gz && \
    tar -C /usr/local -xzf go1.19.3.linux-amd64.tar.gz

ENV PATH "${PATH}:/usr/local/go/bin"

FROM kfx-deps as kfx-builder

# Copy the deb file from the xen build and extract it
COPY --from=xen-builder /debs/ /debs/
RUN dpkg-deb -xv /debs/* /xen

RUN mkdir -p /output/ && \
    echo "/output/lib/" > /etc/ld.so.conf.d/build.conf && \
    echo "/xen/usr/local/lib/" >> /etc/ld.so.conf.d/build.conf && \
    ldconfig

# Settings for dynamic loader
ENV LD_LIBRARY_PATH="/xen/usr/local/lib:/output/lib/"
# Settings for GCC search paths
ENV LIBRARY_PATH="/xen/usr/local/lib:/output/lib/"
ENV C_INCLUDE_PATH="/xen/usr/local/include:/output/include/"
ENV CPLUS_INCLUDE_PATH="/xen/usr/local/include:/output/include/"
ENV PKG_CONFIG_PATH="/xen/usr/local/lib/pkgconfig/:/output/lib/pkgconfig/"

COPY dwarf2json /build/dwarf2json
WORKDIR /build/dwarf2json
RUN go build -buildvcs=false && \
    mkdir -p /output/bin && \
    cp dwarf2json /output/bin/
RUN if [ ! -f "/output/bin/dwarf2json" ]; then \
    echo "dwarf2json build failed!"; \
    exit 1; \
fi


# Build Capstone and install to deb prefix and the system
COPY capstone /build/capstone
RUN mkdir -p capstone/build
WORKDIR /build/capstone/build
RUN cmake -DCMAKE_INSTALL_PREFIX=/output \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_BUILD_TYPE=Release /build/capstone/ && \
    make -C /build/capstone/build/ -j$(nproc) && \
    make install
RUN if [ ! -f /output/lib/libcapstone.a ]; then \
        echo "No file libcapstone.a found, Capstone build failed!"; \
        exit 1; \
    fi

# Build libxdc and install to deb prefix and the system
COPY libxdc /build/libxdc
WORKDIR /build/libxdc/
RUN make PREFIX=/output/ install
RUN if [ ! -f /output/lib/libxdc.a ]; then \
        echo "No file libxdc.a found, libxdc build failed!"; \
        exit 1; \
    fi

# Build libvmi and install to deb prefix
COPY libvmi /build/libvmi
WORKDIR /build/libvmi/
RUN autoreconf -vif && \
    ./configure --disable-kvm --disable-file --disable-bareflank --disable-vmifs \
        --prefix=/output/ && \
    make -j$(nproc) && \
    make install
RUN if [ ! -f /output/lib/libvmi.a ]; then \
        echo "No file libvmi.a found, libvmi build failed!"; \
        exit 1; \
    fi

# Build KF/x and install to deb prefix
COPY src /build/kfx/src
COPY configure.ac Makefile.am /build/kfx/
WORKDIR /build/kfx/
RUN autoreconf -vif && \
    ./configure --prefix=/output/ && \
    make -j$(nproc) && \
    make install

RUN if [ ! -f /output/bin/kfx ]; then \
        echo "No file kfx found, KF/x build failed!"; \
        exit 1; \
    fi

FROM deps AS deb-builder

ARG KFX_VERSION
ENV KFX_VERSION ${KFX_VERSION}

COPY --from=xen-builder /debs /xen-debs
COPY --from=kfx-builder /output /kfx/usr/local/

RUN mkdir -p /debs/

# Setup xen deb directory
RUN dpkg-deb -xv /xen-debs/* /xen
COPY package/resource/xen/etc/default/grub.d/xen.cfg /xen/etc/default/grub.d/xen.cfg
COPY package/resource/xen/etc/modules-load.d/xen.conf /xen/etc/modules-load.d/xen.conf
COPY package/resource/xen/usr/bin/kfx-find-xen-defaults /xen/usr/bin/kfx-find-xen-defaults
RUN chmod +x /xen/usr/bin/kfx-find-xen-defaults

# Create xen deb
COPY package/resource/xen/postinst /xen/DEBIAN/postinst
COPY package/resource/xen/postrm /xen/DEBIAN/postrm
RUN export XENVERSION=$(ls /xen-debs | head -n 1 |awk '{ n=split($0,a,"-"); split(a[n],b,"."); print b[1]"."b[2]"."b[3]; }') && \
    export ARCH=$(dpkg --print-architecture) && \
    mkdir -p /xen/DEBIAN && \
    echo "Package: kfx-xen" > /xen/DEBIAN/control && \
    echo "Source: kfx-xen" >> /xen/DEBIAN/control && \
    echo "Version: ${XENVERSION}" >> /xen/DEBIAN/control && \
    echo "Architecture: ${ARCH}" >> /xen/DEBIAN/control && \
    echo "Maintainer: UNMAINTAINED: KF/x Team <rowan.hart@intel.com>" >> /xen/DEBIAN/control && \
    echo "Section: misc" >> /xen/DEBIAN/control && \
    echo "Priority: optional" >> /xen/DEBIAN/control && \
    echo "Description: Xen for KFx" >> /xen/DEBIAN/control && \
    echo "Depends: libpixman-1-0, libpng16-16, libnettle6 | libnettle7 | libnettle8, libgnutls30, libfdt1, libyajl2, libaio1" >> /xen/DEBIAN/control && \
    echo "Conflicts: xen-hypervisor-4.16-${ARCH}, xen-hypervisor-4.15-${ARCH}, xen-hypervisor-4.14-${ARCH}, xen-hypervisor-4.13-${ARCH}, xen-hypervisor-4.12-${ARCH}, xen-hypervisor-4.11-${ARCH}, xen-hypervisor-4.10-${ARCH}, xen-hypervisor-4.9-${ARCH}" >> /xen/DEBIAN/control && \
    echo "Installed-Size: $(du -ks /xen/ | awk '{print $1}')" >> /xen/DEBIAN/control && \
    find /xen/etc -type f -printf '%P\n' | xargs -i echo '/etc/{}' > /xen/DEBIAN/conffiles && \
    chown -R root:root /xen/ && \
    dpkg-deb --build -z0 /xen "/debs/kfx-xen_${XENVERSION}-${KFX_VERSION}_${ARCH}.deb"

# Create combined deb
COPY package/resource/bundle/postinst /xen/DEBIAN/postinst
RUN export XENVERSION=$(ls /xen-debs | head -n 1 |awk '{ n=split($0,a,"-"); split(a[n],b,"."); print b[1]"."b[2]"."b[3]; }') && \
    export ARCH=$(dpkg --print-architecture) && \
    tar -C /kfx/usr/local/ -cf /kfx.tar bin include lib && \
    tar -C /xen/usr/local/ -xf /kfx.tar && \
    mkdir -p /xen/DEBIAN && \
    echo "Package: kfx-bundle" > /xen/DEBIAN/control && \
    echo "Source: kfx-bundle" >> /xen/DEBIAN/control && \
    echo "Version: ${XENVERSION}" >> /xen/DEBIAN/control && \
    echo "Architecture: ${ARCH}" >> /xen/DEBIAN/control && \
    echo "Maintainer: UNMAINTAINED: KF/x Team <rowan.hart@intel.com>" >> /xen/DEBIAN/control && \
    echo "Section: misc" >> /xen/DEBIAN/control && \
    echo "Priority: optional" >> /xen/DEBIAN/control && \
    echo "Description: Xen for KFx" >> /xen/DEBIAN/control && \
    echo "Depends: libpixman-1-0, libpng16-16, libnettle6 | libnettle7 | libnettle8, libgnutls30, libfdt1, libyajl2, libaio1, libfuse2, liblzma5, libpcre3, libunwind8, zlib1g" >> /xen/DEBIAN/control && \
    echo "Conflicts: xen-hypervisor-4.16-${ARCH}, xen-hypervisor-4.15-${ARCH}, xen-hypervisor-4.14-${ARCH}, xen-hypervisor-4.13-${ARCH}, xen-hypervisor-4.12-${ARCH}, xen-hypervisor-4.11-${ARCH}, xen-hypervisor-4.10-${ARCH}, xen-hypervisor-4.9-${ARCH}" >> /xen/DEBIAN/control && \
    echo "Installed-Size: $(du -ks /xen/ | awk '{print $1}')" >> /xen/DEBIAN/control && \
    find /xen/etc -type f -printf '%P\n' | xargs -i echo '/etc/{}' > /xen/DEBIAN/conffiles && \
    chown -R root:root /xen/ && \
    dpkg-deb --build -z0 /xen "/debs/kfx-bundle_${XENVERSION}-${KFX_VERSION}_${ARCH}.deb"

# Create KF/x deb
COPY package/resource/kfx/postinst /kfx/DEBIAN/postinst
RUN mkdir -p /kfx/DEBIAN && \
    mkdir -p /kfx/etc/ld.so.conf.d/ && \
    echo "/usr/local/lib" >> /kfx/etc/ld.so.conf.d/kfx.conf && \
    export ARCH=$(dpkg --print-architecture) && \
    echo "Package: kfx" > /kfx/DEBIAN/control && \
    echo "Source: kfx" >> /kfx/DEBIAN/control && \
    echo "Version: ${KFX_VERSION}" >> /kfx/DEBIAN/control && \
    echo "Architecture: ${ARCH}" >> /kfx/DEBIAN/control && \
    echo "Maintainer: UNMAINTAINED: KF/x Team <rowan.hart@intel.com>" >> /kfx/DEBIAN/control && \
    echo "Section: misc" >> /kfx/DEBIAN/control && \
    echo "Priority: optional" >> /kfx/DEBIAN/control && \
    echo "Description: Kernel Fuzzer for Xen" >> /kfx/DEBIAN/control && \
    echo "Depends: libfuse2, liblzma5, libpcre3, libunwind8, zlib1g" >> /kfx/DEBIAN/control && \
    echo "Installed-Size: $(du -ks /kfx/ | awk '{print $1}')" >> /kfx/DEBIAN/control && \
    echo "/etc/ld.so.conf.d/kfx.conf" > /kfx/DEBIAN/conffiles && \
    chown -R root:root /kfx/ && \
    dpkg-deb --build -z0 /kfx "/debs/kfx_${KFX_VERSION}_${ARCH}.deb"
