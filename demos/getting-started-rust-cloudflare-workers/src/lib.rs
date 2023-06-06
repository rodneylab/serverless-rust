use serde::Deserialize;
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
struct Message {
    name: String,
    email: String,

    #[allow(dead_code)]
    message: String,
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    log_request(&req);
    let router = Router::new();

    router
        .get("/health_check", |_, _| Response::ok("OK"))
        .get("/api-version", |_, _| {
            let version = env!("CARGO_PKG_VERSION");
            Response::ok(format!("API version: {version}"))
        })
        .post_async("/message", |mut req, _| async move {
            let Message { name, email, .. } = match req.json().await {
                Ok(value) => value,
                Err(_) => return Response::error("Bad request", 400),
            };
            console_log!("New message from {name} ({email})");
            Response::ok(format!("Thanks {name}, we'll be in touch!"))
        })
        .run(req, env)
        .await
}
