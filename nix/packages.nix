{ inputs, ... }:
let
  perSystem =
    {
      inputs',
      pkgs,
      ...
    }:
    let
      rust-stable = inputs'.fenix.packages.stable;
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rust-stable.toolchain;

      src = craneLib.cleanCargoSource ../.;

      inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;

      args = {
        inherit src version;
        strictDeps = true;
      };

      # Creating a package of only the dependencies ensures that the derivation
      # exists to make it able to be cached.
      cargoArtifacts = craneLib.buildDepsOnly args;

      tern = craneLib.buildPackage {
        inherit (args)
          src
          strictDeps
          version
          ;
        inherit cargoArtifacts;
        pname = "tern";
      };

      mkTernPackage =
        pname:
        craneLib.buildPackage {
          inherit (args)
            src
            strictDeps
            version
            ;
          inherit pname cargoArtifacts;
          cargoExtraArgs = "-p ${pname}";
        };
    in
    {
      checks = {
        lint = craneLib.cargoClippy {
          inherit src cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- -Dwarnings";
        };
      };

      packages = {
        inherit rust-stable tern;

        default = tern;

        tern-deps = cargoArtifacts;

        tern-cli = mkTernPackage "tern-cli";
        tern-core = mkTernPackage "tern-core";
        tern-derive = mkTernPackage "tern-derive";
      };
    };
in
{
  inherit perSystem;
}
