//! Bindings for the [Basic device information](https://www.axis.com/vapix-library/subjects/t10175981/section/t10132180/display) API.
// TODO: Consider creating enum with error codes.
// TODO: Implement `getSupportedVersions`.
// TODO: Proper documentation.
// TODO: Consider adding support for checking if the API should be present.
use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{ajr_http2, ajr_http2::AjrHttpError, HttpClient};

const PATH: &str = "axis-cgi/basicdeviceinfo.cgi";

const API_VERSION: &str = "1.0";

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAllPropertiesData {
    pub property_list: PropertyList,
}
#[derive(Debug)]
pub struct GetAllPropertiesRequest<'a> {
    client: &'a HttpClient,
}

impl GetAllPropertiesRequest<'_> {
    pub async fn send(self) -> Result<GetAllPropertiesData, AjrHttpError> {
        ajr_http2::execute_request(
            PATH,
            json!({
                "apiVersion": API_VERSION,
                "method": "getAllProperties",
            }),
            self.client,
        )
        .await
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAllUnrestrictedPropertiesData {
    pub property_list: UnrestrictedPropertyList,
}
#[derive(Debug)]
pub struct GetAllUnrestrictedPropertiesRequest<'a> {
    client: &'a HttpClient,
}

impl GetAllUnrestrictedPropertiesRequest<'_> {
    pub async fn send(self) -> Result<GetAllUnrestrictedPropertiesData, AjrHttpError> {
        ajr_http2::execute_request(
            PATH,
            json!({
                "apiVersion": API_VERSION,
                "method": "getAllUnrestrictedProperties",
            }),
            self.client,
        )
        .await
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPropertiesData {
    pub property_list: HashMap<String, String>,
}

#[derive(Debug)]
pub struct GetPropertiesRequest<'a> {
    client: &'a HttpClient,
    property_list: Vec<String>,
}

// TODO: Consider helping users discover properties by using an enum or methods.
impl GetPropertiesRequest<'_> {
    pub async fn send(self) -> Result<GetPropertiesData, AjrHttpError> {
        ajr_http2::execute_request(
            PATH,
            json!({
                "apiVersion": API_VERSION,
                "method": "getProperties",
                "params": {"propertyList":self.property_list}
            }),
            self.client,
        )
        .await
    }
}

// TODO: Consider exposing a flat struct
// Pros of flat:
// - user does not need to know which substructure to look in; I tried
//   `getAllUnrestrictedProperties` so this wasn't an issue but I did that because I didn't know
//   the name of `architecture` and I was too lazy to look it up in the docs.
#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PropertyList {
    #[serde(flatten)]
    pub restricted: RestrictedPropertyList,
    #[serde(flatten)]
    pub unrestricted: UnrestrictedPropertyList,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RestrictedPropertyList {
    pub architecture: String,
    pub soc: String,
    pub soc_serial_number: String,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UnrestrictedPropertyList {
    pub brand: String,
    pub build_date: String,
    #[serde(rename = "HardwareID")]
    pub hardware_id: String,
    pub prod_full_name: String,
    pub prod_nbr: String,
    pub prod_short_name: String,
    pub prod_type: String,
    pub prod_variant: String,
    pub serial_number: String,
    pub version: String,
    #[serde(rename = "WebURL")]
    pub web_url: String,
}

pub struct Client<'a>(&'a HttpClient);

impl<'a> Client<'a> {
    pub fn new(http_client: &'a HttpClient) -> Self {
        Self(http_client)
    }

    pub fn get_properties(&self, properties: &[impl Display]) -> GetPropertiesRequest {
        GetPropertiesRequest {
            client: self.0,
            property_list: properties.iter().map(ToString::to_string).collect(),
        }
    }

    /// Fetch all properties.
    ///
    /// Please see the VAPIX Library documentation for [getAllProperties](https://www.axis.com/vapix-library/subjects/t10175981/section/t10132180/display?section=t10132180-t10132250).
    pub fn get_all_properties(&self) -> GetAllPropertiesRequest {
        GetAllPropertiesRequest { client: self.0 }
    }

    /// Fetch the subset of properties that are available without authentication.
    ///
    /// Please see the VAPIX Library documentation for [getAllUnrestrictedProperties](https://www.axis.com/vapix-library/subjects/t10175981/section/t10132180/display?section=t10132180-t10160656).
    pub fn get_all_unrestricted_properties(&self) -> GetAllUnrestrictedPropertiesRequest {
        GetAllUnrestrictedPropertiesRequest { client: self.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ajr2::ResponseEnvelope;

    #[test]
    fn can_serialize_responses() {
        let texts = vec![include_str!(
            "basic_device_info/get_all_unrestricted_properties_10_12_initial_response.json"
        )];
        for text in texts {
            let resp: ResponseEnvelope<GetAllUnrestrictedPropertiesData> =
                serde_json::from_str(text).unwrap();
            println!("{}", serde_json::to_string(&resp).unwrap());
        }
    }
}
