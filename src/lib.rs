use serde::Deserialize;
use serde_json::json;
use worker::{wasm_bindgen::JsValue, *};

mod utils;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum RequestJson {
    #[serde(rename = "url_verification")]
    Verification(VerificationRequestJson),
    #[serde(rename = "event_callback")]
    Event(EventRequestJson),
}

#[derive(Deserialize)]
struct VerificationRequestJson {
    challenge: String,
}

#[derive(Deserialize)]
struct EventRequestJson {
    event: Event,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Event {
    #[serde(rename = "team_join")]
    Join(JoinEvent),
}

#[derive(Deserialize)]
struct JoinEvent {
    user: User,
}

#[derive(Deserialize)]
struct User {
    id: String,
    is_bot: bool,
    is_app_user: bool,
}

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    utils::set_panic_hook();

    let router = Router::new();

    router
        .post_async("/listen", |mut req, ctx| async move {
            let webhook_url = ctx.var("WEBHOOK_URL")?.to_string();
            let message = ctx.var("MESSAGE")?.to_string();

            let data = req.json().await?;
            match data {
                RequestJson::Verification(data) => {
                    let mut headers = Headers::new();
                    headers.set("Content-Type", "text/plain")?;

                    Ok(Response::from_bytes(data.challenge.into_bytes())?
                        .with_status(200)
                        .with_headers(headers))
                }
                RequestJson::Event(EventRequestJson {
                    event: Event::Join(JoinEvent { user }),
                }) => match (user.is_bot, user.is_app_user) {
                    (false, false) => Fetch::Request(Request::new_with_init(
                        &webhook_url,
                        RequestInit::new().with_method(Method::Post).with_body(Some(
                            JsValue::from_str(&serde_json::to_string(&json!({
                                "text": message.replace("@{id}", &user.id),
                            }))?),
                        )),
                    )?)
                    .send()
                    .await,
                    _ => Ok(Response::empty()?.with_status(200)),
                },
            }
        })
        .run(req, env)
        .await
}
