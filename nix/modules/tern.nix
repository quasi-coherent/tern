{ ternLib }:
{
  config,
  ...
}:
let
  cfg = config.tern;
  perSystem =
    { pkgs, ... }:
    let
      build = (ternLib.forPkgs pkgs).mkBuildArgs {
        inherit (cfg)
          cargoRoot
          rustToolchain
          extraSources
          cargoExtraArgs
          cargoBuildExtraArgs
          ;
      };
    in
    {
      packages.ternApp = build.crane.buildPackage {
        inherit (build) cargoArtifacts;
        inherit (build.buildArgs)
          pname
          version
          src
          strictDeps
          cargoExtraArgs
          cargoBuildExtraArgs
          ;
      };
    };
in
{
  inherit perSystem;
}
