fleet_commander: target/release/fleet_commander
	cp target/release/fleet_commander .
	strip fleet_commander

target/release/fleet_commander: src resources Cargo.lock Cargo.toml
	cargo build --release --features embed_resources
