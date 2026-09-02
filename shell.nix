{ pkgs ? import <nixpkgs> {} }:

let
  pgData = ".pgdata";
  pgPort = "5433";
  dbName = "cono_dev_db";
  dbUser = "dev";
in
pkgs.mkShell {
  buildInputs = [
    pkgs.cargo
    pkgs.cargo-flamegraph
    pkgs.rustc
    pkgs.rustfmt
    pkgs.sea-orm-cli
    pkgs.python313
    pkgs.python313Packages.requests
    pkgs.nodejs
    pkgs.fontconfig

    pkgs.postgresql
  ];

  RUST_BACKTRACE = 1;
  TMPDIR = "/tmp";

  # Create a Postgres instance to dev against
  DATABASE_URL = "postgres://${dbUser}@localhost:${pgPort}/${dbName}";

  shellHook = ''
    export PGDATA="$PWD/${pgData}"
    export PGPORT=${pgPort}

    if [ ! -d "$PGDATA" ]; then
      echo "Initializing postgres data dir at $PGDATA"
      initdb -D "$PGDATA" \
        --username=${dbUser} \
        --auth=trust
    fi

    if ! pg_ctl status -D "$PGDATA" > /dev/null 2>&1; then
      echo "Starting postgres"
      pg_ctl -D "$PGDATA" \
        -l "$PGDATA/postgres.log" \
        -o "-k $PGDATA" \
        -W \
        start
    fi

    # Ensure DB exists
    if ! psql -h localhost -p ${pgPort} -U ${dbUser} -lqt | cut -d \| -f 1 | grep -qw ${dbName}; then
      echo "Creating database ${dbName}"
      createdb -h localhost -p ${pgPort} -U ${dbUser} ${dbName}
    fi

    echo "Postgres ready"
    echo "DATABASE_URL=$DATABASE_URL"
  '';

  # How to kill the db if needed:
  # pg_ctl stop -D $PGDATA -m fast
}
