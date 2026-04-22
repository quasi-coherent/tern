{
  perSystem.treefmt = {
    projectRootFile = "flake.nix";
    programs = {
      rustfmt.enable = true;
      nixfmt.enable = true;
      typos.enable = true;
    };
    settings.formatter.rustfmt = {
      options = [
        "--config-path"
        "rustfmt.toml"
      ];
    };
  };
}
