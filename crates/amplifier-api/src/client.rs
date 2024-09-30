pub mod requests;
use requests::AmplifierApiRequest;
use reqwest::header;
use tracing::{info_span, Instrument};

use crate::error::AmplifierApiError;

/// Client for the Amplifier API
#[expect(
    clippy::module_name_repetitions,
    reason = "makes the type easir to understand to the user of this crate"
)]
#[derive(Clone, Debug)]
pub struct AmplifierApiClient {
    inner: reqwest::Client,
    url: url::Url,
}

impl AmplifierApiClient {
    /// Create a new `AmplifierApiClient`.
    ///
    /// It requires a `.pem` encoded certificate to be attached to the client. The certificate is
    /// issued by Axelar.
    ///
    /// # Errors
    ///
    /// This function will return an error if the underlying reqwest client cannot be constructed
    pub fn new(url: url::Url, identity: &identity::Identity) -> Result<Self, AmplifierApiError> {
        let authenticated_client = authenticated_client(identity)?;
        Ok(Self {
            url,
            inner: authenticated_client,
        })
    }

    /// Send a request to Axelars Amplifier API
    #[tracing::instrument(skip_all, fields(req = ?request))]
    pub async fn send_request<'a, T>(
        &self,
        request: &T,
    ) -> Result<Result<T::Res, T::Error>, AmplifierApiError>
    where
        T: AmplifierApiRequest<'a> + core::fmt::Debug,
    {
        let endpoint = request.path(&self.url)?;
        let method = T::METHOD;

        async {
            let payload = simd_json::to_vec(&request.payload())?;
            let response = self
                .inner
                .request(T::METHOD, endpoint.as_str())
                .body(payload)
                .send()
                .await?;
            let status = response.status();
            let mut bytes = response
                .bytes()
                .await?
                .try_into_mut()
                .expect("not holding unique data on the buffer");
            if status.is_success() {
                {
                    let json = String::from_utf8_lossy(bytes.as_ref());
                    tracing::info!(response_body = ?json, "Response JSON");
                };
                let result = simd_json::from_slice::<T::Res>(bytes.as_mut())?;
                Ok(Ok(result))
            } else {
                {
                    let json = String::from_utf8_lossy(bytes.as_ref());
                    tracing::error!(
                        status = %status,
                        body = %json,
                        "Failed to execute request"
                    );
                };

                let error = simd_json::from_slice::<T::Error>(bytes.as_mut())?;
                Ok(Err(error))
            }
        }
        .instrument(info_span!("amplifir api request", ?method, ?endpoint).or_current())
        .await
    }
}

fn authenticated_client(
    identity: &identity::Identity,
) -> Result<reqwest::Client, AmplifierApiError> {
    const KEEP_ALIVE_INTERVAL: core::time::Duration = core::time::Duration::from_secs(15);
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        "Accept-Encoding",
        header::HeaderValue::from_static("gzip, deflate"),
    );
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );

    let temp_client = reqwest::Client::builder()
        .use_rustls_tls()
        .identity(identity.0.expose_secret().clone())
        .http2_keep_alive_interval(KEEP_ALIVE_INTERVAL)
        .http2_keep_alive_while_idle(true)
        .default_headers(headers)
        .build()?;
    Ok(temp_client)
}

/// helpers for deserializing `.pem` encoded certificates
pub mod identity {
    /// Represents a `.pem` encoded certificate
    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct Identity(
        #[serde(deserialize_with = "serde_utils::deserialize_identity")]
        pub  redact::Secret<reqwest::Identity>,
    );

    impl Identity {
        /// Creates a new [`Identity`].
        #[must_use]
        pub const fn new(identity: reqwest::Identity) -> Self {
            Self(redact::Secret::new(identity))
        }
    }

    mod serde_utils {
        use serde::{Deserialize, Deserializer};

