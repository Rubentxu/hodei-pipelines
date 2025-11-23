# HWP (Hodei Worker Protocol) - Implementation Summary

## Overview

The HWP (Hodei Worker Protocol) is a gRPC-based protocol for communication between the Hodei Jobs orchestrator and worker agents. This document summarizes the implementation and usage.

## Status: ✅ COMPLETED

### What Was Implemented

1. **Protobuf Definitions** (`crates/hwp-proto/protos/hwp.proto`)
2. **gRPC Code Generation** using tonic-build
3. **Client and Server Stubs** for WorkerService

## Protocol Definition

### Messages

```protobuf
message WorkerRegistration {
  string worker_id = 1;
  repeated string capabilities = 2;
}

message WorkerStatus {
  string worker_id = 1;
  string state = 2;
}

message AssignJobRequest {
  string job_id = 1;
  string name = 2;
  string image = 3;
  repeated string command = 4;
}

message JobAccepted {
  string job_id = 1;
}

message JobResult {
  string job_id = 1;
  int32 exit_code = 2;
}

message LogEntry {
  string job_id = 1;
  string data = 2;
}

message CancelJobRequest {
  string job_id = 1;
}

message Empty {
  // Empty message for gRPC responses
}
```

### Service Methods

```protobuf
service WorkerService {
  rpc RegisterWorker(WorkerRegistration) returns (WorkerStatus);
  rpc AssignJob(AssignJobRequest) returns (JobAccepted);
  rpc StreamLogs(stream LogEntry) returns (Empty);
  rpc CancelJob(CancelJobRequest) returns (Empty);
}
```

## Implementation Details

### Dependencies

**Dependencies:**
- `tonic = "0.10"` - gRPC framework
- `prost = "0.12"` - Protocol Buffers serialization
- `prost-types = "0.12"` - Protobuf well-known types

**Build Dependencies:**
- `tonic-build = "0.10"` - Code generator
- `prost-build = "0.12"` - Protobuf compiler integration

### Build Process

The protocol is compiled during the build process:
1. `tonic-build` processes `protos/hwp.proto`
2. Generates Rust code in `target/debug/build/hwp-proto-*/out/`
3. The `tonic::include_proto!` macro includes the generated code

### Generated API

#### Client (WorkerServiceClient)

```rust
pub struct WorkerServiceClient<T> {
    inner: tonic::client::Grpc<T>,
}

impl WorkerServiceClient<tonic::transport::Channel> {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<StdError>,
}

impl<T> WorkerServiceClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
{
    pub async fn register_worker(
        &mut self,
        request: impl tonic::Request<WorkerRegistration>,
    ) -> Result<tonic::Response<WorkerStatus>, tonic::Status>;

    pub async fn assign_job(
        &mut self,
        request: impl tonic::Request<AssignJobRequest>,
    ) -> Result<tonic::Response<JobAccepted>, tonic::Status>;

    pub async fn stream_logs(
        &mut self,
        request: impl tonic::IntoStreamingRequest<Message = LogEntry>,
    ) -> Result<tonic::Response<Empty>, tonic::Status>;

    pub async fn cancel_job(
        &mut self,
        request: impl tonic::Request<CancelJobRequest>,
    ) -> Result<tonic::Response<Empty>, tonic::Status>;
}
```

#### Server (WorkerService trait)

```rust
#[async_trait]
pub trait WorkerService: Send + Sync + 'static {
    async fn register_worker(
        &self,
        request: tonic::Request<WorkerRegistration>,
    ) -> std::result::Result<tonic::Response<WorkerStatus>, tonic::Status>;

    async fn assign_job(
        &self,
        request: tonic::Request<AssignJobRequest>,
    ) -> std::result::Result<tonic::Response<JobAccepted>, tonic::Status>;

    async fn stream_logs(
        &self,
        request: tonic::Request<tonic::Streaming<LogEntry>>,
    ) -> std::result::Result<tonic::Response<Empty>, tonic::Status>;

    async fn cancel_job(
        &self,
        request: tonic::Request<CancelJobRequest>,
    ) -> std::result::Result<tonic::Response<Empty>, tonic::Status>;
}

pub struct WorkerServiceServer<T: WorkerService> {
    inner: _Inner<T>,
    // ... configuration options
}
```

## Usage Examples

