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
	@if command -v pwsh >/dev/null 2>&1; then \
		pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/build-release.ps1; \
	elif command -v powershell >/dev/null 2>&1; then \
		powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-release.ps1; \
	else \
		echo "PowerShell is required for package-windows"; \
		exit 1; \
	fi

clean:
	cargo clean
	rm -rf dist
