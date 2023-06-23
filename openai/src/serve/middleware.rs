use std::net::IpAddr;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    http::header,
    Error, HttpResponse,
};
use futures_core::future::LocalBoxFuture;

#[derive(serde::Serialize)]
struct MiddlewareMessage<'a> {
    msg: &'a str,
}

pub struct TokenAuthorization;

impl<S, B> Transform<S, ServiceRequest> for TokenAuthorization
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = TokenAuthorizationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TokenAuthorizationMiddleware {
            service: Rc::new(service),
        }))
    }
}
pub struct TokenAuthorizationMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for TokenAuthorizationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let authorization = request.headers().get(header::AUTHORIZATION);

        let bad_response = |msg: &str, request: ServiceRequest| -> Self::Future {
            let (req, _pl) = request.into_parts();
            let resp = HttpResponse::Unauthorized()
                .json(MiddlewareMessage { msg })
                // constructed responses map to "right" body
                .map_into_right_body();
            Box::pin(async { Ok(ServiceResponse::new(req, resp)) })
        };

        match authorization {
            Some(token) => {
                let token = token.clone();
                let svc = Rc::clone(&self.service);
                Box::pin(async move {
                    match token::verify_access_token_for_u8(token.as_bytes()).await {
                        Ok(_) => {
                            // forwarded responses map to "left" body
                            svc.call(request)
                                .await
                                .map(ServiceResponse::map_into_left_body)
                        }
                        Err(err) => bad_response(&err.to_string(), request).await,
                    }
                })
            }
            None => bad_response("access_token is required!", request),
        }
    }
}

use crate::token;

#[cfg(feature = "sign")]
use super::sign::Sign;
#[cfg(feature = "limit")]
use super::tokenbucket::{TokenBucket, TokenBucketContext};

#[cfg(feature = "limit")]
pub struct TokenBucketRateLimiter(Rc<TokenBucketContext>);

#[cfg(feature = "limit")]
impl TokenBucketRateLimiter {
    pub fn new(tb: TokenBucketContext) -> Self {
        Self(Rc::new(tb))
    }
}

#[cfg(feature = "limit")]
impl<S, B> Transform<S, ServiceRequest> for TokenBucketRateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = TokenBacketMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TokenBacketMiddleware {
            service: Rc::new(service),
            tb: self.0.clone(),
        }))
    }
}

#[cfg(feature = "limit")]
pub struct TokenBacketMiddleware<S> {
    service: Rc<S>,
    tb: Rc<TokenBucketContext>,
}

impl<S, B> Service<ServiceRequest> for TokenBacketMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let bad_response = |msg: &str, request: ServiceRequest| -> Self::Future {
            let (req, _pl) = request.into_parts();
            let resp = HttpResponse::TooManyRequests()
                .json(MiddlewareMessage { msg })
                // constructed responses map to "right" body
                .map_into_right_body();
            Box::pin(async { Ok(ServiceResponse::new(req, resp)) })
        };

        let conn_info = request.connection_info().clone();
        let addr = if let Some(addr) = conn_info.realip_remote_addr() {
            addr.parse::<IpAddr>()
        } else {
            conn_info.host().parse::<IpAddr>()
        };

        match addr {
            Ok(addr) => {
                let svc = self.service.clone();
                let tb = self.tb.clone();
                Box::pin(async move {
                    match tb.acquire(addr).await {
                        Ok(condition) => {
                            match condition {
                                true => {
                                    // forwarded responses map to "left" body
                                    svc.call(request)
                                        .await
                                        .map(ServiceResponse::map_into_left_body)
                                }
                                false => bad_response("Too Many Requests", request).await,
                            }
                        }
                        Err(err) => bad_response(&err.to_string(), request).await,
                    }
                })
            }
            Err(err) => bad_response(&err.to_string(), request),
        }
    }
}

pub struct ApiSign(Rc<Option<String>>);

impl ApiSign {
    pub fn new(s: Option<String>) -> Self {
        Self(Rc::new(s))
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiSign
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiSignMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiSignMiddleware {
            service: service,
            secret_key: self.0.clone(),
        }))
    }
}
pub struct ApiSignMiddleware<S> {
    service: S,
    secret_key: Rc<Option<String>>,
}

impl<S, B> Service<ServiceRequest> for ApiSignMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let ok_response = |request: ServiceRequest| -> Self::Future {
            let res = self.service.call(request);
            Box::pin(async move {
                // forwarded responses map to "left" body
                res.await.map(ServiceResponse::map_into_left_body)
            })
        };
        match self.secret_key.as_deref() {
            Some(secret_key) => {
                match Sign::handle_request(&request, secret_key) {
                    Ok(_) => ok_response(request),
                    Err(msg) => {
                        let (req, _) = request.into_parts();
                        let resp = HttpResponse::BadRequest()
                            .json(MiddlewareMessage { msg: &msg })
                            // constructed responses map to "right" body
                            .map_into_right_body();
                        Box::pin(async { Ok(ServiceResponse::new(req, resp)) })
                    }
                }
            }
            None => ok_response(request),
        }
    }
}
