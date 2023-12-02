{ nixpkgs ? import <nixpkgs> { }}:
let
	pinnedPkgs = nixpkgs.fetchFromGitHub {
		owner  = "NixOS";
		repo   = "nixpkgs";
		rev    = "4ecab3273592f27479a583fb6d975d4aba3486fe";
		sha256 = "btHN1czJ6rzteeCuE/PNrdssqYD2nIA4w48miQAFloM=";
	};
	pkgs = import pinnedPkgs {};
in pkgs.mkShell {
	buildInputs = with pkgs; [
		cargo
		cacert # Required to download crates
		gnutar
		zip
		pv
	];
}

