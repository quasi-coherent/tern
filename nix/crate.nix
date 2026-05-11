{ ... }:
let
  perSystem =
    {
      cargoArtifacts,
      craneNightly,
      manifest,
      mkTernPackage,
      ternSrc,
      ...
    }:
    let
      tern = mkTernPackage "tern";
      tern-cli = mkTernPackage "tern-cli";
      tern-core = mkTernPackage "tern-core";
      tern-derive = mkTernPackage "tern-derive";
      tern-executor = mkTernPackage "tern-executor";
    in
    {
      packages = {
        inherit
          tern
          tern-cli
          tern-core
          tern-derive
          tern-executor
          ;

        default = tern;

        tern-docs = craneNightly.cargoDoc {
          inherit (manifest) pname version;
          inherit cargoArtifacts;
          src = ternSrc;
          cargoExtraArgs = "-Zunstable-options --cfg=docsrs";
        };
      };
    };
in
{
  inherit perSystem;
}
