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
use futures_util::future::LocalBoxFuture;

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

        let bad_response = |msg: String, request: ServiceRequest| -> Self::Future {
            let (req, _pl) = request.into_parts();
            let resp = HttpResponse::Unauthorized()
                .json(MiddlewareMessage { msg: &msg })
                // constructed responses map to "right" body
                .map_into_right_body();
            Box::pin(async { Ok(ServiceResponse::new(req, resp)) })
        };

        if let Some(token) = authorization {
            let token = token.clone();
            let svc = Rc::clone(&self.service);
            Box::pin(async move {
                match crate::token::verify_access_token_for_u8(token.as_bytes()).await {
                    Ok(_) => {
                        let res = svc.call(request);
                        // forwarded responses map to "left" body
                        res.await.map(ServiceResponse::map_into_left_body)
                    }
                    Err(err) => bad_response(err.to_string(), request).await,
                }
            })
        } else {
            bad_response("AccessToken is required!".to_string(), request)
        }
    }
}

#[cfg(feature = "limit")]
use super::tokenbucket::TokenBucket;

#[cfg(feature = "limit")]
pub struct TokenBucketRateLimiter {
    tb: Rc<TokenBucket>,
}

#[cfg(feature = "limit")]
impl TokenBucketRateLimiter {
    pub fn new(tb: TokenBucket) -> Self {
        Self { tb: Rc::new(tb) }
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
            tb: self.tb.clone(),
        }))
    }
}

#[cfg(feature = "limit")]
pub struct TokenBacketMiddleware<S> {
    service: Rc<S>,
    tb: Rc<TokenBucket>,
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
        let bad_response = |msg: String, request: ServiceRequest| -> Self::Future {
            let (req, _pl) = request.into_parts();
            let resp = HttpResponse::TooManyRequests()
                .json(MiddlewareMessage { msg: &msg })
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

        let svc = self.service.clone();
        let tb = self.tb.clone();
        Box::pin(async move {
            match addr {
                Ok(addr) => {
                    match tb.acquire(addr).await {
                        true => {
                            let res = svc.call(request);
                            // forwarded responses map to "left" body
                            res.await.map(ServiceResponse::map_into_left_body)
                        }
                        false => bad_response("Too Many Requests".to_string(), request).await,
                    }
                }
                Err(err) => bad_response(err.to_string(), request).await,
            }
        })
    }
}
