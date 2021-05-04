{ sources ? import ./sources.nix, system ? builtins.currentSystem }:
import sources.nixpkgs {
  overlays = [
    (_: pkgs: rec {
      wasmvm = pkgs.rustPlatform.buildRustPackage rec {
        name = "wasmvm";
        src = sources.wasmvm;
        cargoSha256 = sha256:1xdxx0w6swfn714nbjf3i69jp3hzpvlrik84wnxsz5qc710frix2;
        buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.libiconv
        ];
        doCheck = false;
      };
      wasmd = pkgs.buildGoModule rec {
        name = "wasmd";
        src = sources.wasmd;
        subPackages = [ "cmd/wasmd" ];
        vendorSha256 = sha256:1g7g5wpilciywm2j5sgjx676v4mdbwmibm2z3vpl4vapmgjsxh1f;
        doCheck = false;
        preFixup = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
          patchelf --set-rpath "${wasmvm}/lib" $out/bin/wasmd
        '' + pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
          install_name_tool -change @rpath/libwasmvm.dylib "${wasmvm}/lib/libwasmvm.dylib" $out/bin/wasmd
        '';
      };
    })
    (_: pkgs: {
      pystarport = (import sources.chain-main { inherit system pkgs; }).pystarport-unbind;
    })
  ];
  config = { };
  inherit system;
}
