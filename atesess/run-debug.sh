RUST_LOG=wasmer_compiler_cranelift=error,wasmer_wasi=error,thrussh=error,cranelift=error,regalloc=error,tokio_tungstenite=info,tungstenite=info,mio=info,trust_dns_proto=info,trace cargo run --bin atesess -- run --port 8888 --tls-port 4443 --native-files-path ../tokweb/public