        pub(crate) fn deserialize_identity<'de, D>(
            deserializer: D,
        ) -> Result<redact::Secret<reqwest::Identity>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let raw_string = String::deserialize(deserializer)?;
            let identity = reqwest::Identity::from_pem(raw_string.as_bytes())
                .inspect_err(|err| {
                    tracing::error!(?err, "cannot parse identity");
                })
                .map_err(serde::de::Error::custom)?;
            Ok(redact::Secret::new(identity))
        }
    }

    #[cfg(test)]
    mod tests {
        use serde::Deserialize;
        use simd_json;

        use super::*;

        #[test]
        fn test_deserialize_identity() {
            #[derive(Debug, Deserialize)]
            struct DesiredOutput {
                #[expect(dead_code, reason = "we don't care about reading the data in the test")]
                identity: Identity,
            }

            let identity_str = "
-----BEGIN CERTIFICATE-----
MIIC3zCCAcegAwIBAgIJALAul9kzR0W/MA0GCSqGSIb3DQEBBQUAMA0xCzAJBgNV
BAYTAmx2MB4XDTIyMDgwMjE5MTE1NloXDTIzMDgwMjE5MTE1NlowDTELMAkGA1UE
BhMCbHYwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQC8WWPaghYJcXQp
W/GAoFqKrQIwxy+h8vdZiURVzzqDKt/Mz45x0Zqj8RVSe4S0lLfkRxcgrLz7ZYSc
TKsVcur8P66F8A2AJaC4KDiYj4azkTtYQDs+RDLRJUCz5xf/Nw7m+6Y0K7p/p2m8
bPSm6osefz0orQqpwGogqOwI0FKMkU+BpYjMb+k29xbOec6aHxlaPlHLBPa+n3WC
V96KwmzSMPEN6Fn/G6PZ5PtwmNg769PiXKk02p+hbnx5OCKvi94mn8vVBGgXF6JR
Vq9IQQvfFm6G6tf7q+yxMdR2FBR2s03t1daJ3RLGdHzXWTAaNRS7E93OWx+ZyTkd
kIVM16HTAgMBAAGjQjBAMAkGA1UdEwQCMAAwEQYJYIZIAYb4QgEBBAQDAgeAMAsG
A1UdDwQEAwIFoDATBgNVHSUEDDAKBggrBgEFBQcDAjANBgkqhkiG9w0BAQUFAAOC
AQEAU/uQHjntyIVR4uQCRoSO5VKyQcXXFY5pbx4ny1yrn0Uxb9P6bOxY5ojcs0r6
z8ApT3sUfww7kzhle/G5DRtP0cELq7N2YP+qsFx8UO1GYZ5SLj6xm81vk3c0+hrO
Q3yoS60xKd/7nVsPZ3ch6+9ND0vVUOkefy0aeNix9YgbYjS11rTj7FNiHD25zOJd
VpZtHkvYDpHcnwUCd0UAuu9ntKKMFGwc9GMqzfY5De6nITvlqzH8YM4AjKO26JsU
7uMSyHtGF0vvyzhkwCqcuy7r9lQr9m1jTsJ5pSaVasIOJe+/JBUEJm5E4ppdslnW
1PkfLWOJw34VKkwibWLlwAwTDQ==
-----END CERTIFICATE-----
-----BEGIN PRIVATE KEY-----
MIIEpAIBAAKCAQEAvFlj2oIWCXF0KVvxgKBaiq0CMMcvofL3WYlEVc86gyrfzM+O
cdGao/EVUnuEtJS35EcXIKy8+2WEnEyrFXLq/D+uhfANgCWguCg4mI+Gs5E7WEA7
PkQy0SVAs+cX/zcO5vumNCu6f6dpvGz0puqLHn89KK0KqcBqIKjsCNBSjJFPgaWI
zG/pNvcWznnOmh8ZWj5RywT2vp91glfeisJs0jDxDehZ/xuj2eT7cJjYO+vT4lyp
NNqfoW58eTgir4veJp/L1QRoFxeiUVavSEEL3xZuhurX+6vssTHUdhQUdrNN7dXW
id0SxnR811kwGjUUuxPdzlsfmck5HZCFTNeh0wIDAQABAoIBAQCNJFNukCMhanKI
98xu/js7RlCo6urn6mGvJ+0cfJE1b/CL01HEOzUt+2BmEgetJvDy0M8k/i0UGswY
MF/YT+iFpNcMqYoEaK4aspFOyedAMuoMxP1gOMz363mkFt3ls4WoVBYFbGtyc6sJ
t4BSgNpFvUXAcIPYF0ewN8XBCRODH6v7Z6CrbvtjlUXMuU02r5vzMh8a4znIJmZY
40x6oNIss3YDCGe8J6qMWHByMDZbO63gBoBYayTozzCzl1TG0RZ1oTTL4z36wRto
uAhjoRek2kiO5axIgKPR/tYlyKzwLkS5v1W09K+pvsabAU6gQlC8kUPk7/+GOaeI
wGMI9FAZAoGBAOJN8mqJ3zHKvkyFW0uFMU14dl8SVrCZF1VztIooVgnM6bSqNZ3Y
nKE7wk1DuFjqKAi/mgXTr1v8mQtr40t5dBEMdgDpfRf/RrMfQyhEgQ/m1WqBQtPx
Suz+EYMpcH05ynrfSbxCDNYM4OHNJ1QfIvHJ/Q9wt5hT7w+MOH5h5TctAoGBANUQ
cXF4QKU6P+dLUYNjrYP5Wjg4194i0fh/I9NVoUE9Xl22J8l0lybV2phkuODMp1I+
rBi9AON9skjdCnwtH2ZbRCP6a8Zjv7NMLy4b4dQqfoHwTdCJ0FBfgZXhH4i+AXMb
XsKotxKGqCWgFKY8LB3UJ0qakK6h9Ze+/zbnZ9z/AoGBAJwrQkD3SAkqakyQMsJY
9f8KRFWzaBOSciHMKSi2UTmOKTE9zKZTFzPE838yXoMtg9cVsgqXXIpUNKFHIKGy
/L/PI5fZiTQIPBfcWRHuxEne+CP5c86i0xvc8OTcsf4Y5XwJnu7FfeoxFPd+Bcft
fMXyqCoBlREPywelsk606+M5AoGAfXLICJJQJbitRYbQQLcgw/K+DxpQ54bC8DgT
pOvnHR2AAVcuB+xwzrndkhrDzABTiBZEh/BIpKkunr4e3UxID6Eu9qwMZuv2RCBY
KyLZjW1TvTf66Q0rrRb+mnvJcF7HRbnYym5CFFNaj4S4g8QsCYgPdlqZU2kizCz1
4aLQQYsCgYAGKytrtHi2BM4Cnnq8Lwd8wT8/1AASIwg2Va1Gcfp00lamuy14O7uz
yvdFIFrv4ZPdRkf174B1G+FDkH8o3NZ1cf+OuVIKC+jONciIJsYLPTHR0pgWqE4q
FAbbOyAg51Xklqm2Q954WWFmu3lluHCWUGB9eSHshIurTmDd+8o15A==
-----END PRIVATE KEY-----
";

            let mut data = simd_json::to_string(&simd_json::json!({
                "identity": identity_str
            }))
            .unwrap()
            .into_bytes();

            let _output: DesiredOutput =
                simd_json::from_slice(data.as_mut()).expect("Failed to deserialize identity");
        }
    }
}
