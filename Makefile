.PHONY: run check release package package-windows clean

run:
	cargo run

check:
	cargo fmt --check
	cargo check

release:
	cargo build --release

package:
	scripts/build-release.sh

package-windows:
	scripts/build-windows.sh

clean:
	cargo clean
	rm -rf dist
