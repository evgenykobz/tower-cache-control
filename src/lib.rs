#![doc = include_str!("../README.md")]

use std::{mem, task, time::Duration};

use futures::future::BoxFuture;
pub use headers::CacheControl;
use headers::HeaderMapExt;
use http::{Request, Response, StatusCode};
use tower_layer::Layer;
use tower_service::Service;

/// Middleware [Layer] for the [CacheControlService] service.
#[derive(Clone, Debug)]
pub struct CacheControlLayer {
    default: Option<CacheControl>,
}
impl CacheControlLayer {
    pub fn new(header: CacheControl) -> Self {
        Self {
            default: Some(header),
        }
    }
}
impl Default for CacheControlLayer {
    fn default() -> Self {
        Self { default: None }
    }
}
impl<S> Layer<S> for CacheControlLayer
where
    S: Clone + Send + Sync + 'static,
{
    type Service = CacheControlService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CacheControlService {
            inner,
            default: self.default.clone(),
        }
    }
}
/// # `Cache-Control` setter [Service].
///
/// Assigns a value based on a response status:
/// * on `1xx` and `2xx` takes a `no-cache` request header directive or falls back to a default one;
/// * on `301`, likely a permanent move, sets a day *TTL* and asks *CDN* to cache the response, too;
/// * on any other `3xx` takes the default and prevents *CDN* from caching the response;
/// * on `4xx` caching is disabled;
/// * on `5xx` 30 min *TTL* is set.
///
/// *TTL* defaults to `5` seconds.
#[derive(Clone, Debug)]
pub struct CacheControlService<S>
where
    S: Clone + Send + Sync + 'static,
{
    inner: S,
    default: Option<CacheControl>,
}
impl<ReqB, ResB, S> Service<Request<ReqB>> for CacheControlService<S>
where
    S: Service<Request<ReqB>, Response = Response<ResB>> + Clone + Send + Sync + 'static,
    S::Future: Send + 'static,
    ReqB: Send + 'static,
    ResB: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, req: Request<ReqB>) -> Self::Future {
        let default = self
            .default
            .clone()
            .unwrap_or(CacheControl::new().with_max_age(Duration::from_secs(5)));
        let clone = self.inner.clone();
        let mut inner = mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let header = req
                .headers()
                .typed_get::<CacheControl>()
                .and_then(|header| header.ne(&CacheControl::new()).then_some(header));
            let mut response = inner.call(req).await?;
            let None = response.headers().typed_get::<CacheControl>() else {
                return Ok(response);
            };
            let header = match response.status() {
                StatusCode::MOVED_PERMANENTLY => default
                    .with_max_age(Duration::from_secs(86_400))
                    .with_public(),
                s if s.is_success() => header.unwrap_or(default),
                s if s.is_redirection() => header.unwrap_or(default).with_private(),
                s if s.is_client_error() => default.with_no_cache().with_private(),
                _ => default
                    .with_max_age(Duration::from_secs(1_800))
                    .with_public(),
            };
            response.headers_mut().typed_insert(header);
            Ok(response)
        })
    }
}
