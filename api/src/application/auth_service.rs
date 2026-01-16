use crate::domain::entities::DomainError;
use crate::domain::ports::UserRepository;
use argon2::{
    Algorithm, Argon2, Params, PasswordHasher, Version,
    password_hash::{PasswordHash, PasswordVerifier},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct AuthService {
    user_repository: Arc<dyn UserRepository>,
    pepper: String,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        pepper: String,
        jwt_secret: String,
    ) -> Self {
        Self {
            user_repository,
            pepper,
            jwt_secret,
        }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<String, DomainError> {
        let user = self
            .user_repository
            .find_by_email(email)
            .await?
            .ok_or_else(|| DomainError::ValidationError("Invalid credentials".to_string()))?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| DomainError::Internal("Invalid password hash in DB".to_string()))?;

        let password_with_pepper = format!("{}{}", password, self.pepper);

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default());

        if argon2
            .verify_password(password_with_pepper.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Err(DomainError::ValidationError(
                "Invalid credentials".to_string(),
            ));
        }

        let expiration = Utc::now()
            .checked_add_signed(Duration::hours(24))
            .ok_or_else(|| DomainError::Internal("Time overflow".to_string()))?
            .timestamp();

        let claims = Claims {
            sub: user.email.clone(),
            exp: expiration as usize,
            iat: Utc::now().timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(token)
    }

    pub async fn get_current_user(&self, token: &str) -> Result<crate::domain::entities::User, DomainError> {
        use jsonwebtoken::{Validation, DecodingKey, decode};

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| DomainError::ValidationError(format!("Invalid token: {}", e)))?;

        let email = token_data.claims.sub;

        self.user_repository
            .find_by_email(&email)
            .await?
            .ok_or_else(|| DomainError::NotFound("User not found".to_string()))
    }

    pub async fn update_user(&self, token: &str, name: Option<String>, email: Option<String>) -> Result<crate::domain::entities::User, DomainError> {
        let mut user = self.get_current_user(token).await?;

        if let Some(n) = name {
            user.name = n;
        }
        if let Some(e) = email {
            user.email = e;
        }

        self.user_repository.update(&user).await
    }

    pub async fn change_password(&self, token: &str, current_password: &str, new_password: &str) -> Result<(), DomainError> {
        let mut user = self.get_current_user(token).await?;

        // Verify current password
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| DomainError::Internal("Invalid password hash in DB".to_string()))?;

        let password_with_pepper = format!("{}{}", current_password, self.pepper);
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default());

        if argon2
            .verify_password(password_with_pepper.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Err(DomainError::ValidationError("Invalid current password".to_string()));
        }

        // Hash new password
        let salt = argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let new_password_with_pepper = format!("{}{}", new_password, self.pepper);
        
        user.password_hash = argon2
            .hash_password(new_password_with_pepper.as_bytes(), &salt)
            .map_err(|e| DomainError::Internal(e.to_string()))?
            .to_string();

        self.user_repository.update(&user).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::User;
    use crate::domain::ports::MockUserRepository;
    use argon2::{Argon2, PasswordHasher};
    use mockall::predicate::*;

    fn hash_password(password: &str, pepper: &str) -> String {
        let argon2 = Argon2::default();
        let salt = argon2::password_hash::SaltString::generate(
            &mut argon2::password_hash::rand_core::OsRng,
        );
        let password_peppered = format!("{}{}", password, pepper);
        argon2
            .hash_password(password_peppered.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn test_login_success() {
        let mut repo = MockUserRepository::new();
        let pepper = "pepper".to_string();
        let jwt_secret = "secret".to_string();

        let password_hash = hash_password("password", &pepper);
        let user = User {
            email: "test@example.com".to_string(),
            password_hash,
            ..Default::default()
        };

        repo.expect_find_by_email()
            .with(eq("test@example.com"))
            .returning(move |_| Ok(Some(user.clone())));

        let service = AuthService::new(Arc::new(repo), pepper, jwt_secret);
        let token = service.login("test@example.com", "password").await;

        assert!(token.is_ok());
    }

    #[tokio::test]
    async fn test_login_invalid_password() {
        let mut repo = MockUserRepository::new();
        let pepper = "pepper".to_string();
        let jwt_secret = "secret".to_string();

        let password_hash = hash_password("correct", &pepper);
        let user = User {
            email: "test@example.com".to_string(),
            password_hash,
            ..Default::default()
        };

        repo.expect_find_by_email()
            .returning(move |_| Ok(Some(user.clone())));

        let service = AuthService::new(Arc::new(repo), pepper, jwt_secret);
        let result = service.login("test@example.com", "wrong").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_login_user_not_found() {
        let mut repo = MockUserRepository::new();
        let pepper = "pepper".to_string();
        let jwt_secret = "secret".to_string();

        repo.expect_find_by_email().returning(|_| Ok(None));

        let service = AuthService::new(Arc::new(repo), pepper, jwt_secret);
        let result = service.login("unknown@example.com", "password").await;

        assert!(result.is_err());
    }
}
