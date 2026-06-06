{
  outputs = inputs: import ./. inputs;

  inputs = {
    actions-nix = {
      url = "github:nialov/actions.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-substituters = [ "https://tern.cachix.org" ];
    extra-trusted-public-keys = [
      "tern.cachix.org-1:wkC6dqWR8tLGcrTI40AOPQ48BdZaYXP/aen9znVbAMc="
    ];
  };
}
