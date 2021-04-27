{ system ? builtins.currentSystem, pkgs ? import ./nix { inherit system; } }:
pkgs.mkShell {
  buildInputs = [
    pkgs.wasmd
    pkgs.pystarport
    (pkgs.poetry2nix.mkPoetryEnv { projectDir = ./integration_tests; })
  ] ++ (pkgs.lib.optionals pkgs.stdenv.isDarwin [
    pkgs.darwin.apple_sdk.frameworks.Security
    pkgs.darwin.libiconv
  ]);
}
