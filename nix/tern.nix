let
  perSystem =
    { craneLib, ... }:
    let
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

      tern-cli = mkTernPackage "tern-cli";
      tern-core = mkTernPackage "tern-core";
      tern-derive = mkTernPackage "tern-derive";
    in
    {
      packages = {
        inherit
          tern
          tern-cli
          tern-core
          tern-derive
          ;

        tern-deps = cargoArtifacts;
      };
    };
in
{
  inherit perSystem;
}
