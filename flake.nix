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
        flake-utils.follows = "flake-utils";
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

          nativeBuildInputs = [ ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs; [
            darwin.apple_sdk.frameworks.SystemConfiguration
          ]);
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        nxrm_two_portal = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });

        mavenSettingsFile = pkgs.writeText "settings.xml" ''
          <settings xmlns="http://maven.apache.org/SETTINGS/1.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
          xsi:schemaLocation="http://maven.apache.org/SETTINGS/1.0.0 https://maven.apache.org/xsd/settings-1.0.0.xsd">
            <servers>
              <server>
                 <id>central.testing</id>
	               <username>fake_username</username>
	               <password>fake_password</password>
               </server>
             </servers>
           </settings>
        '';

        mvnLocal = pkgs.writeShellApplication {
          name = "mvnLocal";

          runtimeInputs = with pkgs; [ maven ];

          text = ''
            mvn \
              --settings='${mavenSettingsFile}' \
              "$@";
          '';
        };
      in
      rec {
        checks = {
          inherit nxrm_two_portal;

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

        # uncomment if there is a binary to be run
        # apps.nxrm_two_portal = flake-utils.lib.mkApp {
        #   drv = packages.nxrm_two_portal;
        #   name = "nxrm_two_portal";
        # };
        # apps.default = apps.nxrm_two_portal;

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks.${system};

          packages = with pkgs; [
            rustToolchain
            cargo-edit
            cargo-msrv
            cargo-outdated

            # GitHub tooling
            gh

            # Nix tooling
            nixpkgs-fmt

            # Java tooling
            maven
            jdk17
            gnupg

            mvnLocal
          ];
        };
      });
}
