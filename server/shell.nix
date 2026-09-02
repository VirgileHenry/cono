{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  PORT="8080";
  SERVE_PATH="../client/build";
  # DATABASE_URL is set by the upper shell (where the db is started)
}
