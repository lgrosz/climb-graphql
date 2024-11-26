use deadpool::managed;
use s3::{error::S3Error, Bucket, Region};

pub type Pool = managed::Pool<S3ClientManager>;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct S3Config {
    pub host: String,
    pub port: u32,
    pub region: String,
    pub bucket: String,
    pub style: S3ClientManagerStyle,
}

impl S3Config {
    pub fn create_pool(
        &self,
    ) -> Result<Pool, deadpool::managed::BuildError> {
        let manager = {
            let region = Region::Custom {
                region: self.region.to_string(),
                endpoint: format!("http://{}:{}", self.host, self.port).to_string(),
            };

            S3ClientManager {
                region,
                bucket: self.bucket.to_string(),
                style: self.style,
            }
        };

        managed::Pool::builder(manager)
            .max_size(10)
            .build()
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy)]
pub enum S3ClientManagerStyle {
    Subdomain,
    Path,
}

pub struct S3ClientManager {
    pub region: Region,
    pub bucket: String,
    pub style: S3ClientManagerStyle,
}

impl S3ClientManager {
    pub fn new(region: Region, bucket: String, style: S3ClientManagerStyle) -> Self {
        S3ClientManager {
            region,
            bucket,
            style,
        }
    }
}

impl managed::Manager for S3ClientManager {
    type Type = Bucket;
    type Error = S3Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let creds = s3::creds::Credentials::default()?;
        let bucket = Bucket::new(&self.bucket, self.region.clone(), creds)
            .map(|bucket| {
                match &self.style {
                    S3ClientManagerStyle::Subdomain => bucket,
                    S3ClientManagerStyle::Path => bucket.with_path_style(),
                }
            })?;
        Ok(*bucket)
    }

    async fn recycle(
        &self,
        _: &mut Self::Type,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<Self::Error> {
        Ok(())
    }
}
