use crate::{
    config::ConfigStore,
    login::{login, login_page},
    session::{get_session_cookie, SessionStore},
};
use hyper::{Body, Client, Method, Request, Response};

// Handles incoming requests
pub async fn proxy(
    req: Request<Body>,
    conf: ConfigStore,
    store: SessionStore,
) -> Result<Response<Body>, hyper::Error> {
    // TODO: make special route configurable '/proxrs' + route
    // Check for special routes
    match (req.method(), req.uri().path()) {
        // Login page
        (&Method::GET, "/proxrs/login") => return login_page(&conf).await,

        // Login request
        (&Method::POST, "/proxrs/login") => return login(req, conf, store).await,

        // TODO: Logout request /proxrs/logout
        // TODO: Session debug page /proxrs/session

        // Ignore all other requests
        _ => (),
    }

    // Check if the request has an session cookie
    let session_token = match get_session_cookie(&req, &conf).await {
        Some(session_token) => session_token,
        None => return login_page(&conf).await,
    };

    // Check if the session token is valid
    let mut token = match store.get_token(&session_token).await {
        Some(token) => match token.is_valid() {
            true => token,
            false => {
                store.remove(&session_token).await;
                return login_page(&conf).await;
            }
        },
        None => return login_page(&conf).await,
    };

    // Renew the session token
    token.renew(&conf).await;

    // Build the request to the proxied site
    let method = req.method().clone();
    let headers = req.headers().clone();
    let uri = format!("http://81.173.114.61:8237{}", req.uri());
    let mut new_req = Request::new(req.into_body());
    *new_req.uri_mut() = uri.parse().unwrap();
    *new_req.method_mut() = method;
    *new_req.headers_mut() = headers;

    // Send the request to the proxied site
    let res = Client::new().request(new_req).await?;

    // Send the response back to the client
    Ok(res)
}
