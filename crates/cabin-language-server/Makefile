.PHONY: build install-vscode

build:
	cargo build --release

install-vscode: build
	cp ./target/release/cabin-language-server.exe ../../../vscode-cabin/bin/cabin-language-server-windows-x86_64.exe
	cd ../../../vscode-cabin && npm run install