{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
        libPath = with pkgs; [
          libffi
          spirv-tools
          vulkan-volk
          vulkan-tools
          vulkan-loader
          vulkan-headers
          vulkan-validation-layers
          shader-slang
        ] ++ (if pkgs.stdenv.isLinux then (with pkgs; [
          wayland-protocols
          wayland.dev
          wayland
          libGL
          libxcb
          libX11
          libXrandr
          libXinerama
          libXcursor
          libXi
          libxkbcommon
          libglvnd
        ]) else []); 

      in
        {
        defaultPackage = naersk-lib.buildPackage ./.;
        devShell = pkgs.mkShell rec {
          packages = with pkgs; [
            bacon
          ];
          nativeBuildInputs = [ ] ++ libPath;

          buildInputs = with pkgs; [ 
            cargo rustc rustfmt pre-commit rustPackages.clippy wasm-pack pkg-config 
            shader-slang 
            wayland
            wayland.dev libGL libxkbcommon
          ];

          env = {
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath libPath}";


            VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
            VULKAN_SDK = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
          };
        };
      }
    );
}
