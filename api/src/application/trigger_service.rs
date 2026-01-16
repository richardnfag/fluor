use crate::application::invocation_service::InvocationService;
use crate::domain::entities::{DomainError, Trigger};
use crate::domain::ports::TriggerRepository;
use std::sync::Arc;
use tracing::warn;

pub struct TriggerService {
    repository: Arc<dyn TriggerRepository>,
    invocation_service: Arc<InvocationService>,
}

impl TriggerService {
    pub fn new(
        repository: Arc<dyn TriggerRepository>,
        invocation_service: Arc<InvocationService>,
    ) -> Self {
        Self {
            repository,
            invocation_service,
        }
    }

    pub async fn create_trigger(&self, trigger: Trigger) -> Result<Trigger, DomainError> {
        let created = self.repository.save(&trigger).await?;
        if let Err(e) = self.invocation_service.load_routes().await {
            warn!("Failed to refresh routes: {}", e);
        }
        Ok(created)
    }

    pub async fn list_triggers(&self) -> Result<Vec<Trigger>, DomainError> {
        self.repository.find_all().await
    }

    pub async fn delete_trigger(&self, name: &str) -> Result<(), DomainError> {
        self.repository.delete(name).await?;
        if let Err(e) = self.invocation_service.load_routes().await {
            warn!("Failed to refresh routes: {}", e);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::{MockFunctionRepository, MockTriggerRepository};
    use crate::domain::wasm_runtime::MockWasmRuntime;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_create_trigger() {
        let mut trigger_repo = MockTriggerRepository::new();
        let func_repo = MockFunctionRepository::new();
        let runtime = MockWasmRuntime::new();

        let trigger = Trigger {
            name: "test-trigger".to_string(),
            function_name: "test-func".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            readonly: false,
        };

        trigger_repo
            .expect_save()
            .with(always())
            .returning(|t| Ok(t.clone()));

        trigger_repo.expect_find_all().returning(|| Ok(vec![]));

        // Use Arc to share the repo
        let trigger_repo_arc = Arc::new(trigger_repo);

        let invocation_service = Arc::new(InvocationService::new(
            trigger_repo_arc.clone(),
            Arc::new(func_repo),
            Arc::new(runtime),
        ));

        let service = TriggerService::new(trigger_repo_arc, invocation_service);

        let result = service.create_trigger(trigger).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.name, "test-trigger");
    }
    #[tokio::test]
    async fn test_list_triggers() {
        let mut trigger_repo = MockTriggerRepository::new();
        let func_repo = MockFunctionRepository::new();
        let runtime = MockWasmRuntime::new();

        trigger_repo.expect_find_all().returning(|| {
            Ok(vec![Trigger {
                name: "t1".to_string(),
                function_name: "f1".to_string(),
                method: "GET".to_string(),
                path: "/1".to_string(),
                readonly: false,
            }])
        });

        let trigger_repo_arc = Arc::new(trigger_repo);

        let invocation_service = Arc::new(InvocationService::new(
            trigger_repo_arc.clone(),
            Arc::new(func_repo),
            Arc::new(runtime),
        ));

        let service = TriggerService::new(trigger_repo_arc, invocation_service);
        let result = service.list_triggers().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delete_trigger() {
        let mut trigger_repo = MockTriggerRepository::new();
        let func_repo = MockFunctionRepository::new();
        let runtime = MockWasmRuntime::new();

        // Delete expectation
        trigger_repo
            .expect_delete()
            .with(eq("t1"))
            .returning(|_| Ok(()));

        // Refresh routes expectation
        trigger_repo.expect_find_all().returning(|| Ok(vec![]));

        let trigger_repo_arc = Arc::new(trigger_repo);

        let service = TriggerService::new(
            trigger_repo_arc.clone(),
            Arc::new(InvocationService::new(
                trigger_repo_arc,
                Arc::new(func_repo),
                Arc::new(runtime),
            )),
        );

        let result = service.delete_trigger("t1").await;
        assert!(result.is_ok());
    }
}
