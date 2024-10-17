{
  description = "Translate the subset of the NXRM2 API into the new Central Portal Publisher API";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-analyzer" "rust-src" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = craneLib.cleanCargoSource (craneLib.path ./.);

        commonArgs = {
          inherit src;

          # uncomment if the project is a workspace
          pname = "nxrm_two_portal";
          version = "0.1.0";

          buildInputs = with pkgs; [ libclang ];
          LIBCLANG_PATH="${pkgs.libclang}/lib";

          nativeBuildInputs = (with pkgs; [ cmake pkg-config git clang ]) ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs; [
            darwin.apple_sdk.frameworks.SystemConfiguration
            xcbuild
          ]);
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        nxrm_two_portal = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          # Run tests via cargo nextest
          doCheck = false;
        });

        mkMvn = { name, settingsFile }: pkgs.writeShellApplication {
          inherit name;

          runtimeInputs = with pkgs; [ maven ];

          text = ''
            #!/usr/bin/env bash
            git_root=$(git rev-parse --show-toplevel)
            settings_file="$git_root/${settingsFile}"
            mvn \
               --settings="$settings_file" \
              -Dcentral.url='http://localhost:2727'\
               "$@";
          '';
        };

        mvnLocalProxy = mkMvn {
          name = "mvnLocalProxy";
          settingsFile = "settings-local.xml";
        };

        mvnStagingProxy = mkMvn {
          name = "mvnStagingProxy";
          settingsFile = "settings-staging.xml";
        };

        mvnProductionProxy = mkMvn {
          name = "mvnProductionProxy";
          settingsFile = "settings-production.xml";
        };

        mkGradle = { name, credentialsFile }: pkgs.writeShellApplication {
          inherit name;

          runtimeInputs = with pkgs; [ gradle ];

          text = ''
            #!/usr/bin/env bash
            git_root=$(git rev-parse --show-toplevel)
            # shellcheck disable=SC1091
            source "$git_root/${credentialsFile}"
            gradle \
              -PcentralProxyUsername="$USERNAME" \
              -PcentralProxyPassword="$PASSWORD" \
              "$@";
          '';
        };

        gradleLocalProxy = mkGradle {
          name = "gradleLocalProxy";
          credentialsFile = "credentials-local";
        };

        gradleStagingProxy = mkGradle {
          name = "gradleStagingProxy";
          credentialsFile = "credentials-staging";
        };

        gradleProductionProxy = mkGradle {
          name = "gradleProductionProxy";
          credentialsFile = "credentials-production";
        };
      in
      rec {
        checks = {
          inherit nxrm_two_portal;

          nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
          });

          doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          fmt = craneLib.cargoFmt (commonArgs // {
            inherit src;
          });
        };

        packages.nxrm_two_portal = nxrm_two_portal;
        packages.default = packages.nxrm_two_portal;

        apps.nxrm_two_portal = flake-utils.lib.mkApp {
          drv = packages.nxrm_two_portal;
          name = "nxrm_two_portal";
        };
        apps.default = apps.nxrm_two_portal;

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks.${system};

          packages = with pkgs; [
            rustToolchain
            cargo-edit
            cargo-msrv
            cargo-outdated
            cargo-nextest

            # Orchestration
            just
            licensure

            # GitHub tooling
            gh

            # Nix tooling
            nixpkgs-fmt

            # Java tooling
            maven
            gradle
            jdk17
            gnupg

            mvnLocalProxy
            mvnStagingProxy
            mvnProductionProxy
            gradleLocalProxy
            gradleStagingProxy
            gradleProductionProxy
          ];
        };
      });
}
