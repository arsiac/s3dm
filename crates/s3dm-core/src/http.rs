//! HTTP 连接器与 TLS 校验控制
//!
//! 提供两套 smithy HTTP 客户端：
//! - 默认（`default_connector`）：使用系统/内置根证书做标准 TLS 校验；
//! - 跳过校验（`SkipVerifyConnector`）：基于 `reqwest` 关闭证书验证，
//!   仅在用户显式开启「跳过 TLS 校验」时用于自签名/内网 HTTPS 端点。
//!
//! `build_shared_http_client` 根据 `skip_tls_verify` 选择其一并返回
//! aws-sdk-s3 所需的 `SharedHttpClient`。

use aws_smithy_http_client::default_connector;
use aws_smithy_runtime_api::client::http::{
    HttpConnector, HttpConnectorFuture, SharedHttpConnector, http_client_fn,
};
use aws_smithy_runtime_api::client::orchestrator::{HttpRequest, HttpResponse};
use aws_smithy_runtime_api::client::result::ConnectorError;
use aws_smithy_types::body::SdkBody;
use http_body_util::BodyExt;

/// 使用 `reqwest`（关闭 TLS 证书校验）的 HTTP 连接器，实现 smithy 的 `HttpConnector`。
///
/// 仅在用户显式开启「跳过 TLS 校验」时使用，关闭证书验证存在安全风险。
#[derive(Clone, Debug)]
struct SkipVerifyConnector {
    client: reqwest::Client,
}

impl SkipVerifyConnector {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("failed to build skip-TLS-verify HTTP client");
        Self { client }
    }
}

impl HttpConnector for SkipVerifyConnector {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let request = match request.try_into_http1x() {
            Ok(req) => req,
            Err(e) => {
                return HttpConnectorFuture::ready(Err(ConnectorError::user(e.into())));
            }
        };
        let client = self.client.clone();
        HttpConnectorFuture::new(async move {
            // 将 smithy 的 http::Request<SdkBody> 转换为 reqwest 请求
            let (parts, body) = request.into_parts();
            let body_bytes = body.collect().await.map_err(io_boxed)?.to_bytes();
            let method = parts.method;
            let url = match reqwest::Url::parse(&parts.uri.to_string()) {
                Ok(u) => u,
                Err(e) => return Err(io_boxed(e)),
            };
            let mut reqwest_request = reqwest::Request::new(method, url);
            *reqwest_request.headers_mut() = parts.headers.clone();
            reqwest_request
                .body_mut()
                .replace(reqwest::Body::from(body_bytes));

            let response = client.execute(reqwest_request).await.map_err(io_boxed)?;

            let status = response.status();
            let resp_bytes = response.bytes().await.map_err(io_boxed)?;
            let sdk_body = SdkBody::from(resp_bytes.as_ref());
            let resp = http::Response::builder()
                .status(status)
                .body(sdk_body)
                .map_err(io_boxed)?;
            HttpResponse::try_from(resp).map_err(io_boxed)
        })
    }
}

/// 将任意错误包装为 `ConnectorError`。
fn io_boxed<E: std::fmt::Display>(e: E) -> ConnectorError {
    ConnectorError::other(Box::new(std::io::Error::other(e.to_string())), None)
}

/// 根据是否跳过 TLS 校验构建 smithy HTTP 客户端。
pub(crate) fn build_shared_http_client(
    skip_tls_verify: bool,
) -> aws_sdk_s3::config::SharedHttpClient {
    if skip_tls_verify {
        http_client_fn(move |_settings, _components| {
            SharedHttpConnector::new(SkipVerifyConnector::new())
        })
    } else {
        http_client_fn(move |settings, _components| {
            default_connector(settings, None).expect("failed to build default HTTP connector")
        })
    }
}
