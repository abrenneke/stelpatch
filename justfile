set shell := ["pwsh", "-c"]

bench:
  cd cw_games && cargo bench

profile:
  #!pwsh -c
  $PATH = cd cw_games && cargo bench --no-run --message-format=json-render-diagnostics \
    | jq -js '[.[] | select(.reason=="compiler-artifact") | select(.executable != null) | select(.target.kind | map(.=="bench") | any)] | last | .executable'
  samply record $PATH --bench --profile-time 10

load-stellaris:
  cd cw_games && cargo run --release --bin load_stellaris