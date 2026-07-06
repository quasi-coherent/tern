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
      inherit cargoArtifacts crane buildArgs;
    };

  forPkgs =
    pkgs:
    let
      system = pkgs.stdenvNoCC.hostPlatform.system;
      stableToolchain = inputs.fenix.packages.${system}.stable.toolchain;
      nightlyToolchain = inputs.fenix.packages.${system}.latest.toolchain;

      mkCrane =
        {
          rustToolchain ? stableToolchain,
        }:
        mkCrane' { inherit pkgs rustToolchain; };

      mkBuildArgs =
        {
          cargoRoot,
          rustToolchain ? stableToolchain,
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
    in
    {
      inherit mkCrane mkBuildArgs;

      # Use the nightly toolchain by default.
      # Some common `cargo doc` features need it.
      mkDocs =
        {
          cargoRoot,
          rustToolchain ? nightlyToolchain,
          rustdocFlags ? "",
          cargoExtraArgs ? "--locked",
          cargoDocExtraArgs ? "--no-deps",
        }:
        let
          build = mkBuildArgs { inherit cargoExtraArgs cargoRoot rustToolchain; };
          inherit (build) cargoArtifacts crane buildArgs;

        in
        crane.cargoDoc {
          inherit cargoArtifacts cargoDocExtraArgs;
          inherit (buildArgs)
            pname
            version
            src
            strictDeps
            cargoExtraArgs
            ;
          env.RUSTDOCFLAGS = rustdocFlags;
        };
    };
in
{
  inherit forPkgs;
}
