# Packaging

This directory contains packaging utilities to build .deb packages for KF/x. Due to
restrictions, we do not publish these .deb files, but you may choose to do so.

## Dependencies

The packaging processes requires that you have `docker` installed as well as
[act](https://github.com/nektos/act) to run the packaging workflow locally.

## Running With Script

There is a convenience script, `package/package.sh`

## Running Manually

### Running the Workflow

The workflow for packaging is not located in the .github directory, so you can
invoke the packaging workflow with:

```sh
$ act -W package/package.yml
```

### Retrieving Build Outputs

If you have a recent version of `act`, you can retrieve the build outputs by passing:

```sh
$ act -W package/package.yml --artifact-server-path=/PATH/TO/OUTPUT/
```

Due to size restrictions when checking out the local repository to run the workflow,
ensure the path is *not* a subdirectory of this repository!

You should see several files in the output:

```
ls /tmp/release/1/debs-*/debs/ 
/tmp/release/1/debs-bionic/debs/:
kfx_0.0.1-git-bionic_amd64.deb  kfx-bundle_4.16.1-0.0.1-git-bionic_amd64.deb.gz__  kfx-xen_4.16.1-0.0.1-git-bionic_amd64.deb.gz__

/tmp/release/1/debs-bullseye/debs/:
kfx_0.0.1-git-bullseye_amd64.deb  kfx-bundle_4.16.1-0.0.1-git-bullseye_amd64.deb.gz__  kfx-xen_4.16.1-0.0.1-git-bullseye_amd64.deb.gz__

/tmp/release/1/debs-buster/debs/:
kfx_0.0.1-git-buster_amd64.deb  kfx-bundle_4.16.1-0.0.1-git-buster_amd64.deb.gz__  kfx-xen_4.16.1-0.0.1-git-buster_amd64.deb.gz__

/tmp/release/1/debs-focal/debs/:
kfx_0.0.1-git-focal_amd64.deb  kfx-bundle_4.16.1-0.0.1-git-focal_amd64.deb.gz__  kfx-xen_4.16.1-0.0.1-git-focal_amd64.deb.gz__

/tmp/release/1/debs-jammy/debs/:
kfx_0.0.1-git-jammy_amd64.deb.gz__  kfx-bundle_4.16.1-0.0.1-git-jammy_amd64.deb.gz__  kfx-xen_4.16.1-0.0.1-git-jammy_amd64.deb.gz__
```

These files are compressed with `gzip` compression if over a certain size, and you can
decompress them as a batch by running:

```sh
$ find /PATH/TO/OUTPUT/ -type f -name '*.gz__' -exec sh -c 'mv "${0}" "${0%.gz__}.gz" && gunzip "${0%.gz__}.gz"' {} \;
