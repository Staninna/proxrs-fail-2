use crate::{
    config::{ConfigKey::*, ConfigStore},
    error::internal_error,
    session::{Session, SessionStore, User},
};
use hyper::{header::SET_COOKIE, http::HeaderValue, Body, Request, Response, StatusCode};
use tera::Tera;

// Handles login requests
pub async fn login(
    req: Request<Body>,
    conf: ConfigStore,
    tera: Tera,
    store: SessionStore,
) -> Result<Response<Body>, hyper::Error> {
    // Extract the request body containing user credentials
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;

    // Deserialize the request body into a User struct
    let user: User = match serde_urlencoded::from_bytes(&body_bytes) {
        Ok(user) => user,
        Err(_) => return login_page(&conf, tera).await,
    };

    // Validate user credentials
    // TODO: Replace this with a database lookup
    if !user.username.is_empty() && !user.password.is_empty() {
        // Generate a session token
        let session = match Session::new(user.username, &conf, &store).await {
            Some(token) => {
                // Add the session to the sessions map
                store.add(token.clone()).await;
                token
            }
            None => return redirect(Response::new(Body::from(""))),
        };

        // Create the response
        let mut response = Response::new(Body::from(""));

        // Set an session cookie
        let cookie_name = conf.get(SessionCookieName).await;
        let cookie = format!(
            "{}={}; HttpOnly; Path=/; Expires={}",
            cookie_name,
            session.token,
            session.expires.format("%a, %d %b %Y %T GMT")
        );
        response
            .headers_mut()
            .insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());

        // Redirect to the home page
        return redirect(response);
    }
    // If the user is not valid, return the login page
    login_page(&conf, tera).await
}

pub async fn login_page(conf: &ConfigStore, tera: Tera) -> Result<Response<Body>, hyper::Error> {
    // Get the login page path from the config
    let login_page_path = conf.get(LoginPageTemplate).await;

    // Load the login page using the tera template engine
    let login_page = match tera.render(&login_page_path, &tera::Context::new()) {
        Ok(login_page) => login_page,
        Err(_) => return internal_error(&conf, tera).await,
    };

    // Create the response
    let mut response = Response::new(Body::from(login_page));

    // Set the content type
    response
        .headers_mut()
        .insert("Content-Type", "text/html".parse().unwrap());

    Ok(response)
}

fn redirect(mut response: Response<Body>) -> Result<Response<Body>, hyper::Error> {
    // Redirect / of the proxy to the home page
    *response.status_mut() = StatusCode::FOUND;
    response
        .headers_mut()
        .insert("Location", "/".parse().unwrap());

    Ok(response)
}