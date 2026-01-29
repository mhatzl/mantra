
db-reset:
    rm -f mantra.db
    cargo sqlx database create -D "sqlite://mantra.db?mode=rwc"
    cargo sqlx migrate run --source mantra/migrations -D "sqlite://mantra.db?mode=rwc"
    cargo sqlx prepare --workspace

db-prep:
    cargo sqlx prepare --workspace
