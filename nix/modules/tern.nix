{
  ternConfig,
  mkBuildArgs,
}:
let
  build = mkBuildArgs {
    inherit (ternConfig)
      cargoRoot
      rustToolchain
      extraSources
      cargoExtraArgs
      cargoBuildExtraArgs
      ;
  };
  inherit (build) buildArgs;
in
build.crane.buildPackage {
  inherit (build) cargoArtifacts;
  inherit (buildArgs)
    pname
    version
    src
    strictDeps
    cargoExtraArgs
    cargoBuildExtraArgs
    ;
}
