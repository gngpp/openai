use actix_web::{dev::ServiceRequest, http::header::HeaderMap};

pub struct Sign;

impl Sign {
    pub fn handle_request(req: &ServiceRequest, secret_key: &str) -> Result<(), String> {
        let headers = req.headers();
        // Extract the signature in the request
        let signature = headers
            .get("X-Authorization")
            .ok_or("header X-Authorization cannot been empty")?
            .to_str()
            .map_err(|op| op.to_string())?;

        let timestamp = headers
            .get("X-Time")
            .ok_or("header X-Time cannot been empty")?
            .to_str()
            .map_err(|op| op.to_string())?
            .parse::<u64>()
            .map_err(|op| op.to_string())?;

        let since_the_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards");

        // Verify the time difference
        if since_the_epoch.as_secs() < timestamp || (since_the_epoch.as_secs() - timestamp) > 60 {
            return Err("The request is no longer valid".to_string());
        }

        // Verify the signature
        let valid_signature = Self::validate_signature(
            req.method().as_str(),
            &req.uri().to_string(),
            headers,
            secret_key,
            timestamp,
            signature,
        )?;

        if valid_signature {
            Ok(())
        } else {
            // The signature verification failed and an error message was returned
            Err("Invalid signature.".to_string())
        }
    }

    fn validate_signature(
        method: &str,
        url: &str,
        headers: &HeaderMap,
        secret_key: &str,
        timestamp: u64,
        signature: &str,
    ) -> Result<bool, String> {
        // Create an ordered key-value pair map for storing HTTP headers
        let mut sorted_headers = std::collections::BTreeMap::new();
        headers
            .iter()
            .filter(|(k, _)| !k.as_str().eq("X-Authorization"))
            .for_each(|(k, v)| {
                println!("{}", k);
                sorted_headers.insert(k.as_str(), String::from_utf8_lossy(v.as_bytes()));
            });
        // Build the canonical request string
        let canonical_request = format!(
            "{method}\n{url}\n{timestamp}\n{}",
            Self::join_headers(&sorted_headers)
        );

        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        // Create alias for HMAC-SHA256
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(canonical_request.as_bytes());
        let result = mac.finalize();

        // Converts the message digest to a hexadecimal string
        let generated_signature = format!("{:02X}", result.into_bytes());

        // Verify that the signatures are consistent
        Ok(signature == generated_signature)
    }

    fn join_headers(
        headers: &std::collections::BTreeMap<&str, std::borrow::Cow<'_, str>>,
    ) -> String {
        headers
            .iter()
            .map(|(key, value)| format!("{}:{}", key, value))
            .collect::<Vec<String>>()
            .join("\n")
    }
}
