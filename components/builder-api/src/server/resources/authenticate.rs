// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env;

use actix_web::http::{Method, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Path};

use oauth_client::error::Error as OAuthError;

use crate::protocol::originsrv;
use crate::server::error::{Error, Result};
use crate::server::framework::middleware::{session_create_oauth, session_create_short_circuit};
use crate::server::AppState;

pub struct Authenticate {}

impl Authenticate {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/authenticate/{code}", Method::GET, authenticate)
    }
}

//
// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
fn authenticate(req: HttpRequest<AppState>) -> HttpResponse {
    let code = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok
    debug!("authenticate called, code = {}", code);

    match do_authenticate(&req, &code) {
        Ok(session) => HttpResponse::Ok().json(session),
        Err(Error::OAuth(OAuthError::HttpResponse(_code, _response))) => {
            HttpResponse::new(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            warn!("Oauth client error, {:?}", e);
            e.into()
        }
    }
}

//
// Internal - these functions should return Result<..>
//
fn do_authenticate(req: &HttpRequest<AppState>, code: &str) -> Result<originsrv::Session> {
    if env::var_os("HAB_FUNC_TEST").is_some() {
        return session_create_short_circuit(req, code);
    }

    let oauth = &req.state().oauth;
    let (token, user) = oauth.authenticate(code)?;
    let session = session_create_oauth(req, &token, &user, &oauth.config.provider)?;

    let id_str = session.get_id().to_string();
    if let Err(e) = req.state().segment.identify(&id_str) {
        debug!("Error identifying a user in segment, {}", e);
    }

    Ok(session)
}
