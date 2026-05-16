{
  perSystem =
    {
      cargoArtifacts,
      crane,
      lib,
      self',
      workspace,
      ...
    }:
    let
      # This source is built from a fileset explicitly including directories
      # with migrations in them.
      #
      # This is because test targets need them to build, but directories with
      # migrations contain .sql files that are filtered out by crane since .sql
      # is not typically Rust source.
      src = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions [
          (crane.fileset.commonCargoSources ../.)
          ../tests/migrations/migrations01
          ../examples/simple_lib/migrations
        ];
      };
    in
    {
      checks = {
        # Build the main package as a check.
        inherit (self'.packages) tern;

        tern-clippy = crane.cargoClippy {
          inherit (workspace) pname version;
          inherit cargoArtifacts src;
          strictDeps = true;
          cargoClippyExtraArgs = "--all-features --all-targets";
        };

        tern-test = crane.cargoTest {
          inherit (workspace) pname version;
          inherit cargoArtifacts src;
          strictDeps = true;
          cargoTestExtraArgs = "--all-features --all-targets";
        };
      };
    };
}
