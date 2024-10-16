{
  description = "oak";
  inputs = {
    systems.url = "github:nix-systems/x86_64-linux";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-utils.inputs.systems.follows = "systems";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs = { self, systems, nixpkgs, flake-utils, rust-overlay, crane }:
    (flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              rust-overlay.overlays.default
            ];
            config = {
              android_sdk.accept_license = true; # accept all of the sdk licenses
              allowUnfree = true; # needed to get android stuff to compile
            };
          };
          linux_kernel_version = "6.9.1";
          linux_kernel_src = builtins.fetchurl {
            url = "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-${linux_kernel_version}.tar.xz";
            sha256 = "01b414ba98fd189ecd544435caf3860ae2a790e3ec48f5aa70fdf42dc4c5c04a";
          };
          linux_kernel_config = ./oak_containers/kernel/configs/${linux_kernel_version}/minimal.config;
          # Build the linux kernel for Oak Containers as a nix package, which simplifies
          # reproducibility.
          # Note that building a package via nix is not by itself a guarantee of
          # reproducibility; see https://reproducible.nixos.org.
          # Common kernel configuration
          commonLinuxKernelConfig = {
            # To allow reproducibility, the following options need to be configured:
            # - CONFIG_MODULE_SIG is not set
            # - CONFIG_MODULE_SIG_ALL is not set
            # - CONFIG_DEBUG_INFO_DWARF_TOOLCHAIN_DEFAULT is not set
            configfile = linux_kernel_config;
            # And also the following build variables.
            # See https://docs.kernel.org/kbuild/reproducible-builds.html.
            extraMakeFlags = [
              "KBUILD_BUILD_USER=user"
              "KBUILD_BUILD_HOST=host"
            ];
            version = linux_kernel_version;
            src = linux_kernel_src;
            allowImportFromDerivation = true;
          };
          # Patched kernel
          linux_kernel = pkgs.linuxManualConfig (commonLinuxKernelConfig // {
            kernelPatches = [{
              name = "virtio-dma";
              patch = ./oak_containers/kernel/patches/virtio-dma.patch;
            }
            {
              name = "tdx-skip-probe-roms";
              patch = ./oak_containers/kernel/patches/tdx-probe-roms.patch;
            }];
          });
          # Vanilla kernel
          vanilla_linux_kernel = pkgs.linuxManualConfig commonLinuxKernelConfig;
          androidSdk =
            (pkgs.androidenv.composeAndroidPackages {
              platformVersions = [ "30" ];
              buildToolsVersions = [ "30.0.0" ];
              includeEmulator = false;
              includeNDK = false;
              includeSources = false;
              includeSystemImages = false;
            }).androidsdk;
          rustToolchain =
            # This should be kept in sync with the value in bazel/rust/defs.bzl
            pkgs.rust-bin.nightly."2024-09-05".default.override {
              extensions = [
                "clippy"
                "llvm-tools-preview"
                "rust-analyzer"
                "rust-src"
                "rustfmt"
              ];
              targets = [
                "wasm32-unknown-unknown"
                "x86_64-unknown-linux-musl"
                "x86_64-unknown-none"
              ];
            };
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          src = ./.;
        in
        {
          packages = { inherit linux_kernel; inherit vanilla_linux_kernel; };
          formatter = pkgs.nixpkgs-fmt;
          # We define a recursive set of shells, so that we can easily create a shell with a subset
          # of the dependencies for specific CI steps, without having to pull everything all the time.
          #
          # To add a new dependency, you can search it on https://search.nixos.org/packages and add its
          # name to one of the shells defined below.
          devShells = rec {
            # Base shell with shared dependencies.
            base = with pkgs; mkShell {
              packages = [
                cachix
                just
                ps
                which
              ];
            };
            # Minimal shell with only the dependencies needed to run the Rust tests.
            rust = with pkgs; mkShell {
              inputsFrom = [
                base
              ];
              packages = [
                (rust-bin.selectLatestNightlyWith (toolchain: rustToolchain))
                cargo-audit
                cargo-deadlinks
                cargo-binutils
                cargo-deny
                cargo-nextest
                cargo-udeps
                cargo-vet
                protobuf
                buf # utility to convert binary protobuf to json; for breaking change detection.
                systemd
                qemu_kvm
                python312
                wasm-pack
              ];
            };
            # For some reason node does not know how to find the prettier plugin, so we need to
            # manually specify its fully qualified path.
            prettier = with pkgs; writeShellScriptBin "prettier" ''
              ${nodePackages.prettier}/bin/prettier \
              --plugin "${nodePackages.prettier-plugin-toml}/lib/node_modules/prettier-plugin-toml/lib/index.js" \
              "$@"
            '';
            # Minimal shell with only the dependencies needed to run the format and check-format
            # steps.
            lint = with pkgs; mkShell {
              packages = [
                bazel-buildtools
                cargo-deadlinks
                clang-tools
                hadolint
                ktfmt
                ktlint
                nixpkgs-fmt
                nodePackages.markdownlint-cli
                shellcheck
              ];
              buildInputs = [
                prettier
              ];
            };
            # Minimal shell with only the dependencies needed to run the bazel steps.
            bazelShell = with pkgs; mkShell {
              shellHook = ''
                export ANDROID_HOME="${androidSdk}/libexec/android-sdk"
                export GRADLE_OPTS="-Dorg.gradle.project.android.aapt2FromMavenOverride=${androidSdk}/libexec/android-sdk/build-tools/28.0.3/aapt2";
              '';
              packages = [
                autoconf
                autogen
                automake
                jdk11_headless
                bazel
                androidSdk
                bazel-buildtools
              ];
            };
            # Shell for building Oak Containers kernel and system image. This is not included in the
            # default shell because it is not needed as part of the CI.
            containers = with pkgs; mkShell {
              # We need access to the kernel source and configuration, not just the binaries, to
              # build the system image with nvidia drivers in it.
              # See oak_containers/system_image/build-base.sh (and nvidia_base_image.Dockerfile) for
              # more details.
              shellHook = ''
                export LINUX_KERNEL="${linux_kernel}"
                export VANILLA_LINUX_KERNEL="${vanilla_linux_kernel}"
                export LINUX_KERNEL_VERSION="${linux_kernel_version}"
                export LINUX_KERNEL_SOURCE="${linux_kernel_src}"
                export LINUX_KERNEL_CONFIG="${linux_kernel_config}"
              '';
              inputsFrom = [
                base
                bazelShell
                rust
              ];
              packages = [
                bc
                bison
                cpio
                curl
                docker
                elfutils
                fakeroot
                flex
                jq
                libelf
                perl
                strip-nondeterminism
                glibc
                glibc.static
                ncurses
                netcat
                umoci
              ];
            };
            # Shell for container kernel image provenance workflow.
            bzImageProvenance = with pkgs; mkShell {
              shellHook = ''
                export LINUX_KERNEL="${linux_kernel}"
                export VANILLA_LINUX_KERNEL="${vanilla_linux_kernel}"
              '';
              inputsFrom = [
                rust
              ];
              packages = [
                bc
                bison
                curl
                elfutils
                flex
                libelf
              ];
            };
            # Shell for container stage 1 image provenance workflow.
            stage1Provenance = with pkgs; mkShell {
              inputsFrom = [
                rust
              ];
              packages = [
                cpio
                glibc
                glibc.static
                strip-nondeterminism
              ];
            };
            systemImageProvenance = with pkgs; mkShell {
              inputsFrom = [
                rust
                bazelShell
              ];
              packages = [
                elfutils
              ];
            };
            # Shell for most CI steps (i.e. without contaniners support).
            ci = pkgs.mkShell {
              inputsFrom = [
                rust
                bazelShell
                lint
              ];
            };
            # By default create a shell with all the inputs.
            default = pkgs.mkShell {
              packages = [ ];
              inputsFrom = [
                containers
                rust
                bazelShell
                lint
              ];
            };
          };
        }));
}
