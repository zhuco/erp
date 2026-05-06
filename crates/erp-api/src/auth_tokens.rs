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
        let key = jsonwebtoken::DecodingKey::from_secret(self.secret.as_bytes());
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = true;
        let data = jsonwebtoken::decode::<Claims>(token, &key, &validation)?;
        Ok(data.claims)
    }
}
