use hmac::{Mac, SimpleHmac};
use sha2::Sha256;

use nom::{
    bytes::complete::tag,
    character::complete::{alphanumeric1, char, hex_digit1},
    sequence::separated_pair,
    IResult,
};
use serde::{Deserialize, Serialize};

type HmacSha256 = SimpleHmac<Sha256>;

#[derive(Deserialize, Serialize)]
struct MuxPlaybackId {
    policy: String,
    id: String,
}

#[derive(Deserialize, Serialize)]
struct MuxData {
    status: String,
    playback_ids: Vec<MuxPlaybackId>,
    id: String,
    duration: Option<f32>,
    created_at: u32,
    aspect_ratio: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct MuxEvent {
    r#type: String,
    data: MuxData,
    id: String,
    created_at: String,
}

fn hmac_sha_256_verify(key: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let mut mac = HmacSha256::new_from_slice(key).expect("Error parsing HMAC_SHA256 key");
    mac.update(message);
    let result = mac.finalize().into_bytes();
    result.as_slice() == signature
}

pub struct MuxWebhookEvent {
    signing_secret: String,
}

impl MuxWebhookEvent {
    pub fn new(signing_secret: &str) -> MuxWebhookEvent {
        MuxWebhookEvent {
            signing_secret: signing_secret.into(),
        }
    }

    pub fn parse_mux_signature_header(mux_signature: &str) -> IResult<&str, (&str, &str)> {
        let mut parser = separated_pair(
            nom::sequence::preceded(tag("t="), alphanumeric1),
            char(','),
            nom::sequence::preceded(tag("v1="), hex_digit1),
        );
        parser(mux_signature)
    }

    pub fn verify_event(&self, mux_signature: &str, raw_request_body: &str) -> bool {
        let (timestamp, signature) =
            match MuxWebhookEvent::parse_mux_signature_header(mux_signature) {
                Ok((_, (val_timestamp, val_signature))) => (val_timestamp, val_signature),
                Err(_) => return false,
            };
        let payload = format!("{}.{}", timestamp, raw_request_body);
        hmac_sha_256_verify(
            self.signing_secret.as_bytes(),
            payload.as_bytes(),
            &hex::decode(signature).unwrap(),
        )
    }
}
