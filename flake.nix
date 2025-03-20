{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, ... }:


    flake-utils.lib.eachDefaultSystem (system:

      let
        pkgs = nixpkgs.legacyPackages.${system};
        clang = pkgs.llvmPackages_17.clang;
        pythonEnv = pkgs.python3.withPackages (ps: []);

        toolchain = with fenix.packages.${system}; combine [
          minimal.cargo
          minimal.rustc
          minimal.rust-std
          targets.wasm32-unknown-unknown.latest.rust-std
	  targets.x86_64-unknown-linux-musl.latest.rust-std
        ];

        version = (builtins.fromTOML (builtins.readFile ./engine/Cargo.toml)).workspace.package.version;

        appleDeps = with pkgs.darwin.apple_sdk.frameworks; [
          CoreServices
          SystemConfiguration
          pkgs.libiconv-darwin
        ];

        rustPlatform = pkgs.makeRustPlatform {
          inherit (fenix.packages.${system}.minimal) cargo rustc;
          inherit (fenix.packages.${system}.latest) rust-std;
        };

	# wasm-bindgen-cli = pkgs.rustPlatform.buildRustPackage rec {
	#   pname = "wasm-bindgen-cli";
	#   version = "0.2.92";
	#   src = pkgs.fetchFromGitHub {
	#     owner = "rustwasm";
	#     repo = "wasm-bindgen";
	#     rev = "${version}";
	#     sha256 = "sha256-VMt+J5sazHPqmAdsoueS2WW6Pn1tvugaJaPnSJq9038=";
	#   };
	#   cargoHash = "sha256-+iIHleftJ+Yl9QHEBVI91NOhBw9qtUZfgooHKoyY1w4=";
	#   buildInputs = with pkgs; [ openssl ];
	#   nativeBuildInputs = with pkgs; [ pkg-config ];
	#   cargoBuildFlags = ["--package wasm-bindgen-cli"];
	# };

        buildInputs = (with pkgs; [
          git
          openssl
          pkg-config
          lld_17
          pythonEnv
          ruby
          maturin
          nodePackages.pnpm
          nodePackages.nodejs
          toolchain
          nodejs
          uv
          wasm-pack
          pkgs.gcc
          napi-rs-cli
	  wasm-bindgen-cli

	  # For building the typescript client.
	  pixman
	  cairo
	  pango
	  libjpeg
	  giflib
	  librsvg
        ]) ++ (if pkgs.stdenv.isDarwin then appleDeps else []);
        nativeBuildInputs = [
          pkgs.openssl
          pkgs.pkg-config
          pkgs.ruby
          pythonEnv
          pkgs.maturin
        ];

      in
        {
          packages.default = rustPlatform.buildRustPackage {
            pname = "baml-cli";
            version = version;
            src = let
              extraFiles = pkgs.copyPathToStore ./engine/baml-runtime/src/cli/initial_project/baml_src;
            in pkgs.symlinkJoin {
              name = "source";
              paths = [ ./engine extraFiles ];
            };
            LIBCLANG_PATH = pkgs.libclang.lib + "/lib/";
            BINDGEN_EXTRA_CLANG_ARGS = if pkgs.stdenv.isDarwin then
              "-I${pkgs.llvmPackages_17.libclang.lib}/lib/clang/17/headers "
            else
              "-isystem ${pkgs.llvmPackages_17.libclang.lib}/lib/clang/17/include -isystem ${pkgs.glibc.dev}/include";

            cargoLock = { lockFile = ./engine/Cargo.lock; outputHashes = {
            }; };

            # Add build-time environment variables
            RUSTFLAGS = if pkgs.stdenv.isDarwin
              then
                "--cfg tracing_unstable -C linker=lld"
              else
                "--cfg tracing_unstable -Zlinker-features=+lld -C linker=gcc";

            OPENSSL_STATIC = "1";
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
            OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";

            # Modify the test phase to only run library tests
            checkPhase = ''
              runHook preCheck
              echo "Running cargo test --lib"
              cargo test --lib
              runHook postCheck
            '';

            inherit buildInputs;
            inherit nativeBuildInputs;

            PYTHON_SYS_EXECUTABLE="${pythonEnv}/bin/python3";
            LD_LIBRARY_PATH="${pythonEnv}/lib";
            PYTHONPATH="${pythonEnv}/${pythonEnv.sitePackages}";
            CC="${clang}/bin/clang";

          };
          devShell = pkgs.mkShell rec {
            inherit buildInputs;
            PATH="${clang}/bin:$PATH";
            LIBCLANG_PATH = pkgs.libclang.lib + "/lib/";
            BINDGEN_EXTRA_CLANG_ARGS = if pkgs.stdenv.isDarwin then
              "-I${pkgs.llvmPackages_17.libclang.lib}/lib/clang/17/headers "
            else
              "-isystem ${pkgs.llvmPackages_17.libclang.lib}/lib/clang/17/include -isystem ${pkgs.glibc.dev}/include";
            RUSTFLAGS = if pkgs.stdenv.isDarwin
              then
                "--cfg tracing_unstable -C linker=lld"
              else
                "--cfg tracing_unstable -Zlinker-features=+lld -C linker=gcc";
          };
        }
    );
}
