#!/bin/sh

set -e

apt-get update
apt-get install -y lsb-release patch

SYSTEM=$(lsb_release -is)
DISTRIBUTION=$(lsb_release -cs)

if [ "$SYSTEM" = "Debian" ]
then
    echo "deb-src http://deb.debian.org/debian ${DISTRIBUTION} main" >> /etc/apt/sources.list
    apt-get update
else
    sed -i 's/# deb-src/deb-src/g' /etc/apt/sources.list
    apt-get update
fi

apt-get --quiet --yes install git build-essential libfdt-dev libpixman-1-dev libssl-dev libsdl1.2-dev autoconf libtool xtightvncviewer tightvncserver x11vnc uuid-runtime uuid-dev bridge-utils python3-dev liblzma-dev libc6-dev wget git bcc bin86 gawk iproute2 libcurl4-openssl-dev bzip2 libpci-dev libc6-dev libc6-dev-i386 linux-libc-dev zlib1g-dev libncurses5-dev patch libvncserver-dev libssl-dev libsdl-dev iasl libbz2-dev e2fslibs-dev ocaml libx11-dev bison flex ocaml-findlib xz-utils gettext libyajl-dev libpixman-1-dev libaio-dev libfdt-dev cabextract libglib2.0-dev autoconf automake libtool libjson-c-dev libfuse-dev liblzma-dev autoconf-archive kpartx python3-pip libsystemd-dev cmake snap gcc-multilib nasm binutils bc libunwind-dev ninja-build

wget -O /usr/local/go1.15.3.linux-amd64.tar.gz https://golang.org/dl/go1.15.3.linux-amd64.tar.gz
tar -C /usr/local -xzf /usr/local/go1.15.3.linux-amd64.tar.gz

HAS_PYTHON_IS_PYTHON=$(apt-cache search --names-only '^python-is-python2$')

if [ ! -z "$HAS_PYTHON_IS_PYTHON" ]
then
    apt-get --quiet --yes install python-is-python2
fi

# libgnutls28 is required for the password-protected VNC to work in Xen 4.16+.
# See: https://bugs.gentoo.org/832494
apt-get install -y libgnutls28-dev
apt-get --quiet --yes build-dep xen
apt-get autoremove -y
apt-get clean

rm -rf /var/lib/apt/lists* /tmp/* /var/tmp/*
