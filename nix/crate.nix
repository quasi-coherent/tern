{
  perSystem =
    {
      cargoArtifacts,
      crane,
      src,
      workspace,
      ...
    }:
    let
      tern = crane.buildPackage {
        inherit (workspace) pname version;
        inherit cargoArtifacts src;
        cargoBuildExtraArgs = "--all-features";
        strictDeps = true;
      };
    in
    {
      packages = {
        inherit tern;
        default = tern;
      };
    };
}
