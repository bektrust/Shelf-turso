pub struct Db {
    pub inner: libsql::Database,
}

impl Db {
    pub async fn connect() -> Self {
        let _ = dotenvy::dotenv();

        let url = env_var("TURSO_URL");
        let token = env_var("TURSO_TOKEN");

        // let url = "libsql://sut-dbadmin.aws-eu-west-1.turso.io".to_string();
        // let token = "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJhIjoicnciLCJpYXQiOjE3OTE5NzIwMjYsImlkIjoiMDFhMTM5M2UtN2MwMS03NTM1LWJlNWMtODhkNTVjMjZjMGVjIiwia2lkIjoiUGZqRmtTVER2MXVHcVlzWXhzMHhRUjdwNGVlaHhrUnhQNjVRTUsyc2lJcyIsInJpZCI6IjFmNjcwYTBlLTFhMmYtNGZmMy1hY2U1LTBmMjU1ZGE2YjZkZiJ9.kx17LLK2nHgLRmUB5ULs18wZPuh53NYa0F-YcrzGITl_zKPyszFoX_6ujkCLGx7UElUamBE5yTIxzVI40IP2AQ".to_string();

        let db = libsql::Builder::new_remote(url, token)
            .build()
            .await
            .expect("db");

        Self { inner: db }
    }
}

/// The value of the environment variable `name`, or a panic that names it.
fn env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        panic!(
            "{name} is not set — put it in .env (see .env.example), or run `{name}=... topcoat dev`"
        )
    })
}
