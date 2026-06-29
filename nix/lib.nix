{ inputs, lib, ... }:
let
  mkCargoSource =
    {
      crane,
      cargoRoot,
      extraSources ? [ ],
      ...
    }:
    lib.fileset.toSource {
      root = cargoRoot;
      fileset = lib.fileset.unions (
        [
          (crane.fileset.commonCargoSources cargoRoot)
        ]
        ++ extraSources
      );
    };

  mkCrane' =
    {
      pkgs,
      rustToolchain ? null,
    }:
    let
      system = pkgs.stdenvNoCC.hostPlatform.system;
      crane' = inputs.crane.mkLib pkgs;
      toolchain =
        if isNull rustToolchain then inputs.fenix.packages.${system}.stable.toolchain else rustToolchain;
      crane = crane'.overrideToolchain toolchain;
    in
    {
      inherit crane toolchain;
    };

  mkBuildArgs' =
    {
      pkgs,
      cargoRoot,
      rustToolchain ? null,
      extraSources ? [ ],
      cargoExtraArgs ? "--locked",
      cargoBuildExtraArgs ? "",
    }:
    let
      inherit (mkCrane' { inherit pkgs rustToolchain; }) crane;
      crate = crane.crateNameFromCargoToml { src = crane.cleanCargoSource cargoRoot; };
      src = mkCargoSource {
        inherit
          lib
          crane
          cargoRoot
          extraSources
          ;
      };
      buildArgs = {
        inherit (crate) pname version;
        inherit cargoBuildExtraArgs cargoExtraArgs src;
        strictDeps = true;
      };
      cargoArtifacts = crane.buildDepsOnly {
        inherit (buildArgs)
          pname
          version
          src
          strictDeps
          cargoExtraArgs
          cargoBuildExtraArgs
          ;
      };
    in
    {
      inherit crane buildArgs cargoArtifacts;
    };

  forPkgs = pkgs: {
    mkCrane =
      {
        rustToolchain ? null,
      }:
      mkCrane' { inherit pkgs rustToolchain; };

    mkBuildArgs =
      {
        cargoRoot,
        rustToolchain ? null,
        extraSources ? [ ],
        cargoExtraArgs ? "--locked",
        cargoBuildExtraArgs ? "",
      }:
      mkBuildArgs' {
        inherit
          pkgs
          cargoRoot
          rustToolchain
          extraSources
          cargoExtraArgs
          cargoBuildExtraArgs
          ;
      };
  };
in
{
  inherit forPkgs;
}
