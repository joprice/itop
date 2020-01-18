release: build-release build-musl

build-release:
	cross build --release

build-musl: cargo-build-musl

cargo-build-musl:
	RUSTFLAGS='-C link-arg=-s'  cross build --release --target x86_64-unknown-linux-musl

run-docker:
	 docker run -v $PWD:/data --rm -it ubuntu /data/target/x86_64-unknown-linux-musl/release/itop
