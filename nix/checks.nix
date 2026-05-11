{ ... }:
{
  perSystem =
    {
      crane,
      cargoArtifacts,
      manifest,
      self',
      ternSrcExtras,
      ...
    }:
    {
      checks = {
        # Build all workspace members as part of checks.
        inherit (self'.packages)
          tern
          tern-cli
          tern-core
          tern-derive
          tern-executor
          ;

        tern-clippy = crane.cargoClippy {
          inherit (manifest) pname version;
          inherit cargoArtifacts;
          src = ternSrcExtras;
          strictDeps = true;
          cargoClippyExtraArgs = "--all-features --all-targets -- -Dwarnings";
        };

        tern-test = crane.cargoTest {
          inherit (manifest) pname version;
          inherit cargoArtifacts;
          src = ternSrcExtras;
          strictDeps = true;
          cargoTestExtraArgs = "--all-features --all-targets";
        };
      };
    };
}