### Server Implementation

```rust
use hwp_proto::pb::worker_service_server::{WorkerService, WorkerServiceServer};
use tonic::{Request, Response, Status};

pub struct MyWorkerService {
    // Your state here
}

#[tonic::async_trait]
impl WorkerService for MyWorkerService {
    async fn register_worker(
        &self,
        request: Request<WorkerRegistration>,
    ) -> Result<Response<WorkerStatus>, Status> {
        // Handle worker registration
        let worker_id = request.get_ref().worker_id.clone();
        Ok(Response::new(WorkerStatus {
            worker_id,
            state: "ready".to_string(),
        }))
    }

    async fn assign_job(
        &self,
        request: Request<AssignJobRequest>,
    ) -> Result<Response<JobAccepted>, Status> {
        // Handle job assignment
        let job_id = request.get_ref().job_id.clone();
        Ok(Response::new(JobAccepted { job_id }))
    }

    async fn stream_logs(
        &self,
        request: Request<tonic::Streaming<LogEntry>>,
    ) -> Result<Response<Empty>, Status> {
        // Handle log streaming
        let mut stream = request.into_inner();
        while let Some(log_entry) = stream.next().await {
            // Process log entries
        }
        Ok(Response::new(Empty {}))
    }

    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<Empty>, Status> {
        // Handle job cancellation
        Ok(Response::new(Empty {}))
    }
}

// Start the server
use tonic::transport::Server;

let addr = "0.0.0.0:50051".parse().unwrap();
let worker_service = MyWorkerService::new();
let server = WorkerServiceServer::new(worker_service);

Server::builder()
    .add_service(server)
    .serve(addr)
    .await?;
```

### Client Implementation

```rust
use hwp_proto::pb::worker_service_client::WorkerServiceClient;

let mut client = WorkerServiceClient::connect("http://localhost:50051").await?;

// Register worker
let response = client.register_worker(WorkerRegistration {
    worker_id: "worker-1".to_string(),
    capabilities: vec!["docker".to_string()],
}).await?;

let status = response.into_inner();
println!("Worker status: {}", status.state);

// Assign job
let response = client.assign_job(AssignJobRequest {
    job_id: "job-123".to_string(),
    name: "build-app".to_string(),
    image: "rust:1.70".to_string(),
    command: vec!["cargo".to_string(), "build".to_string()],
}).await?;

let accepted = response.into_inner();
println!("Job accepted: {}", accepted.job_id);

// Stream logs (client streaming)
let mut client_stream = client.stream_logs(tonic::Request::new(
    tokio_stream::wrappers::ReceiverStream::new(rx)
)).await?;

let log_entry = LogEntry {
    job_id: "job-123".to_string(),
    data: "Building application...".to_string(),
};

client_stream.send(log_entry).await?;
```

## Build Status

✅ **Protobuf compilation**: Fixed name collision (StreamLogs message vs StreamLogs method)
✅ **Version compatibility**: Aligned tonic-build 0.10 with tonic 0.10 and prost 0.12
✅ **Code generation**: Both client and server stubs generated successfully
✅ **Workspace integration**: hwp-proto crate integrated into workspace
✅ **Tests**: All tests pass

## Files

- **Proto Definition**: `crates/hwp-proto/protos/hwp.proto`
- **Build Script**: `crates/hwp-proto/build.rs`
- **Library**: `crates/hwp-proto/src/lib.rs`
- **Cargo Config**: `crates/hwp-proto/Cargo.toml`

## Next Steps

1. **Server Integration**: Implement the WorkerService in the orchestrator
2. **Client Integration**: Create worker agent clients that use this protocol
3. **Testing**: Add comprehensive integration tests
4. **Documentation**: Add examples to docs/

## Troubleshooting

### Common Issues

1. **"StreamLogs is not a message type"**
   - Fixed by renaming the message to `LogEntry` to avoid naming conflict

2. **Prost version conflicts**
   - Fixed by aligning all prost/tonic versions in the crate

3. **Missing gRPC service code**
   - Fixed by ensuring tonic-build generates server code with `.build_server(true)`

## References

- [tonic documentation](https://docs.rs/tonic/)
- [prost documentation](https://docs.rs/prost/)
- [gRPC Protocol Buffers](https://grpc.io/docs/what-is-grpc/introduction/)
