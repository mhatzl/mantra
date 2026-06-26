# Note: DATABASE_PATH set in ".env".
db_path := justfile_directory() + "/mantra.db"
db_url := "sqlite://" + db_path + "?mode=rwc"
test_db_path := justfile_directory() + "/mantra_test.db"
test_db_url := "sqlite://" + test_db_path + "?mode=rwc"

db-reset:
    rm -f {{ db_path }}
    cargo sqlx database create --database-url {{ db_url }}
    cargo sqlx migrate run --source mantra/migrations --database-url {{ db_url }}
    just db-prep

# Only preparing data for the mantra crate, because queries are only used in this crate
# and publishing crates only includes content inside a crate.
# See: https://github.com/launchbadge/sqlx/issues/3644
[working-directory: 'mantra']
db-prep:
    cargo sqlx prepare --database-url {{ db_url }}

profraw-file := justfile_directory() + "/target/nextest/default/raw-coverage/profdata-%p-%m.profraw"

testcov:
    rm -rf target/nextest/default
    mkdir -p target/nextest/default/coverage/raw-coverage
    - RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="{{ profraw-file }}" cargo nextest run
    grcov . -s . --binary-path ./target/debug/ -t html -t cobertura-pretty --ignore-not-existing -o ./target/nextest/default/coverage/ --ignore='/**/*' --ignore='target/*'

collect:
    cargo run -p mantra -- --db-url={{ test_db_url }} collect --product-id="mantra@main" --product-version="latest"

report:
    rm -rf target/mantra-report
    cargo run -p mantra -- --db-url={{ test_db_url }} report --formats=json --formats=html  --output-dir=target/mantra-report/

rm-test-db:
    rm -f {{ test_db_path }}

# Call: `just setup-test-db <copied path to cfg file>`
setup-test-db CFG DB="test_db":
    cargo run -p mantra -- --db-url="sqlite://{{ DB }}.db?mode=rwc" --config={{ CFG }} collect
