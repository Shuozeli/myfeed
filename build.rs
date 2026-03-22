fn main() {
    prost_build::Config::new()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .compile_protos(&["proto/myfeed.proto"], &["proto/"])
        .expect("failed to compile proto files");
}
