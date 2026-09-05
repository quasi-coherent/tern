{
  description = "Dev inputs";
  inputs = {
    dev-flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "dev-nixpkgs";
    };
    dev-nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "dev-nixpkgs";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "dev-nixpkgs";
  };
  nixConfig = {
    extra-substituters = [
      "https://fenix.cachix.org"
      "https://tern.cachix.org"
    ];
    extra-trusted-public-keys = [
      "tern.cachix.org-1:wkC6dqWR8tLGcrTI40AOPQ48BdZaYXP/aen9znVbAMc="
      "fenix.cachix.org-1:ecJhr+RdYEdcVgUkjruiYhjbBloIEGov7bos90cZi0Q="
    ];
  };
  outputs = _: { };
}
