build-HelloWorldFunction:
	cargo build --release
	cp ../my-target/release/bootstrap $(ARTIFACTS_DIR)
