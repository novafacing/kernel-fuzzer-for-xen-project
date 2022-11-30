#!/bin/bash
#
# mkdeb: package the dist/install output of a Xen build in a .deb

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "${SCRIPT_DIR}/common.sh"

usage() {
    echo "usage: mkdeb.sh <distro> <version>"
    echo ""
    echo "  distro: A distro name, for example 'jammy'"
    echo "  version: A version string, for example 'v0.0.1'"
    fail
}

DISTRO="${1}"
VERSION="${2}"

if [ -z "${DISTRO}" ]; then
    usage
fi

if [ -z "${VERSION}" ]; then
    usage
fi


ARCH=amd64

## BUILD XEN DEB

# Prepare the directory to package
cp -a /dist-xen deb

# Debian doesn't use /usr/lib64 for 64-bit libraries
if test -d deb/usr/lib64 ; then
  cp -a deb/usr/lib64/* deb/usr/lib/
  rm -rf deb/usr/lib64
fi

XENVERSION=$(ls deb/boot/*.config | xargs basename | awk -F'\.c' '{print $1}' | awk -F'-' '{print $2}')

# Fill in the debian boilerplate
mkdir -p deb/DEBIAN
cat >deb/DEBIAN/control <<EOF
Package: xen-hypervisor
Source: xen-hypervisor
Version: $XENVERSION
Architecture: $ARCH
Maintainer: Unmaintained snapshot
Depends: libpixman-1-0, libpng16-16, libnettle6 | libnettle7, libgnutls30, libfdt1, libyajl2, libaio1
Conflicts: xen-hypervisor-4.6-amd64, xen-hypervisor-4.7-amd64, xen-hypervisor-4.8-amd64, xen-hypervisor-4.9-amd64, xen-hypervisor-4.10-amd64, xen-hypervisor-4.11-amd64, xen-hypervisor-4.12-amd64
Section: admin
Priority: optional
Installed-Size: $(du -ks deb | cut -f1)
Description: Xen Hypervisor built for KFX
EOF

mkdir -p deb/etc/default/grub.d/
mkdir -p deb/etc/modules-load.d/
cp package/extra/etc/default/grub.d/xen.cfg deb/etc/default/grub.d/
cp package/extra/etc/modules-load.d/xen.conf deb/etc/modules-load.d/
cp package/extra/usr/bin/kfx-find-xen-defaults deb/usr/bin/

# Find all /etc files and add them to conffiles
find deb/etc -type f -printf /etc/%P\\n >deb/DEBIAN/conffiles
chmod +x package/postinst
chmod +x package/postrm
cp package/postinst deb/DEBIAN/postinst
cp package/postrm deb/DEBIAN/postrm

# Package it up
chown -R root:root deb
dpkg-deb --build -z0 deb "xen-$DISTRO.deb"
mv *.deb /out

log_info "Generated xen deb package"

## KFX, LibVMI & tools

# Fill in the debian boilerplate
mkdir -p deb/DEBIAN
cat >deb/DEBIAN/control <<EOF
Package: kfx-bundle
Source: kfx-bundle
Version: $VERSION
Architecture: $ARCH
Maintainer: Unmaintained snapshot
Depends: libglib2.0-dev, libjson-c3 | libjson-c4, libpixman-1-0, libpng16-16, libnettle6 | libnettle7, libgnutls30, libfdt1, libyajl2, libaio1
Conflicts: xen-hypervisor-4.6-amd64, xen-hypervisor-4.7-amd64, xen-hypervisor-4.8-amd64, xen-hypervisor-4.9-amd64, xen-hypervisor-4.10-amd64, xen-hypervisor-4.11-amd64, xen-hypervisor-4.12-amd64
Section: admin
Priority: optional
Installed-Size: $(du -ks deb | cut -f1)
Description: KFX bundle
EOF

mkdir -p deb/usr/bin/
cp -avr /build/usr/bin/* deb/usr/bin/

mkdir -p deb/usr/lib/
cp -avr /build/usr/lib/* deb/usr/lib/

mkdir -p deb/usr/include/
cp -avr /build/usr/include/* deb/usr/include/

cp -avr /build/dwarf2json/dwarf2json deb/usr/bin/

# Package it up
chown -R root:root deb
dpkg-deb --build -z0 deb "kfx-$DISTRO.deb"
mv *.deb /out
rm -rf deb

log_info "Generated kfx deb package"
