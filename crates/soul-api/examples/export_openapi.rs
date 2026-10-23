fn main() {
    print!(
        "{}",
        soul_api::openapi::json().expect("OpenAPI DTO serialization")
    );
}
