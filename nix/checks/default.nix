{ pkgs, ternLib' }:
let
  build = ternLib'.mkBuildArgs {
    cargoRoot = ../../.;
    extraSources = [
      ../../tests/migrations/mysql/plain
      ../../tests/migrations/mysql/updown
      ../../tests/migrations/pg/plain
      ../../tests/migrations/pg/updown
      ../../tests/migrations/sqlite/plain
      ../../tests/migrations/sqlite/updown
    ];
  };
  inherit (build) buildArgs crane cargoArtifacts;

  tests = pkgs.callPackage ./test-it.nix { inherit buildArgs cargoArtifacts crane; };
in
{
  inherit (tests) ternTests tern-doit;
}
