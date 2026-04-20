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
            wayland-protocols
            wayland
            libGL

            libxcb
            libX11
            libX11
            libXrandr
            libXinerama
            libXcursor
            libXi

            libxkbcommon

            libglvnd
            spirv-tools
            vulkan-volk
            vulkan-tools
            vulkan-loader
            vulkan-headers
            vulkan-validation-layers
        ]; 
      in
        {
        defaultPackage = naersk-lib.buildPackage ./.;
        devShell = with pkgs; mkShell {
          nativeBuildInputs = [
            wayland-protocols
            libxkbcommon
            wayland
          ] ++ libPath;

          buildInputs = [ cargo rustc rustfmt pre-commit rustPackages.clippy wasm-pack pkg-config];

          env = {
            RUST_SRC_PATH = rustPlatform.rustLibSrc;

            LD_LIBRARY_PATH = if pkgs.stdenv.isDarwin 
              then []
              else pkgs.lib.makeLibraryPath libPath;

            VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
            VULKAN_SDK = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
          };
        };
      }
    );
}
