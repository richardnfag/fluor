use crate::domain::entities::{DomainError, Function, Trigger, User};
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<User, DomainError>;
    async fn update(&self, user: &User) -> Result<User, DomainError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait FunctionRepository: Send + Sync {
    async fn find_all(&self) -> Result<Vec<Function>, DomainError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Function>, DomainError>;
    async fn save(&self, function: &Function) -> Result<Function, DomainError>;
    async fn update(&self, function: &Function) -> Result<Function, DomainError>;
    async fn delete(&self, name: &str) -> Result<(), DomainError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TriggerRepository: Send + Sync {
    async fn find_all(&self) -> Result<Vec<Trigger>, DomainError>;
    async fn save(&self, trigger: &Trigger) -> Result<Trigger, DomainError>;
    async fn delete(&self, name: &str) -> Result<(), DomainError>;
}
