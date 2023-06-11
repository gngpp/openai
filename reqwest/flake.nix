{
  description = "Build env";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustVersion = "1.65.0";

        rust = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [
            "rust-src" # rust-analyzer
          ];
        };

        nixLib = nixpkgs.lib;
        craneLib = (crane.mkLib pkgs).overrideToolchain rust;


        envVars = rec {
          RUST_BACKTRACE = 1;
          MOLD_PATH = "${pkgs.mold.out}/bin/mold";
          RUSTFLAGS = "-Clink-arg=-fuse-ld=${MOLD_PATH} -Clinker=clang";
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };

        # Allow more files to be included in the build workspace
        workspaceSrc = ./.;
        # workspaceSrcString = builtins.toString workspaceSrc;

        workspaceFilter = path: type:
          (craneLib.filterCargoSources path type);

        # The main application derivation
        reqwest-impersonate = craneLib.buildPackage
          (rec {
            src = nixLib.cleanSourceWith
              {
                src = workspaceSrc;
                filter = workspaceFilter;
              };

            doCheck = false;

            buildInputs = with pkgs;
              [
                openssl
                boringssl
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ ];

            nativeBuildInputs = with pkgs;
              [
                clang
                pkg-config
              ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ ];

            LD_LIBRARY_PATH = nixLib.makeLibraryPath buildInputs;
          } // envVars);
      in
      {
        checks = {
          inherit reqwest-impersonate;
        };

        packages.default = reqwest-impersonate;


        devShells.rust = pkgs.mkShell ({
          nativeBuildInputs = [
            rust
          ];
        } // envVars);


        devShells.default = reqwest-impersonate;
      });
}



