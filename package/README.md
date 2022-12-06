# Packaging

This directory contains packaging utilities to build .deb packages for KF/x. Due to
restrictions, we do not publish these .deb files, but you may choose to do so.

## Dependencies

The packaging processes requires that you have `docker` installed as well as
[act](https://github.com/nektos/act) to run the packaging workflow locally.

## Running the Workflow

The workflow for packaging is not located in the .github directory, so you can
invoke the packaging workflow with:

```
$ act -W package/package.yml
```