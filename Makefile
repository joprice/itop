release: build-release build-musl

build-release:
	cross build --release

build-musl: cargo-build-musl strip-musl-binary

cargo-build-musl:
	cross build --release --target x86_64-unknown-linux-musl

strip-musl-binary:
	docker run --rm -it -v`pwd`:/data rustembedded/cross:x86_64-unknown-linux-musl-0.1.16 strip /data/target/x86_64-unknown-linux-musl/release/itop
