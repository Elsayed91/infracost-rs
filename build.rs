fn main() {
    // Register the Infracost GraphQL schema with cynic
    cynic_codegen::register_schema("infracost")
        .from_sdl_file("schemas/infracost.graphql")
        .expect("Failed to load Infracost GraphQL schema")
        .as_default()
        .expect("Failed to set Infracost schema as default");

    // Tell cargo to rerun if the schema changes
    println!("cargo:rerun-if-changed=schemas/infracost.graphql");
}
