use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    // subject ?? user_id
    pub sub: String,
    pub tenant_id: String,
    pub perms: Vec<String>,
    #[serde(default)]
    pub is_admin: bool,
    pub iat: usize,
    pub exp: usize,
    /// Login session. Absent only on tokens issued before sessions existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    /// Credential epoch captured when the session was opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cep: Option<i32>,
}

impl Claims {
    pub fn bearer(
        sub: impl Into<String>,
        tenant_id: impl Into<String>,
        perms: Vec<String>,
        is_admin: bool,
        iat: usize,
        exp: usize,
    ) -> Self {
        Self {
            sub: sub.into(),
            tenant_id: tenant_id.into(),
            perms,
            is_admin,
            iat,
            exp,
            sid: None,
            cep: None,
        }
    }
}

impl Default for Claims {
    fn default() -> Self {
        Self {
            sub: String::new(),
            tenant_id: String::new(),
            perms: Vec::new(),
            is_admin: false,
            iat: 0,
            exp: 0,
            sid: None,
            cep: None,
        }
    }
}

#[derive(Clone)]
pub struct JwtService {
    secret: String,
}

impl JwtService {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    pub fn encode(&self, claims: &Claims) -> anyhow::Result<String> {
        let key = jsonwebtoken::EncodingKey::from_secret(self.secret.as_bytes());
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
        Ok(jsonwebtoken::encode(&header, claims, &key)?)
    }

    pub fn decode(&self, token: &str) -> anyhow::Result<Claims> {
        self.decode_with(token, true)
    }

    /// Signature still binds the caller. Expiry is ignored only for session renewal.
    pub fn decode_ignoring_exp(&self, token: &str) -> anyhow::Result<Claims> {
        self.decode_with(token, false)
    }

    fn decode_with(&self, token: &str, validate_exp: bool) -> anyhow::Result<Claims> {
        let key = jsonwebtoken::DecodingKey::from_secret(self.secret.as_bytes());
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = validate_exp;
        let data = jsonwebtoken::decode::<Claims>(token, &key, &validation)?;
        Ok(data.claims)
    }
}
