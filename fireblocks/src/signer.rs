use anyhow::{Context, Result};
use hub_core::{anyhow, chrono::Utc, serde_json};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub(crate) struct RequestSigner {
    secret: EncodingKey,
    api_key: String,
}

impl RequestSigner {
    pub(crate) fn new(secret: EncodingKey, api_key: String) -> Self {
        Self { secret, api_key }
    }

    pub(crate) fn sign(&self, uri: String, body: impl Serialize) -> Result<String> {
        let header = Header::new(Algorithm::RS256);
        let payload = Payload::new(uri, self.api_key.clone(), body)?;

        Ok(encode(&header, &payload, &self.secret)?)
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Payload {
    uri: String,
    nonce: u64,
    iat: u64,
    exp: u64,
    sub: String,
    #[serde(rename = "bodyHash")]
    body_hash: String,
}

impl Payload {
    pub(crate) fn new(uri: String, sub: String, body: impl Serialize) -> Result<Self> {
        let time = Utc::now();
        let nonce = u64::try_from(time.timestamp_nanos()).context("time is running backwards")?;
        let iat = u64::try_from(time.timestamp()).context("time is running backwards")?;
        let exp = iat + 30;

        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_vec(&body)?);
        let hash = hasher.finalize().to_vec();

        let hex = hex::encode(hash);

        Ok(Self {
            uri,
            nonce,
            iat,
            exp,
            sub,
            body_hash: hex,
        })
    }
}
