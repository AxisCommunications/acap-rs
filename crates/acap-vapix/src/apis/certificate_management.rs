//! Bindings for the [Certificate management API](https://www.axis.com/vapix-library/subjects/t10175981/section/t10161277/display).
// TODO: Implement remaining methods.
// TODO: Improve documentation.
// TODO: Return actionable error instead of `anyhow::Error`.
use anyhow::Context;

const PATH: &str = "acap-vapix/services";
const FOREWORD: &str = concat!(
    r#"<SOAP-ENV:Envelope"#,
    r#" xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/""#,
    r#" xmlns:xs="http://www.w3.org/2001/XMLSchema""#,
    r#" xmlns:tds="http://www.onvif.org/ver10/device/wsdl""#,
    r#" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#,
    r#" xmlns:xsd="http://www.w3.org/2001/XMLSchema""#,
    // Note that this is not set in the example for SetWebServerTlsConfiguration
    r#" xmlns:onvif="http://www.onvif.org/ver10/schema""#,
    r#" xmlns:tt="http://www.onvif.org/ver10/schema""#,
    r#" xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope">"#,
    r#"<SOAP-ENV:Body>"#
);
const AFTERWORD: &str = concat!(r#"</SOAP-ENV:Body>"#, r#"</SOAP-ENV:Envelope>""#);

async fn execute(body: String, client: &crate::http::Client) -> anyhow::Result<()> {
    let response = client
        .post(PATH)?
        .replace_with(|b| {
            b.header("Content-Type", "text/xml;charset=UTF-8")
                .body(format!("{FOREWORD}{body}{AFTERWORD}"))
        })
        .send()
        .await?;

    // TODO: Parse out the actual error message.
    let status = response.status();
    if !status.is_success() {
        let err = Err(anyhow::anyhow!(
            "Certificate management request failed with status code {status}"
        ));
        return match response.text().await {
            Ok(text) => err.with_context(|| text),
            Err(e) => err.with_context(|| e.to_string()),
        };
    }

    Ok(())
}

#[non_exhaustive]
pub struct DeleteCertificatesRequest<'a> {
    id: &'a str,
}

impl DeleteCertificatesRequest<'_> {
    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<()> {
        execute(
            format!(
                concat!(
                    r#"<tds:DeleteCertificates xmlns="http://www.onvif.org/ver10/device/wsdl">"#,
                    r#"<CertificateID>{certificate_id}</CertificateID>"#,
                    r#"</tds:DeleteCertificates>"#
                ),
                certificate_id = self.id
            ),
            client,
        )
        .await
    }
}

pub fn delete_certificates(certificate_id: &str) -> DeleteCertificatesRequest {
    DeleteCertificatesRequest { id: certificate_id }
}

#[non_exhaustive]
pub struct LoadCaCertificatesRequest<'a> {
    id: &'a str,
    certificate_b64_data: &'a str,
}

impl LoadCaCertificatesRequest<'_> {
    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<()> {
        execute(
            format!(
                concat!(
                    r#"<tds:LoadCACertificates xmlns="http://www.onvif.org/ver10/device/wsdl">"#,
                    r#"<CACertificate>"#,
                    r#"<tt:CertificateID>{certificate_id}</tt:CertificateID>"#,
                    r#"<tt:Certificate>"#,
                    r#"<tt:Data>{certificate_data}</tt:Data>"#,
                    r#"</tt:Certificate>"#,
                    r#"</CACertificate>"#,
                    r#"</tds:LoadCACertificates>"#,
                ),
                certificate_id = self.id,
                certificate_data = self.certificate_b64_data,
            ),
            client,
        )
        .await
    }
}

pub fn load_ca_certificates<'a>(
    certificate_id: &'a str,
    certificate: &'a str,
) -> LoadCaCertificatesRequest<'a> {
    LoadCaCertificatesRequest {
        id: certificate_id,
        certificate_b64_data: certificate,
    }
}

#[non_exhaustive]
pub struct LoadCertificateWithPrivateKeyRequest<'a> {
    id: &'a str,
    certificate_b64_data: &'a str,
    private_key_b64_data: &'a str,
}

impl LoadCertificateWithPrivateKeyRequest<'_> {
    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<()> {
        execute(
            format!(
                concat!(r#"<tds:LoadCertificateWithPrivateKey xmlns="http://www.onvif.org/ver10/device/wsdl">"#,
                r#"<CertificateWithPrivateKey>"#,
                r#"<tt:CertificateID>{certificate_id}</tt:CertificateID>"#,
                r#"<tt:Certificate>"#,
                r#"<tt:Data>{certificate_data}</tt:Data>"#,
                r#"</tt:Certificate>"#,
                r#"<tt:PrivateKey>"#,
                r#"<tt:Data>{private_key_data}</tt:Data>"#,
                r#"</tt:PrivateKey>"#,
                r#"</CertificateWithPrivateKey>"#,
                r#"</tds:LoadCertificateWithPrivateKey>"#,
                ),
                certificate_id=self.id, certificate_data=self.certificate_b64_data, private_key_data=self.private_key_b64_data
            ),
            client,
        )
            .await
    }
}

pub fn load_certificate_with_private_key<'a>(
    certificate_id: &'a str,
    certificate: &'a str,
    private_key: &'a str,
) -> LoadCertificateWithPrivateKeyRequest<'a> {
    LoadCertificateWithPrivateKeyRequest {
        id: certificate_id,
        certificate_b64_data: certificate,
        private_key_b64_data: private_key,
    }
}
