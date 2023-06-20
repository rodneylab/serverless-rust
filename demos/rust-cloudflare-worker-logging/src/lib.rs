mod utils;

use chrono::{DateTime, Utc};
use reqwest::header::CONTENT_TYPE;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::*;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct LogDataItem {
    pathname: String,
    request_details: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct LogData {
    dt: String,
    level: String,
    message: String,
    item: LogDataItem,
}

async fn log_request(req: &Request, env: &Env) {
    let pathname = req.path();
    let timestamp = Date::now().to_string();
    let request_details = format!(
        "{timestamp} - [{pathname}], located at: {:?}, within: {}",
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
    console_log!("{request_details}");

    let date_time: DateTime<Utc> = match DateTime::parse_from_rfc3339(&timestamp) {
        Ok(value) => value.into(),
        Err(_) => Utc::now(),
    };

    let log_data = LogData {
        dt: date_time.to_rfc3339(),
        level: String::from("info"),
        message: "worker request".to_string(),
        item: LogDataItem {
            pathname,
            request_details,
        },
    };

    let mut buffer = Vec::new();
    log_data
        .serialize(&mut Serializer::new(&mut buffer).with_struct_map())
        .expect("Unable to serialize log data");
    let logtail_source_token = env
        .secret("LOGTAIL_SOURCE_TOKEN")
        .expect("LOGTAIL_SOURCE_TOKEN must be defined")
        .to_string();

    let client = reqwest::Client::new();
    let _ = client
        .post("https://in.logs.betterstack.com")
        .header(CONTENT_TYPE, "application/msgpack")
        .bearer_auth(logtail_source_token)
        .body::<Vec<u8>>(buffer)
        .send()
        .await;
}

#[derive(Deserialize)]
struct TurnstileVerifyResponse {
    success: bool,
}

fn cors_response_headers(request_headers: &worker::Headers, cors_origin: &str) -> worker::Headers {
    let mut headers = worker::Headers::new();
    let origin = match request_headers.get("Origin").unwrap() {
        Some(value) => value,
        None => return headers,
    };
    headers
        .set("Access-Control-Allow-Headers", "Content-Type")
        .expect("Unable to set header");
    headers
        .set("Access-Control-Allow-Methods", "POST")
        .expect("Unable to set header");
    headers.set("Vary", "Origin").expect("Unable to set header");
    if cors_origin.split(',').any(|val| val == cors_origin) {
        headers
            .set("Access-Control-Allow-Origin", &origin)
            .expect("Unable to set header");
    }
    headers
        .set("Access-Control-Max-Age", "86400")
        .expect("Unable to set header");
    headers
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    utils::set_panic_hook();

    log_request(&req, &env).await;

    let router = Router::new();

    router
        .get("/health_check", |_, _| Response::ok("OK"))
        .options("/v0/captcha", |req, ctx| {
            let headers =
                cors_response_headers(req.headers(), &ctx.var("CORS_ORIGIN")?.to_string());

            Ok(Response::empty()
                .unwrap()
                .with_headers(headers)
                .with_status(204))
        })
        .post_async("/v0/captcha", |mut req, ctx| async move {
            // Read in inputs
            let client_ip_option = req.headers().get("CF-Connecting-IP").unwrap();
            let turnstile_secret = ctx.secret("TURNSTILE_SECRETKEY")?.to_string();

            let headers =
                cors_response_headers(req.headers(), &ctx.var("CORS_ORIGIN")?.to_string());

            let turnstile_response = match req.form_data().await {
                Ok(value) => {
                    if let Some(FormEntry::Field(turnstile_response_value)) =
                        value.get("cf-turnstile-response")
                    {
                        turnstile_response_value
                    } else {
                        return Response::error("Bad request", 400);
                    }
                }
                Err(_) => {
                    return Response::error("Bad request", 400);
                }
            };

            // Prepare Turnstile verification HTTP request
            let client = reqwest::Client::new();

            let mut body_form_map = HashMap::<&str, String>::new();
            if let Some(value) = client_ip_option {
                body_form_map.insert("remoteip", value);
            };
            body_form_map.insert("response", turnstile_response);
            body_form_map.insert("secret", turnstile_secret);

            // Verify and process JSON response
            match client
                .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
                .form(&body_form_map)
                .send()
                .await
            {
                Ok(value) => match value.json::<TurnstileVerifyResponse>().await {
                    Ok(data) => {
                        let TurnstileVerifyResponse { success } = data;
                        console_log!("Turnstile verified: {success}");
                        Ok(Response::ok("OK")?.with_headers(headers))
                    }
                    Err(_) => Response::error("Bad request", 400),
                },
                Err(_) => Response::error("Bad gateway", 501),
            }
        })
        .run(req, env)
        .await
}
