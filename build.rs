fn main() {
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile(&["proto/orderbook.proto"], &["orderbook"])
        .expect("Failed to build protobuf wrapper");
}
