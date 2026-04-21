{
  perSystem.treefmt = {
    projectRootFile = "flake.nix";
    programs = {
      rustfmt = {
        enable = true;
        edition = "2024";
      };
      nixfmt.enable = true;
      typos.enable = true;
    };
  };
}
