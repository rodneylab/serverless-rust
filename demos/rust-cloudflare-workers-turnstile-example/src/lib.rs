use serde::Deserialize;
use std::collections::HashMap;
use worker::*;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
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
    log_request(&req);
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
