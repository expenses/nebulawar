.PHONY: app

app: Fleet\ Commander.app

Fleet\ Commander.app: src resources Cargo.*
	cargo bundle --release --format osx
	cp -r target/release/bundle/osx/Fleet\ Commander.app .