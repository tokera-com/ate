cargo build --example passthru --target wasm32-wasi
cp -f ../target/wasm32-wasi/debug/examples/passthru.wasm ../tokweb/public/bin/example.wasm
