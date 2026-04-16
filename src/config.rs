use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_storage_mode")]
    pub storage_mode: String,
    #[serde(default = "default_data_path")]
    pub data_path: String,

    // ── MotherDuck ──────────────────────────────────────────────────────────
    pub motherduck_token: Option<String>,

    // ── S3 / R2 / MinIO ─────────────────────────────────────────────────────
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_access_key_id: Option<String>,
    pub s3_secret_access_key: Option<String>,
}

fn default_port() -> u16 {
    8080
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_storage_mode() -> String {
    "local".to_string()
}

fn default_data_path() -> String {
    "/data/logs.duckdb".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            storage_mode: default_storage_mode(),
            data_path: default_data_path(),
            motherduck_token: None,
            s3_bucket: None,
            s3_region: None,
            s3_endpoint: None,
            s3_access_key_id: None,
            s3_secret_access_key: None,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env::<Self>()
    }
}
