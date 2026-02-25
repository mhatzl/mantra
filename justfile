
db-reset:
    rm -f mantra.db
    cargo sqlx database create -D "sqlite://mantra.db?mode=rwc"
    cargo sqlx migrate run --source mantra/migrations -D "sqlite://mantra.db?mode=rwc"
    cargo sqlx prepare --workspace

db-prep:
    cargo sqlx prepare --workspace

profraw-file := justfile_directory() + "/target/nextest/default/raw-coverage/profdata-%p-%m.profraw"

testcov:
    rm -rf target/nextest/default
    mkdir -p target/nextest/default/coverage/raw-coverage
    - RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="{{ profraw-file }}" cargo nextest run -p mantra
    grcov . -s . --binary-path ./target/debug/ -t html -t cobertura-pretty --ignore-not-existing -o ./target/nextest/default/coverage/ --ignore='/**/*' --ignore='target/*'

collect:
    cargo run -p mantra -- --db-url="sqlite://mantra_test.db?mode=rwc" collect

report:
    cargo run -p mantra -- --db-url="sqlite://mantra_test.db?mode=rwc" report --formats=json --output-path=target/report.json
