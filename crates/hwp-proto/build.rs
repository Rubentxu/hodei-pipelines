fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf files using tonic-prost-build
    // This generates both message types AND gRPC service code for tonic 0.14.x
    tonic_prost_build::compile_protos("protos/hwp.proto")?;
    Ok(())
}
