build-windows:
	cargo build --release --target=x86_64-pc-windows-msvc

build-linux:
	cargo zigbuild --release --target=x86_64-unknown-linux-gnu

build-mac:
	cargo build --release --target=x86_64-apple-darwin

build-all: build-windows build-linux
	rm builds -rf
	mkdir builds
	cp target/x86_64-pc-windows-msvc/release/cabin.exe builds/windows-x86_64-cabin.exe
	cp target/x86_64-unknown-linux-gnu/release/cabin builds/linux-x86_64-cabin