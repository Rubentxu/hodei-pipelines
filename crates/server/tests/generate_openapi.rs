use hodei_server::api_docs::ApiDoc;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use utoipa::OpenApi;

#[test]
fn generate_openapi_spec() {
    let doc = ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&doc).expect("Failed to serialize to YAML");

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/server -> crates
    path.pop(); // crates -> root
    path.push("docs");
    path.push("openapi.yaml");

    let mut file = File::create(&path).expect("Failed to create openapi.yaml");
    file.write_all(yaml.as_bytes())
        .expect("Failed to write to openapi.yaml");

    println!("OpenAPI spec generated at {:?}", path);
}
